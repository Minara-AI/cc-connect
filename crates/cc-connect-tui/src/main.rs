//! `cc-connect-tui` — vertical-split TUI binary.
//!
//! Two modes:
//!   - `start [--relay <url>]`  — spawn a `cc-connect host-bg` daemon,
//!     join its room, drop you into the TUI.
//!   - `join <ticket> [--relay <url>]` — join an existing room.
//!
//! The `cc-connect room` subcommand in the main `cc-connect` binary just
//! `exec`s into this binary so users can type either form.

use anyhow::{anyhow, Context, Result};
use cc_connect::ticket_payload::TicketPayload;
use cc_connect_core::ticket::decode_room_code;
use cc_connect_tui::{setup, RunOpts};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(name = "cc-connect-tui", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start a new room (spawns `cc-connect host-bg` in the background) and
    /// open the TUI.
    Start {
        #[arg(long, value_name = "URL")]
        relay: Option<String>,
        /// Override / set the saved display name. Persists to
        /// `~/.cc-connect/config.json`. Use empty string to clear.
        #[arg(long, value_name = "NAME")]
        nick: Option<String>,
        /// Trailing args forwarded to `claude`. Use `--` to separate, e.g.
        /// `cc-connect-tui start -- --model opus --resume`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        claude_args: Vec<String>,
    },
    /// Join an existing room by ticket.
    Join {
        ticket: String,
        #[arg(long, value_name = "URL")]
        relay: Option<String>,
        /// Override / set the saved display name. Persists to
        /// `~/.cc-connect/config.json`. Use empty string to clear.
        #[arg(long, value_name = "NAME")]
        nick: Option<String>,
        /// Trailing args forwarded to `claude`. Use `--` to separate.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        claude_args: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Bail early on non-TTY so we don't spin up host-bg, hold an iroh
    // endpoint, etc. before realising we have nowhere to render.
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return Err(anyhow!(
            "TTY required — `cc-connect-tui` must run in an interactive terminal"
        ));
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;

    rt.block_on(async move {
        match cli.cmd {
            Cmd::Start {
                relay,
                nick,
                claude_args,
            } => start(relay.as_deref(), nick.as_deref(), claude_args).await,
            Cmd::Join {
                ticket,
                relay,
                nick,
                claude_args,
            } => join(&ticket, relay.as_deref(), nick.as_deref(), claude_args).await,
        }
    })
}

async fn start(
    relay: Option<&str>,
    nick_override: Option<&str>,
    claude_args: Vec<String>,
) -> Result<()> {
    // First-run wizard: hook + nick + relay choice. Prompts use plain
    // stdin/stdout BEFORE the alt-screen takes over, so they look normal.
    if let Err(e) = setup::ensure_hook_installed() {
        eprintln!("(setup: hook check failed: {e:#})");
    }
    if let Err(e) = setup::ensure_mcp_installed() {
        eprintln!("(setup: mcp install failed: {e:#})");
    }
    if let Err(e) = setup::ensure_self_nick(nick_override) {
        eprintln!("(setup: nick prompt failed: {e:#})");
    }
    let resolved_relay = setup::ensure_relay_choice(relay).unwrap_or_else(|e| {
        eprintln!("(setup: relay prompt failed: {e:#}; defaulting to n0)");
        None
    });

    let exe = std::env::current_exe().context("current_exe")?;
    let cc_connect = exe
        .parent()
        .map(|p| p.join("cc-connect"))
        .ok_or_else(|| anyhow!("can't locate `cc-connect` binary next to cc-connect-tui"))?;
    if !cc_connect.exists() {
        return Err(anyhow!(
            "expected `cc-connect` at {} — both binaries must live side-by-side",
            cc_connect.display()
        ));
    }

    let mut cmd = Command::new(&cc_connect);
    cmd.arg("host-bg").arg("start");
    if let Some(r) = resolved_relay.as_deref() {
        cmd.arg("--relay").arg(r);
    }
    let out = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("spawn {}", cc_connect.display()))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!(
            "host-bg start failed (exit {:?}): {stderr}",
            out.status.code()
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let ticket = stdout
        .lines()
        .find_map(|l| {
            l.split_whitespace()
                .find(|w| w.starts_with("cc1-"))
                .map(|s| s.to_string())
        })
        .ok_or_else(|| anyhow!("host-bg start did not print a ticket; output was:\n{stdout}"))?;
    println!("[room] daemon started, joining…");

    enter_tui(
        ticket,
        resolved_relay.as_deref(),
        claude_args,
        /* hosting */ true,
    )
    .await
}

async fn join(
    ticket: &str,
    relay: Option<&str>,
    nick_override: Option<&str>,
    claude_args: Vec<String>,
) -> Result<()> {
    if let Err(e) = setup::ensure_hook_installed() {
        eprintln!("(setup: hook check failed: {e:#})");
    }
    if let Err(e) = setup::ensure_mcp_installed() {
        eprintln!("(setup: mcp install failed: {e:#})");
    }
    if let Err(e) = setup::ensure_self_nick(nick_override) {
        eprintln!("(setup: nick prompt failed: {e:#})");
    }
    enter_tui(
        ticket.to_string(),
        relay,
        claude_args,
        /* hosting */ false,
    )
    .await
}

async fn enter_tui(
    ticket: String,
    relay: Option<&str>,
    claude_args: Vec<String>,
    hosting: bool,
) -> Result<()> {
    let topic_hex = topic_hex_from_ticket(&ticket)?;
    let claude_bin = std::env::var("CC_CONNECT_CLAUDE_BIN")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "claude".to_string());
    let mut claude_argv = vec![claude_bin];
    claude_argv.extend(claude_args);

    // Auto-reply + bootstrap (matches the wrapper script used by
    // exec_zellij / exec_tmux). When room.rs's exec_tui_fallback wrote
    // both prompt files and set the env vars, prepend
    // `--append-system-prompt <content>` and append the bootstrap as
    // the initial user prompt. This is what makes the TUI path also
    // auto-greet + enter the listener loop on launch.
    apply_auto_reply_bootstrap(&mut claude_argv);

    let claude_cwd: Option<PathBuf> = std::env::current_dir().ok();

    let opts = RunOpts {
        ticket,
        topic_hex,
        no_relay: false,
        relay: relay.map(|s| s.to_string()),
        claude_argv,
        claude_cwd,
        hosting,
    };
    cc_connect_tui::run(opts).await
}

fn topic_hex_from_ticket(ticket: &str) -> Result<String> {
    let bytes =
        decode_room_code(ticket).with_context(|| format!("decode ticket: {ticket:.20}…"))?;
    let payload = TicketPayload::from_bytes(&bytes).context("parse ticket payload")?;
    let mut hex = String::with_capacity(64);
    for b in payload.topic.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    Ok(hex)
}

/// If the launcher (`cc-connect/src/room.rs::exec_tui_fallback`) wrote
/// both the auto-reply system prompt and the bootstrap user prompt, and
/// the user hasn't opted out via `CC_CONNECT_NO_AUTO_REPLY`, prepend
/// `--append-system-prompt <content>` to claude's argv (after the bin)
/// and append the bootstrap content as the trailing positional. Mirrors
/// what `layouts/claude-wrap.sh` does on the multiplexer paths so all
/// three paths boot claude into "say hello + enter listener loop"
/// identically.
fn apply_auto_reply_bootstrap(claude_argv: &mut Vec<String>) {
    if claude_argv.is_empty() {
        return;
    }
    if std::env::var_os("CC_CONNECT_NO_AUTO_REPLY")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        return;
    }
    let auto_reply_path = match std::env::var_os("CC_CONNECT_AUTO_REPLY_FILE") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => return,
    };
    let bootstrap_path = match std::env::var_os("CC_CONNECT_BOOTSTRAP_FILE") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => return,
    };
    let auto_reply = match std::fs::read_to_string(&auto_reply_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let bootstrap = match std::fs::read_to_string(&bootstrap_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    // claude_argv[0] is the bin. Insert the system-prompt flag
    // immediately after it, before any user-supplied trailing args, so
    // those user args still apply. Append the bootstrap as the last
    // positional — claude treats it as the initial user prompt.
    claude_argv.insert(1, "--append-system-prompt".into());
    claude_argv.insert(2, auto_reply);
    claude_argv.push(bootstrap);
}
