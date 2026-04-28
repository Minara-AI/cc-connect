//! Peer Identity — Ed25519 keypair persisted to disk.
//!
//! See `PROTOCOL.md` §2 (Identity) and §11.1 (conformance vector).
//!
//! - Keypair stored at `~/.cc-connect/identity.key` (caller chooses path; this
//!   module is path-agnostic so tests can use a temp dir).
//! - File contents: 32 raw bytes of the Ed25519 secret-key seed.
//! - File mode MUST be `0600` on creation. Same key is used as the iroh
//!   transport key in v0.1, so the Pubkey and the iroh `NodeId` are equal.

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Length of the Ed25519 secret-key seed in bytes.
pub const SEED_LEN: usize = 32;

/// A loaded or freshly generated cc-connect peer Identity.
pub struct Identity {
    signing_key: ed25519_dalek::SigningKey,
}

impl Identity {
    /// Load the Identity at `path`, creating one if the file is absent.
    ///
    /// On creation the parent directory must already exist; the file is
    /// created with mode `0600` per PROTOCOL.md §2.
    pub fn generate_or_load(path: &Path) -> Result<Self> {
        if path.exists() {
            Self::load(path)
        } else {
            Self::generate_to_path(path)
        }
    }

    fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
        let seed: [u8; SEED_LEN] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected exactly {SEED_LEN} bytes in {}", path.display()))?;
        Ok(Self::from_seed(seed))
    }

    fn generate_to_path(path: &Path) -> Result<Self> {
        let mut seed = [0u8; SEED_LEN];
        getrandom::getrandom(&mut seed)
            .map_err(|e| anyhow::anyhow!("OS random source failed: {e}"))?;

        let mut file = open_create_0600(path)
            .with_context(|| format!("create {}", path.display()))?;
        file.write_all(&seed)?;
        file.sync_all()?;

        Ok(Self::from_seed(seed))
    }

    /// Construct an Identity from a known 32-byte seed (test / vector use).
    pub fn from_seed(seed: [u8; SEED_LEN]) -> Self {
        Self {
            signing_key: ed25519_dalek::SigningKey::from_bytes(&seed),
        }
    }

    /// The 32-byte Ed25519 public key bytes.
    pub fn pubkey_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Canonical Pubkey string per PROTOCOL.md §2: RFC 4648 base32, lowercase,
    /// no padding, of the 32-byte public key. Matches iroh's `NodeId` string.
    pub fn pubkey_string(&self) -> String {
        data_encoding::BASE32_NOPAD
            .encode(&self.pubkey_bytes())
            .to_lowercase()
    }
}

#[cfg(unix)]
fn open_create_0600(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn open_create_0600(path: &Path) -> std::io::Result<fs::File> {
    // v0.1 supports Linux + macOS only (PROTOCOL.md §0 / design doc).
    // On non-unix targets, fall back to a normal create — callers should
    // not reach this path in v0.1.
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PROTOCOL.md §11.1 conformance vector.
    /// Seed of 32 zero bytes derives a known Ed25519 public key.
    #[test]
    fn pubkey_from_zero_seed_matches_protocol_vector_11_1() {
        let id = Identity::from_seed([0u8; SEED_LEN]);

        let pubkey_hex = data_encoding::HEXLOWER.encode(&id.pubkey_bytes());
        assert_eq!(
            pubkey_hex,
            "3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
            "PROTOCOL.md §11.1 says zero-seed pubkey hex MUST be this exact string"
        );

        let pubkey_b32 = id.pubkey_string();
        assert_eq!(
            pubkey_b32.len(),
            52,
            "32 raw bytes → 52 base32-no-pad chars"
        );
        assert_eq!(
            pubkey_b32,
            "hnvcppgow2sc2yvdvdicu3ynonsteflxdxrehjr2ybekdc2z3iuq",
            "PROTOCOL.md §11.1 says zero-seed pubkey base32 MUST be this exact string"
        );
    }

    #[cfg(unix)]
    #[test]
    fn generate_creates_key_file_with_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("identity.key");

        let _id = Identity::generate_or_load(&key_path).unwrap();

        let mode = fs::metadata(&key_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "PROTOCOL.md §2 requires file mode 0600 on creation");
    }

    #[test]
    fn generate_then_load_returns_same_pubkey() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("identity.key");

        let first = Identity::generate_or_load(&key_path).unwrap();
        let second = Identity::generate_or_load(&key_path).unwrap();

        assert_eq!(first.pubkey_string(), second.pubkey_string());
        assert_eq!(first.pubkey_bytes(), second.pubkey_bytes());
    }

    #[test]
    fn load_rejects_wrong_length_file() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("identity.key");
        fs::write(&key_path, b"not 32 bytes").unwrap();

        // Avoid `unwrap_err`: it requires `T: Debug`, but `Identity` deliberately
        // does NOT derive `Debug` to keep the SigningKey from leaking via fmt.
        let err = Identity::generate_or_load(&key_path)
            .err()
            .expect("expected error for wrong-length file");
        assert!(
            err.to_string().contains("32 bytes"),
            "error must mention expected length, got: {err}"
        );
    }
}
