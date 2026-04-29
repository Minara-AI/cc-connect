---
name: cc-connect-setup
description: Walk the user through installing cc-connect — check toolchain, run the build, wire the UserPromptSubmit hook into ~/.claude/settings.json, and verify with `cc-connect doctor`. Use when the user asks to "install cc-connect", "set up cc-connect", or anything in that intent.
---

# cc-connect setup

Goal: get the user from a fresh clone to a working `cc-connect room start` setup, with the `UserPromptSubmit` hook wired and the multi-pane chat-ui (Bun + React + Ink) usable next to claude.

The repo ships an interactive `install.sh` that does the heavy lifting. Your job is to make sure it runs cleanly, surface what's happening at each step, and help the user past common failure modes.

## Architecture cheat-sheet (so you can answer questions during setup)

`cc-connect room start/join` opens a 2-pane terminal layout via a multiplexer:

```
┌──────────────────────────┬──────────────┐
│ claude (PTY)             │ cc-chat-ui   │
│ inherits CC_CONNECT_ROOM │ Bun+React+Ink│
│ → hook fires per prompt  │ tails        │
│ → cc_send via MCP        │ log.jsonl    │
└──────────────────────────┴──────────────┘
                |                |
                +--- chat.sock --+
                        |
                  chat-daemon (Rust)
                        |
                  iroh-gossip mesh
```

- **zellij** preferred, **tmux** fallback. If neither is installed, falls back to the legacy single-window `cc-connect-tui`.
- The chat-daemon (`cc-connect chat-daemon start <ticket>`) owns the gossip mesh + chat.sock; it survives any single window so you can detach + re-attach without losing the room.
- chat-ui builds via `bun build --compile` to `target/release/cc-chat-ui` (~60 MB self-contained binary).

## Decide which mode to run in

1. Check the working directory is a cc-connect clone — `Cargo.toml` at the root with a `[workspace]` table and a `cc-connect-core` member. If not, ask the user to `cd` into their clone (or clone the repo) and re-invoke.
2. Check if cc-connect is already installed — look for `target/release/cc-connect`, the hook entry in `~/.claude/settings.json`, and `~/.cc-connect/identity.key`. Tell the user what's already in place so they don't re-do work.

## Run the installer

Default to running `./install.sh` interactively (no `--yes`) so the user sees each prompt. Stream the output to the conversation. The script:

- verifies `rustc` / `cargo` (offers `rustup` install if missing)
- **detects multiplexers**: `zellij` first, `tmux` fallback. If both are present it notes that zellij will be preferred at launch. If neither is found, it warns + offers OS-specific install commands but **does not fail** — `cc-connect room` will fall back to the embedded TUI
- **detects `bun`** for the chat-ui build. If missing, offers `curl https://bun.sh/install | bash`. If user declines, skips the chat-ui build with a warning
- runs `cargo build --workspace --release` (5-10 min on a cold cache)
- builds the chat-ui via `bun install && bun run build` (only if bun is available; ~30 s)
- backs up `~/.claude/settings.json` (timestamped `.bak`) and merges the `UserPromptSubmit` hook entry idempotently
- registers `cc-connect-mcp` (via `claude mcp add` if available, else direct write to `~/.claude.json`)
- symlinks all five binaries (`cc-connect`, `cc-connect-tui`, `cc-connect-hook`, `cc-connect-mcp`, `cc-chat-ui`) into `~/.local/bin`
- runs `cc-connect doctor` at the end

If the user wants unattended install (CI, second machine), use `./install.sh --yes`. To skip the slow build (already built), use `--skip-build` — chat-ui rebuild is also skipped if its binary already exists.

## Failure modes & fixes

- **`cargo build` fails on `ed25519-3.0.0-rc.4`** — the upstream RC is broken against current `pkcs8`. The repo vendors a patched copy under `vendored/` and a `[patch.crates-io]` block in the workspace `Cargo.toml`. If the build fails here, the user's clone is missing `vendored/`. Tell them to re-clone or `git fetch origin main && git reset --hard origin/main`.
- **`rustup` install refused** — the user may want to install Rust via Homebrew / their distro's package manager. Either is fine as long as `rustc --version` shows ≥ 1.85. Re-run `./install.sh --skip-build` after Rust is on PATH? No — let the build run, otherwise the build flag check (which validates the Rust toolchain) is meaningless.
- **`~/.claude/settings.json` already has a `UserPromptSubmit` hook from another tool** — `install.sh` removes prior entries that point at our `cc-connect-hook` binary, but leaves other tools' entries alone. Confirm with the user by `cat`ing the file before/after and showing the diff.
- **Bun installs but isn't on PATH after** — the bun installer writes `~/.bun/bin/bun` and updates the user's shell rc, but the rc isn't sourced in the running install.sh shell. install.sh tries to `export PATH` for the current run, but if that fails it tells the user to open a new shell + re-run with `--skip-build`. Don't worry, the chat-ui binary just won't be built this run.
- **`bun install` fails on a corporate proxy** — bun honors `HTTPS_PROXY` / `npm_config_registry`. Suggest setting those before re-running. As a workaround, the user can skip chat-ui (decline the bun install prompt); `cc-connect room` then falls back to the embedded TUI which is bun-free.
- **`bun build --compile` errors with `Could not resolve: "react-devtools-core"`** — fixed in the committed `package.json` (it's a real dep, not external). If this resurfaces, tell the user to `cd chat-ui && bun add react-devtools-core` and rebuild.
- **Doctor reports `[FAIL]`** — read the message. Common ones:
  - `hook entry: hook command not absolute` → `install.sh` should have written an absolute path; if not, the user edited it after. Re-run.
  - `identity key: file mode 0644 (want 0600)` → `chmod 600 ~/.cc-connect/identity.key`.
  - `active-rooms dir: …` → it's auto-created on first `cc-connect chat-daemon start` or `cc-connect chat`. `[--]` (info) is fine.

## After install

Tell the user:
1. **Restart Claude Code** so it picks up the new hook + MCP tools (Claude Code reads `settings.json` and `~/.claude.json` on launch).
2. **Open a room**: `cc-connect room start` (host a new one) or `cc-connect room join <ticket>` (join someone else's). This launches the multiplexer layout: claude L 60% + cc-chat-ui R 40%, both inheriting `CC_CONNECT_ROOM`.
3. If the user has neither zellij nor tmux installed, `cc-connect room` falls back to the embedded `cc-connect-tui` (single window, claude embedded as PTY). Functionally identical, just less idiomatic.
4. **Hotkeys inside the multiplexer**:
   - **zellij**: `Ctrl-p` then arrow to switch panes, `Ctrl-q` quits zellij (chat-ui owns `Ctrl-Q` inside its pane).
   - **tmux**: `Ctrl-b` then arrow to switch panes (default prefix).
5. **Hotkeys inside cc-chat-ui** (right pane):
   - `Ctrl-Y` copy ticket · `PgUp` / `PgDn` scrollback · `Tab` / `Enter` accept @-mention completion · `Esc` dismiss popup
6. To verify the magic moment, follow the `Two-laptop demo procedure` section in `README.md`.

## Things you should NOT do

- Don't edit `~/.claude/settings.json` directly — always go through `install.sh`. Direct edits skip the backup + idempotency logic and risk corrupting the user's other hooks.
- Don't run `cargo install` — cc-connect isn't on crates.io yet (vendored ed25519 patches block publish; see `TODOS.md`). Always build from the local clone.
- Don't suggest disabling the vendored `[patch.crates-io]` block. Yes, it's ugly. Yes, it's needed. The reason is documented in `TODOS.md` and the comments at the bottom of the workspace `Cargo.toml`.
