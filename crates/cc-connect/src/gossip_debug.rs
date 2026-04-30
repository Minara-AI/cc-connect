//! Opt-in gossip-event tracing for diagnosing mesh-forwarding bugs.
//!
//! Enabled when `CC_CONNECT_GOSSIP_DEBUG` is set to a non-empty value.
//! Every gossip event observed by chat_session's listener task and (when
//! enabled) the host-bg daemon is appended one-line-per-event to
//! `~/.cc-connect/gossip-debug.log`. Each line is plain text:
//!
//! ```text
//! <iso-ts> [<role-label>] <event-summary>
//! ```
//!
//! `role-label` is e.g. `tui Y=if3a3bv6` or `host-bg X=hnvcppgo` so a
//! correlated multi-peer reproduction can be reconstructed by sorting
//! the lines from each machine by timestamp. The file is created at
//! mode 0600 — gossip events include peer NodeIds, no message bodies.

use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Cheap env check; safe to call from hot paths.
pub fn enabled() -> bool {
    std::env::var_os("CC_CONNECT_GOSSIP_DEBUG")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

/// Append one log line. Best-effort: any I/O error is swallowed because
/// gossip-debug must never affect production behaviour.
pub fn log(role_label: &str, summary: &str) {
    if !enabled() {
        return;
    }
    let path = match log_path() {
        Some(p) => p,
        None => return,
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&path)
    {
        Ok(f) => f,
        Err(_) => return,
    };
    let line = format!("{} [{}] {}\n", iso_now(), role_label, summary);
    let _ = f.write_all(line.as_bytes());
}

fn log_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|h| h.join(".cc-connect").join("gossip-debug.log"))
}

fn iso_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_millis())
        .unwrap_or(0);
    format!("ts={secs}.{millis:03}")
}
