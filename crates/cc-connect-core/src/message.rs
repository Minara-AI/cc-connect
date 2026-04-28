//! Message — the chat substrate's atomic unit.
//!
//! See `PROTOCOL.md` §4 (Message schema), §10 (reserved kinds), and §11.2
//! (canonical encoding conformance vector).

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

/// Wire version. v0.1 messages MUST carry `v: 1`.
pub const PROTOCOL_VERSION: u32 = 1;

/// PROTOCOL.md §4: `body` MUST be ≤ 8 KiB after UTF-8 encoding.
pub const BODY_MAX_BYTES: usize = 8 * 1024;

/// 26-character Crockford base32 ULID.
pub const ULID_LEN: usize = 26;

/// The default (and v0.1's only) `kind`.
pub const KIND_CHAT: &str = "chat";

/// v0.2-alpha: file_drop carries an inline base64 attachment.
pub const KIND_FILE_DROP: &str = "file_drop";

/// Inline-attachment ceiling (binary bytes, before base64 expansion).
/// Sized to comfortably fit inside iroh-gossip's per-frame envelope after
/// base64 + JSON overhead (~32 KB binary → ~43 KB base64 → ~44 KB Message).
/// v0.2.1 will swap to iroh-blobs for arbitrary sizes.
pub const ATTACHMENT_MAX_BINARY_BYTES: usize = 32 * 1024;

/// A v0.1 chat Message.
///
/// Field order in the struct matches PROTOCOL.md §4's canonical JSON order
/// (`v, id, author, ts, body, kind`); serde preserves this on emit. `kind`
/// is omitted from the wire form when it equals the default `"chat"` to
/// match the §11.2 vector exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub v: u32,
    pub id: String,
    pub author: String,
    pub ts: i64,
    /// For `kind=chat`: the message body (≤ BODY_MAX_BYTES).
    /// For `kind=file_drop`: the original filename (no path components).
    pub body: String,
    #[serde(default = "default_kind", skip_serializing_if = "is_default_kind")]
    pub kind: String,
    /// v0.2-alpha file_drop attachment: base64-encoded raw file bytes.
    /// Present on the wire and in log.jsonl when kind=file_drop. Capped at
    /// ATTACHMENT_MAX_BINARY_BYTES of *binary* (4/3 expansion in base64).
    /// `None` for chat messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_b64: Option<String>,
}

fn default_kind() -> String {
    KIND_CHAT.to_string()
}
fn is_default_kind(k: &String) -> bool {
    k == KIND_CHAT
}

impl Message {
    /// Construct a chat Message; caller supplies the ULID and timestamp.
    /// Validates body size and ULID well-formedness; normalises the ULID
    /// per Crockford rules (PROTOCOL.md §4).
    pub fn new(id: &str, author: String, ts: i64, body: String) -> Result<Self> {
        validate_body(&body)?;
        let id = normalize_ulid(id)?;
        Ok(Self {
            v: PROTOCOL_VERSION,
            id,
            author,
            ts,
            body,
            kind: KIND_CHAT.to_string(),
            attachment_b64: None,
        })
    }

    /// Construct a file_drop Message (v0.2-alpha).
    ///
    /// `body` is the original filename (no path separators); `bytes` is the
    /// raw file content (capped at ATTACHMENT_MAX_BINARY_BYTES). The bytes
    /// are base64-encoded into `attachment_b64` for inline transport.
    pub fn new_file_drop(
        id: &str,
        author: String,
        ts: i64,
        filename: String,
        bytes: &[u8],
    ) -> Result<Self> {
        if bytes.len() > ATTACHMENT_MAX_BINARY_BYTES {
            bail!(
                "ATTACHMENT_TOO_LARGE: {} bytes exceeds the {} byte cap (v0.2-alpha inline limit; v0.2.1 will swap to iroh-blobs)",
                bytes.len(),
                ATTACHMENT_MAX_BINARY_BYTES
            );
        }
        if filename.is_empty() {
            bail!("FILENAME_EMPTY: file_drop body must be a non-empty filename");
        }
        if filename.contains('/') || filename.contains('\\') || filename.contains('\0') {
            bail!("FILENAME_INVALID: file_drop body MUST NOT contain path separators or NUL");
        }
        let id = normalize_ulid(id)?;
        let attachment_b64 = data_encoding::BASE64.encode(bytes);
        Ok(Self {
            v: PROTOCOL_VERSION,
            id,
            author,
            ts,
            body: filename,
            kind: KIND_FILE_DROP.to_string(),
            attachment_b64: Some(attachment_b64),
        })
    }

    /// Decode the inline attachment to raw bytes. Returns `Ok(None)` if the
    /// Message has no attachment (chat kind, or file_drop with bytes already
    /// stripped post-extraction).
    pub fn decode_attachment(&self) -> Result<Option<Vec<u8>>> {
        match &self.attachment_b64 {
            None => Ok(None),
            Some(s) => data_encoding::BASE64
                .decode(s.as_bytes())
                .map(Some)
                .map_err(|e| anyhow!("ATTACHMENT_BASE64_ERROR: {e}")),
        }
    }

    /// Serialise to PROTOCOL.md §4 canonical bytes.
    pub fn to_canonical_json(&self) -> Result<Vec<u8>> {
        // serde_json's compact output (default) matches §4:
        //   - no insignificant whitespace
        //   - no `\/` escape, no HTML-escape (`<`, `>`, `&` raw)
        //   - named C0 escapes for \b \t \n \f \r, `\u00xx` for other 0x00–0x1F
        //   - DEL (0x7F) and UTF-8 multi-byte raw
        //   - integer numeric `ts` (we use i64; serde_json emits without exponent)
        Ok(serde_json::to_vec(self)?)
    }

    /// Parse a Message from wire bytes (gossip event payload or a JSONL line).
    ///
    /// Performs PROTOCOL.md §4 receiver checks in this order:
    ///   1. JSON parse → reject malformed.
    ///   2. `v` precedence (PROTOCOL §0): reject any `v != 1` *before* the
    ///      unknown-field-tolerance rule applies.
    ///   3. `kind` reservation (§10): v0.1 drops Messages with non-chat kind.
    ///   4. Body cap.
    ///   5. ULID normalise.
    pub fn from_wire_bytes(bytes: &[u8]) -> Result<Self> {
        let mut msg: Message = serde_json::from_slice(bytes)
            .map_err(|e| anyhow!("PARSE_ERROR: {e}"))?;

        if msg.v != PROTOCOL_VERSION {
            bail!(
                "VERSION_MISMATCH: receivers MUST drop messages with v != {PROTOCOL_VERSION} (got v={})",
                msg.v
            );
        }
        match msg.kind.as_str() {
            KIND_CHAT => validate_body(&msg.body)?,
            KIND_FILE_DROP => {
                // For file_drop, body is a filename — small. Attachment may
                // be present (on wire) or absent (after local extraction).
                if msg.body.is_empty() {
                    bail!("FILENAME_EMPTY: file_drop body must be non-empty");
                }
                if msg.body.contains('/') || msg.body.contains('\\') || msg.body.contains('\0') {
                    bail!("FILENAME_INVALID: file_drop body has path separators");
                }
                if let Some(b64) = &msg.attachment_b64 {
                    // Decoded length cap (RFC 4648 base64 expands by ~4/3;
                    // budget = ATTACHMENT_MAX_BINARY_BYTES * 4 / 3 + a bit).
                    if b64.len() > ATTACHMENT_MAX_BINARY_BYTES * 4 / 3 + 4 {
                        bail!(
                            "ATTACHMENT_TOO_LARGE: base64 length {} suggests > {} binary bytes",
                            b64.len(),
                            ATTACHMENT_MAX_BINARY_BYTES
                        );
                    }
                }
            }
            other => {
                bail!(
                    "UNKNOWN_KIND: receivers MUST drop messages with unrecognised kind (got kind={other:?})"
                )
            }
        }
        msg.id = normalize_ulid(&msg.id)?;
        Ok(msg)
    }
}

fn validate_body(body: &str) -> Result<()> {
    if body.len() > BODY_MAX_BYTES {
        bail!(
            "BODY_TOO_LARGE: {} bytes exceeds the {} byte cap (PROTOCOL.md §4)",
            body.len(),
            BODY_MAX_BYTES
        );
    }
    Ok(())
}

/// Normalise a ULID per PROTOCOL.md §4's Crockford rules: case-fold to
/// uppercase; map I/i/L/l → 1, O/o → 0; reject U/u; reject everything else
/// not in the Crockford alphabet.
pub fn normalize_ulid(s: &str) -> Result<String> {
    let mut out = String::with_capacity(ULID_LEN);
    for c in s.chars() {
        let mapped = match c {
            'I' | 'i' | 'L' | 'l' => '1',
            'O' | 'o' => '0',
            'U' | 'u' => bail!("ULID_INVALID_CHAR: U/u reserved by Crockford normalisation"),
            '0'..='9' | 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' | 'H' | 'J' | 'K'
            | 'M' | 'N' | 'P' | 'Q' | 'R' | 'S' | 'T' | 'V' | 'W' | 'X' | 'Y' | 'Z' => c,
            'a' | 'b' | 'c' | 'd' | 'e' | 'f' | 'g' | 'h' | 'j' | 'k' | 'm' | 'n'
            | 'p' | 'q' | 'r' | 's' | 't' | 'v' | 'w' | 'x' | 'y' | 'z' => c.to_ascii_uppercase(),
            other => bail!("ULID_INVALID_CHAR: {other:?} not in Crockford alphabet"),
        };
        out.push(mapped);
    }
    if out.len() != ULID_LEN {
        bail!(
            "ULID_LENGTH: expected {ULID_LEN} chars after normalisation, got {}",
            out.len()
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VEC_PUBKEY: &str = "hnvcppgow2sc2yvdvdicu3ynonsteflxdxrehjr2ybekdc2z3iuq";
    const VEC_ULID: &str = "01HZA8K9F0RS3JXG7QZ4N5VTBC";
    const VEC_TS: i64 = 1714323456789;
    const VEC_BODY: &str = "use postgres";

    /// PROTOCOL.md §11.2 main canonical encoding vector.
    #[test]
    fn protocol_11_2_canonical_encoding_byte_exact() {
        let msg = Message::new(VEC_ULID, VEC_PUBKEY.to_string(), VEC_TS, VEC_BODY.to_string())
            .expect("valid §11.2 inputs");
        let bytes = msg.to_canonical_json().expect("canonical encode");
        let s = std::str::from_utf8(&bytes).expect("valid UTF-8");

        let expected = format!(
            r#"{{"v":1,"id":"{VEC_ULID}","author":"{VEC_PUBKEY}","ts":{VEC_TS},"body":"{VEC_BODY}"}}"#
        );
        assert_eq!(s, expected, "§11.2 canonical encoding MUST match byte-for-byte");

        // Length probe — emitted to PROTOCOL.md §11.2; spec author should
        // verify the published `Length: N bytes.` line matches this number.
        eprintln!("§11.2 canonical length = {} bytes", bytes.len());
    }

    /// PROTOCOL.md §11.2 edge-case body vector.
    /// Body raw bytes: `<` + `é` (UTF-8 0xc3 0xa9) + `>` + LF + `"` + `x`.
    /// Encoded body field including surrounding quotes is what this test
    /// pins; the spec said 12 bytes which was off by one (real value below).
    #[test]
    fn protocol_11_2_edge_body_encoding() {
        let raw = "<é>\n\"x";
        let msg = Message::new(VEC_ULID, VEC_PUBKEY.to_string(), VEC_TS, raw.to_string())
            .expect("valid edge body");
        let bytes = msg.to_canonical_json().expect("encode");
        let s = std::str::from_utf8(&bytes).expect("utf-8");

        // The body field, including the surrounding quotes, MUST appear as:
        //   "<é>\n\"x"
        // where:
        //   `<`, `>`, `é` (raw UTF-8) — passed through unescaped
        //   LF (0x0a)                  — escaped as `\n` (2 ASCII bytes)
        //   `"`                        — escaped as `\"` (2 ASCII bytes)
        let expected_body_field = r#""body":"<é>\n\"x""#;
        assert!(
            s.contains(expected_body_field),
            "expected body field {expected_body_field:?} in: {s}"
        );

        // Probe — print the actual encoded length so the spec author can
        // pin the right number into PROTOCOL.md §11.2.
        eprintln!("edge body field length (incl surrounding quotes) = {}", expected_body_field.len());
    }

    #[test]
    fn roundtrip_canonical() {
        let original = Message::new(VEC_ULID, VEC_PUBKEY.to_string(), VEC_TS, VEC_BODY.to_string()).unwrap();
        let bytes = original.to_canonical_json().unwrap();
        let parsed = Message::from_wire_bytes(&bytes).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn rejects_wrong_wire_version() {
        let bad = br#"{"v":2,"id":"01HZA8K9F0RS3JXG7QZ4N5VTBC","author":"x","ts":1,"body":""}"#;
        let err = Message::from_wire_bytes(bad).err().expect("v=2 must be rejected");
        assert!(err.to_string().contains("VERSION_MISMATCH"), "got: {err}");
    }

    #[test]
    fn rejects_unknown_kind() {
        // "system" is reserved per PROTOCOL.md §10 but not yet supported by
        // v0.2-alpha (only chat + file_drop are accepted).
        let bad = br#"{"v":1,"id":"01HZA8K9F0RS3JXG7QZ4N5VTBC","author":"x","ts":1,"body":"","kind":"system"}"#;
        let err = Message::from_wire_bytes(bad).err().expect("system kind must be rejected");
        assert!(err.to_string().contains("UNKNOWN_KIND"), "got: {err}");
    }

    #[test]
    fn file_drop_kind_accepted_with_filename_and_attachment() {
        // Build a file_drop, encode, parse back, attachment decodes to bytes.
        let bytes = b"<svg>hello</svg>".to_vec();
        let msg = Message::new_file_drop(
            VEC_ULID,
            VEC_PUBKEY.to_string(),
            VEC_TS,
            "design.svg".to_string(),
            &bytes,
        )
        .expect("valid file_drop");
        assert_eq!(msg.kind, KIND_FILE_DROP);
        assert_eq!(msg.body, "design.svg");
        let wire = msg.to_canonical_json().unwrap();
        let parsed = Message::from_wire_bytes(&wire).expect("file_drop wire roundtrip");
        assert_eq!(parsed.kind, KIND_FILE_DROP);
        assert_eq!(parsed.body, "design.svg");
        assert_eq!(parsed.decode_attachment().unwrap().unwrap(), bytes);
    }

    #[test]
    fn file_drop_with_path_separator_in_filename_rejected() {
        let err = Message::new_file_drop(
            VEC_ULID,
            VEC_PUBKEY.to_string(),
            VEC_TS,
            "../etc/passwd".to_string(),
            b"x",
        )
        .err()
        .expect("path separator MUST be rejected");
        assert!(err.to_string().contains("FILENAME_INVALID"), "got: {err}");
    }

    #[test]
    fn file_drop_with_oversized_attachment_rejected_at_constructor() {
        let big = vec![0u8; ATTACHMENT_MAX_BINARY_BYTES + 1];
        let err = Message::new_file_drop(
            VEC_ULID,
            VEC_PUBKEY.to_string(),
            VEC_TS,
            "huge.bin".to_string(),
            &big,
        )
        .err()
        .expect("oversize MUST be rejected");
        assert!(err.to_string().contains("ATTACHMENT_TOO_LARGE"), "got: {err}");
    }

    #[test]
    fn file_drop_with_empty_body_rejected_on_wire() {
        let bad = br#"{"v":1,"id":"01HZA8K9F0RS3JXG7QZ4N5VTBC","author":"x","ts":1,"body":"","kind":"file_drop"}"#;
        let err = Message::from_wire_bytes(bad).err().expect("empty filename rejected");
        assert!(err.to_string().contains("FILENAME_EMPTY"), "got: {err}");
    }

    #[test]
    fn chat_message_has_no_attachment_field_in_canonical_form() {
        let msg = Message::new(VEC_ULID, VEC_PUBKEY.to_string(), VEC_TS, VEC_BODY.to_string()).unwrap();
        let s = String::from_utf8(msg.to_canonical_json().unwrap()).unwrap();
        assert!(
            !s.contains("attachment_b64"),
            "chat Messages must omit attachment_b64 (skip_serializing_if): {s}"
        );
    }

    #[test]
    fn absent_kind_treated_as_chat() {
        let m = br#"{"v":1,"id":"01HZA8K9F0RS3JXG7QZ4N5VTBC","author":"x","ts":1,"body":""}"#;
        let parsed = Message::from_wire_bytes(m).unwrap();
        assert_eq!(parsed.kind, KIND_CHAT);
    }

    #[test]
    fn explicit_chat_kind_parses() {
        let m = br#"{"v":1,"id":"01HZA8K9F0RS3JXG7QZ4N5VTBC","author":"x","ts":1,"body":"","kind":"chat"}"#;
        let parsed = Message::from_wire_bytes(m).unwrap();
        assert_eq!(parsed.kind, KIND_CHAT);
    }

    #[test]
    fn rejects_oversized_body() {
        let body = "x".repeat(BODY_MAX_BYTES + 1);
        let r = Message::new(VEC_ULID, VEC_PUBKEY.to_string(), VEC_TS, body);
        let err = r.err().expect("oversized body must be rejected");
        assert!(err.to_string().contains("BODY_TOO_LARGE"), "got: {err}");
    }

    #[test]
    fn accepts_body_at_exact_cap() {
        let body = "x".repeat(BODY_MAX_BYTES);
        Message::new(VEC_ULID, VEC_PUBKEY.to_string(), VEC_TS, body).expect("8 KiB body OK");
    }

    #[test]
    fn ignores_unknown_top_level_fields() {
        // PROTOCOL §4: receivers MUST ignore unknown top-level fields after
        // the `v` and `kind` checks pass.
        let m = br#"{"v":1,"id":"01HZA8K9F0RS3JXG7QZ4N5VTBC","author":"x","ts":1,"body":"","future_field":42}"#;
        Message::from_wire_bytes(m).expect("unknown top-level fields tolerated");
    }

    #[test]
    fn ulid_normalizes_crockford_aliases() {
        // I, L, O are aliases of 1, 1, 0.
        let raw = "01HZA8K9F0RS3JXG7QZ4N5VTBI"; // 26 chars; trailing I → 1
        let n = normalize_ulid(raw).unwrap();
        assert_eq!(n, "01HZA8K9F0RS3JXG7QZ4N5VTB1");

        let raw = "01HZA8K9F0RS3JXG7QZ4N5VTBO"; // trailing O → 0
        let n = normalize_ulid(raw).unwrap();
        assert_eq!(n, "01HZA8K9F0RS3JXG7QZ4N5VTB0");

        let raw = "01HZA8K9F0RS3JXG7QZ4N5VTBl"; // lowercase l → 1
        let n = normalize_ulid(raw).unwrap();
        assert_eq!(n, "01HZA8K9F0RS3JXG7QZ4N5VTB1");
    }

    #[test]
    fn ulid_rejects_u_and_u_lower() {
        // Crockford reserves U/u; PROTOCOL.md §4 says reject.
        let bad = "01HZA8K9F0RS3JXG7QZ4N5VTBU";
        let err = normalize_ulid(bad).err().expect("U must be rejected");
        assert!(err.to_string().contains("U"), "got: {err}");
    }

    #[test]
    fn ulid_rejects_wrong_length() {
        let too_short = "01HZA8K9F";
        let err = normalize_ulid(too_short).err().expect("short ULID rejected");
        assert!(err.to_string().contains("ULID_LENGTH"), "got: {err}");
    }

    #[test]
    fn ulid_normalizes_lowercase_to_uppercase() {
        // Per §4 canonical output MUST be uppercase.
        let lower = "01hza8k9f0rs3jxg7qz4n5vtbc";
        let n = normalize_ulid(lower).unwrap();
        assert_eq!(n, "01HZA8K9F0RS3JXG7QZ4N5VTBC");
    }

    /// Parse-fails reject malformed JSON gracefully (no panic, error mentions PARSE).
    #[test]
    fn rejects_malformed_json() {
        let err = Message::from_wire_bytes(b"not json").err().unwrap();
        assert!(err.to_string().contains("PARSE_ERROR"), "got: {err}");
    }

    /// `ts` MUST be an integer literal; reject scientific-notation forms.
    #[test]
    fn rejects_exponent_ts() {
        let bad = br#"{"v":1,"id":"01HZA8K9F0RS3JXG7QZ4N5VTBC","author":"x","ts":1.7e12,"body":""}"#;
        // serde_json will fail to deserialise i64 from a float — that's the
        // canonical-encoding-only stance in §4. We accept the parse error.
        let err = Message::from_wire_bytes(bad).err().expect("exponent ts rejected");
        assert!(err.to_string().contains("PARSE_ERROR"), "got: {err}");
    }
}
