//! Internal POSIX helpers shared across cc-connect-core I/O modules.
//!
//! Most entries are `pub(crate)` plumbing for `log_io` / `cursor_io`. The
//! `secure_dir` helper is `pub` because chat_session (a downstream crate)
//! needs the same lstat-strict directory creation discipline that the
//! hook reader applies under PROTOCOL.md §8.
//!
//! See `PROTOCOL.md` §5 (writer locks), §7.3 step 8 (cursor lock + race),
//! §7.4 (lock unification rationale), §8 (active-rooms lstat strictness).

use anyhow::{anyhow, bail, Context, Result};
use std::fs::File;
use std::os::fd::AsFd;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Mode for the kind of advisory lock we want to take on a file.
#[derive(Copy, Clone)]
pub(crate) enum LockKind {
    /// `LOCK_SH` equivalent — multiple readers, blocks writers.
    Shared,
    /// `LOCK_EX` equivalent — exclusive single holder, blocks everyone else.
    Exclusive,
}

/// Acquire a blocking advisory lock on `file`.
///
/// Uses `rustix::fs::fcntl_lock`. On Linux this calls `fcntl(F_OFD_SETLKW)`
/// (per-fd, the modern preferred form). On macOS the fallback may be
/// `fcntl(F_SETLKW)` (per-process), so callers within the same process
/// that share a file via separate `File` handles cannot rely on this for
/// serialisation — see `log_io::append`'s single-syscall write strategy
/// for the actual cross-thread atomicity guarantee. Cross-*process*
/// serialisation works on both kernels.
pub(crate) fn acquire_lock(file: &File, kind: LockKind) -> Result<()> {
    use rustix::fs::{fcntl_lock, FlockOperation};
    let op = match kind {
        LockKind::Shared => FlockOperation::LockShared,
        LockKind::Exclusive => FlockOperation::LockExclusive,
    };
    fcntl_lock(file.as_fd(), op).map_err(|e| anyhow!("fcntl lock acquire: {e}"))
}

/// Release any lock held on `file`. Idempotent: unlocking an already-unlocked
/// file is a no-op.
pub(crate) fn release_lock(file: &File) -> Result<()> {
    use rustix::fs::{fcntl_lock, FlockOperation};
    fcntl_lock(file.as_fd(), FlockOperation::Unlock).map_err(|e| anyhow!("fcntl lock release: {e}"))
}

/// Ensure `path` is a non-symlink directory at mode `0700`, creating it
/// atomically with that mode if missing.
///
/// PROTOCOL.md §8 ("active-rooms") mandates that every implementation
/// "MUST `lstat` the parent directory and refuse if (a) it is a symlink,
/// (b) it is not a directory, or (c) its mode is not exactly `0700`."
/// The reader path in `cc-connect-hook` already enforces this; the writer
/// path used by chat_session must apply the same check or a hostile
/// co-tenant can pre-create the path as a symlink to their own watched
/// directory and trick `chmod 0700` into hardening the wrong target.
///
/// Behaviour:
///   - missing → `mkdir(path, 0o700)` via rustix (no umask race, atomic).
///   - present and a symlink → bail (`REFUSE_SYMLINK`).
///   - present but not a directory → bail (`REFUSE_NOT_DIR`).
///   - present, mode != 0700 → tighten via `chmod` (we own the path).
pub fn ensure_secure_dir(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                bail!(
                    "REFUSE_SYMLINK: {} is a symlink — possible co-tenant attack",
                    path.display()
                );
            }
            if !meta.is_dir() {
                bail!(
                    "REFUSE_NOT_DIR: {} exists but is not a directory",
                    path.display()
                );
            }
            let mode = meta.permissions().mode() & 0o777;
            if mode != 0o700 {
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
                    .with_context(|| format!("chmod 0700 {}", path.display()))?;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Atomic create-with-mode. Avoids the
            // `mkdir`-then-`chmod`-via-umask race that lets a hostile
            // process win between the two calls.
            use rustix::fs::{mkdir, Mode};
            mkdir(path, Mode::from_bits_truncate(0o700))
                .map_err(|e| anyhow!("mkdir {} (mode 0700): {e}", path.display()))?;
        }
        Err(e) => return Err(anyhow!("lstat {}: {e}", path.display())),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_secure_dir_creates_with_0700() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("new-dir");
        ensure_secure_dir(&p).unwrap();
        let meta = std::fs::metadata(&p).unwrap();
        assert!(meta.is_dir());
        assert_eq!(meta.permissions().mode() & 0o777, 0o700);
    }

    #[test]
    fn ensure_secure_dir_tightens_loose_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("loose");
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        ensure_secure_dir(&p).unwrap();
        assert_eq!(
            std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn ensure_secure_dir_refuses_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        std::fs::create_dir(&target).unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let err = ensure_secure_dir(&link).unwrap_err();
        assert!(
            err.to_string().contains("REFUSE_SYMLINK"),
            "expected refusal, got: {err}"
        );
    }

    #[test]
    fn ensure_secure_dir_refuses_regular_file() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("notadir");
        std::fs::write(&p, b"hi").unwrap();
        let err = ensure_secure_dir(&p).unwrap_err();
        assert!(
            err.to_string().contains("REFUSE_NOT_DIR"),
            "expected refusal, got: {err}"
        );
    }
}
