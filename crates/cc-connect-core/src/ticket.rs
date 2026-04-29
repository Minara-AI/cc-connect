//! Room ticket encoding — `cc1-` prefix + base32 + CRC32 over opaque
//! iroh-gossip ticket bytes.
//!
//! See `PROTOCOL.md` §3 (Room ticket) and §11.2 (conformance vector).
//!
//! This module is intentionally agnostic to iroh-gossip's internal byte
//! layout: it takes a `&[u8]` (whatever `iroh_gossip::Ticket::serialize()`
//! produces) and produces / consumes the user-facing `cc1-…` Room code.

use anyhow::{anyhow, Result};

/// Mandatory ASCII prefix that identifies a cc-connect Room code.
pub const ROOM_CODE_PREFIX: &str = "cc1-";

/// Wrap opaque ticket bytes as a Room code.
///
/// Output: `cc1-` + RFC 4648 base32 (lowercase, no padding) of
/// `ticket_bytes || CRC32_ISO_HDLC_be(ticket_bytes)`.
pub fn encode_room_code(ticket_bytes: &[u8]) -> String {
    let crc = crc32_iso_hdlc(ticket_bytes);
    let mut payload = Vec::with_capacity(ticket_bytes.len() + 4);
    payload.extend_from_slice(ticket_bytes);
    payload.extend_from_slice(&crc.to_be_bytes());

    let b32 = data_encoding::BASE32_NOPAD.encode(&payload).to_lowercase();
    let mut out = String::with_capacity(ROOM_CODE_PREFIX.len() + b32.len());
    out.push_str(ROOM_CODE_PREFIX);
    out.push_str(&b32);
    out
}

/// Decode a Room code back to its opaque ticket bytes.
///
/// Errors map to the codes named in `PROTOCOL.md` §3:
/// - `INVALID_PREFIX` — room code does not start with `cc1-` (case-sensitive).
/// - `BASE32_ERROR`   — payload is not valid RFC 4648 base32.
/// - `TRUNCATED`      — decoded payload is shorter than the 4-byte CRC.
/// - `CHECKSUM_MISMATCH` — the trailing CRC does not match a recompute.
pub fn decode_room_code(room_code: &str) -> Result<Vec<u8>> {
    // Step 1: strip the `cc1-` prefix. Case-sensitive lowercase per §3.
    let b32 = room_code
        .strip_prefix(ROOM_CODE_PREFIX)
        .ok_or_else(|| anyhow!("INVALID_PREFIX: room code must start with `{ROOM_CODE_PREFIX}`"))?;

    // Step 2: base32 decode. Case-insensitive on input per §3 step 2;
    // BASE32_NOPAD only accepts uppercase, so normalise first.
    let upper = b32.to_ascii_uppercase();
    let payload = data_encoding::BASE32_NOPAD
        .decode(upper.as_bytes())
        .map_err(|e| anyhow!("BASE32_ERROR: {e}"))?;

    // Step 3: split CRC. Need at least 4 bytes for the CRC.
    if payload.len() < 4 {
        return Err(anyhow!(
            "TRUNCATED: payload {} bytes, need at least 4 for CRC32",
            payload.len()
        ));
    }
    let crc_start = payload.len() - 4;
    let (ticket_bytes, crc_bytes) = payload.split_at(crc_start);
    let claimed_crc = u32::from_be_bytes([crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]]);

    // Step 4: verify CRC.
    let computed = crc32_iso_hdlc(ticket_bytes);
    if claimed_crc != computed {
        return Err(anyhow!(
            "CHECKSUM_MISMATCH: claimed {claimed_crc:#010x} != computed {computed:#010x}"
        ));
    }

    Ok(ticket_bytes.to_vec())
}

/// CRC-32 / ISO-HDLC (the zlib / RFC 1952 / gzip CRC).
///
/// - polynomial: `0xEDB88320` (reversed form of `0x04C11DB7`)
/// - init:       `0xFFFFFFFF`
/// - reflect input + reflect output
/// - xorout:     `0xFFFFFFFF`
///
/// Spec'd in `PROTOCOL.md` §3.
fn crc32_iso_hdlc(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            // Branch-free: if low bit set, XOR with reversed polynomial.
            let mask = (crc & 1).wrapping_neg(); // 0xFFFFFFFF if low bit set, else 0.
            crc = (crc >> 1) ^ (0xEDB88320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Well-known CRC-32/ISO-HDLC test vectors.
    #[test]
    fn crc32_known_vectors() {
        assert_eq!(crc32_iso_hdlc(b""), 0x00000000);
        assert_eq!(crc32_iso_hdlc(b"a"), 0xE8B7BE43);
        // The canonical CRC-32 test string ("123456789") → 0xCBF43926.
        assert_eq!(crc32_iso_hdlc(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn roundtrip_arbitrary_bytes() {
        let inputs: &[&[u8]] = &[
            &[],
            &[0x00],
            &[0xff, 0xfe, 0xfd],
            &[0x42; 32],
            &[0u8; 64],
            b"this is some opaque ticket payload of moderate length 0123456789",
        ];
        for input in inputs {
            let code = encode_room_code(input);
            assert!(
                code.starts_with(ROOM_CODE_PREFIX),
                "missing prefix in {code}"
            );
            let decoded = decode_room_code(&code)
                .unwrap_or_else(|e| panic!("decode failed for {input:?}: {e}"));
            assert_eq!(decoded, *input);
        }
    }

    /// PROTOCOL.md §11.2 conformance vector: 32 zero bytes (placeholder for
    /// an iroh ticket with topic_id of zeros and no bootstrap addrs).
    #[test]
    fn protocol_11_2_zero_topic_id_decodes_cleanly() {
        let zero_ticket = vec![0u8; 32];
        let code = encode_room_code(&zero_ticket);
        let decoded = decode_room_code(&code).expect("§11.2 vector MUST decode");
        assert_eq!(decoded, zero_ticket);

        // The published vector includes the raw cc1-… string. It is whatever
        // this encoder produces; the §11.2 contract is that *every* compliant
        // implementation produces the *same* string. Print it on test run so
        // a reference impl can copy it into PROTOCOL.md §11.2.
        eprintln!("§11.2 zero-topic Room code = {code}");
    }

    #[test]
    fn rejects_missing_prefix() {
        let err = decode_room_code("aaaaaaaa").unwrap_err();
        assert!(err.to_string().contains("INVALID_PREFIX"), "got: {err}");
    }

    #[test]
    fn rejects_wrong_prefix_case() {
        // Per §3 step 1: prefix is case-sensitive lowercase.
        let inner = encode_room_code(b"hello")
            .trim_start_matches(ROOM_CODE_PREFIX)
            .to_string();
        let mixed = format!("CC1-{inner}");
        let err = decode_room_code(&mixed).unwrap_err();
        assert!(err.to_string().contains("INVALID_PREFIX"), "got: {err}");
    }

    #[test]
    fn accepts_uppercase_base32_payload() {
        // Per §3 step 2: base32 is case-insensitive on input.
        let original = encode_room_code(b"some payload bytes");
        let inner = original.trim_start_matches(ROOM_CODE_PREFIX).to_uppercase();
        let upper = format!("{ROOM_CODE_PREFIX}{inner}");
        let decoded = decode_room_code(&upper).expect("uppercase base32 must decode");
        assert_eq!(decoded, b"some payload bytes");
    }

    #[test]
    fn rejects_invalid_base32() {
        let bad = format!("{ROOM_CODE_PREFIX}!!!not-base32!!!");
        let err = decode_room_code(&bad).unwrap_err();
        assert!(err.to_string().contains("BASE32_ERROR"), "got: {err}");
    }

    #[test]
    fn rejects_truncated_payload() {
        // 1 byte of base32 → 5 bits → no full byte after decode.
        let bad = format!("{ROOM_CODE_PREFIX}aa");
        let err = decode_room_code(&bad).unwrap_err();
        assert!(
            err.to_string().contains("TRUNCATED") || err.to_string().contains("BASE32_ERROR"),
            "got: {err}"
        );
    }

    #[test]
    fn rejects_crc_mismatch() {
        let mut code = encode_room_code(b"valid ticket");
        // Flip the last character — overwhelmingly likely to corrupt CRC bytes.
        let last = code.pop().expect("non-empty room code");
        let new_last = if last == 'a' { 'b' } else { 'a' };
        code.push(new_last);
        let err = decode_room_code(&code).unwrap_err();
        assert!(
            err.to_string().contains("CHECKSUM_MISMATCH")
                || err.to_string().contains("BASE32_ERROR"),
            "got: {err}"
        );
    }
}
