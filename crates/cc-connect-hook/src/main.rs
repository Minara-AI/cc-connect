//! cc-connect-hook — Claude Code `UserPromptSubmit` hook.
//!
//! Contract: PROTOCOL.md §7. Always exits 0 (any non-zero blocks the user
//! prompt). Errors fall into `~/.cc-connect/hook.log`.

use std::io::Write;
use std::path::{Path, PathBuf};

fn main() -> ! {
    if let Err(e) = run() {
        let log = home_dir().join(".cc-connect/hook.log");
        let _ = append_log(&log, &format!("hook error: {e:#}\n"));
    }
    // PROTOCOL.md §7.4: always exit 0 from main().
    std::process::exit(0)
}

fn run() -> anyhow::Result<()> {
    // Skeleton placeholder. Implementation tracks PROTOCOL.md §7.3 steps 1-9:
    //   1. Resolve $TMPDIR + $UID, build active-rooms dir.
    //   2. Enumerate *.active, validate PID via kill(pid, 0).
    //   3. Per active Room: open cursor file, fcntl(F_SETLK) exclusive,
    //      stat-vs-fstat retry per the flock-vs-rename race rule.
    //   4. Read log.jsonl with fcntl(F_RDLCK), collect Messages with id > cursor.
    //   5. Format `[chatroom @nick HH:MMZ] body\n` (single Room) or include
    //      `<room-tag>` (multi-Room).
    //   6. Iterative truncate to 8 KiB cap with marker line.
    //   7. Emit chronological-ascending across active Rooms.
    //   8. Atomic cursor advance via .tmp + rename + parent fsync.
    //   9. Release locks, exit 0.
    todo!("PROTOCOL.md §7.3 — implementation pending Week 4-5 of v0.1 plan")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn append_log(path: &Path, msg: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(msg.as_bytes())
}
