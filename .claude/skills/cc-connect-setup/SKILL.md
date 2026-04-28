---
name: cc-connect-setup
description: Walk the user through installing cc-connect — check toolchain, run the build, wire the UserPromptSubmit hook into ~/.claude/settings.json, and verify with `cc-connect doctor`. Use when the user asks to "install cc-connect", "set up cc-connect", or anything in that intent.
---

# cc-connect setup

Goal: get the user from a fresh clone to a working `cc-connect host` / `cc-connect chat <ticket>` setup, with the `UserPromptSubmit` hook wired so chat lines actually surface in their Claude Code context.

The repo ships an interactive `install.sh` that does the heavy lifting. Your job is to make sure it runs cleanly, surface what's happening at each step, and help the user past common failure modes.

## Decide which mode to run in

1. Check the working directory is a cc-connect clone — `Cargo.toml` at the root with a `[workspace]` table and a `cc-connect-core` member. If not, ask the user to `cd` into their clone (or clone the repo) and re-invoke.
2. Check if cc-connect is already installed — look for `target/release/cc-connect`, the hook entry in `~/.claude/settings.json`, and `~/.cc-connect/identity.key`. Tell the user what's already in place so they don't re-do work.

## Run the installer

Default to running `./install.sh` interactively (no `--yes`) so the user sees each prompt. Stream the output to the conversation. The script:

- verifies `rustc` / `cargo` (offers `rustup` install if missing)
- runs `cargo build --workspace --release` (5-10 min on a cold cache)
- backs up `~/.claude/settings.json` (timestamped `.bak`) and merges the `UserPromptSubmit` hook entry idempotently
- runs `cc-connect doctor` at the end

If the user wants unattended install (CI, second machine), use `./install.sh --yes`. To skip the slow build (already built), use `--skip-build`.

## Failure modes & fixes

- **`cargo build` fails on `ed25519-3.0.0-rc.4`** — the upstream RC is broken against current `pkcs8`. The repo vendors a patched copy under `vendored/` and a `[patch.crates-io]` block in the workspace `Cargo.toml`. If the build fails here, the user's clone is missing `vendored/`. Tell them to re-clone or `git fetch origin main && git reset --hard origin/main`.
- **`rustup` install refused** — the user may want to install Rust via Homebrew / their distro's package manager. Either is fine as long as `rustc --version` shows ≥ 1.85. Re-run `./install.sh --skip-build` after Rust is on PATH? No — let the build run, otherwise the build flag check (which validates the Rust toolchain) is meaningless.
- **`~/.claude/settings.json` already has a `UserPromptSubmit` hook from another tool** — `install.sh` removes prior entries that point at our `cc-connect-hook` binary, but leaves other tools' entries alone. Confirm with the user by `cat`ing the file before/after and showing the diff.
- **Doctor reports `[FAIL]`** — read the message. Common ones:
  - `hook entry: hook command not absolute` → `install.sh` should have written an absolute path; if not, the user edited it after. Re-run.
  - `identity key: file mode 0644 (want 0600)` → `chmod 600 ~/.cc-connect/identity.key`.
  - `active-rooms dir: …` → it's auto-created on first `cc-connect chat`. `[--]` (info) is fine.

## After install

Tell the user:
1. **Restart Claude Code** so it picks up the new hook (Claude Code reads `settings.json` on launch).
2. To host: `./target/release/cc-connect host`. To join: `./target/release/cc-connect chat <ticket>`. Add `--no-relay` for LAN-only.
3. To verify the magic moment, follow the `Two-laptop demo procedure` section in `README.md`.

## Things you should NOT do

- Don't edit `~/.claude/settings.json` directly — always go through `install.sh`. Direct edits skip the backup + idempotency logic and risk corrupting the user's other hooks.
- Don't run `cargo install` — cc-connect isn't on crates.io yet (vendored ed25519 patches block publish; see `TODOS.md`). Always build from the local clone.
- Don't suggest disabling the vendored `[patch.crates-io]` block. Yes, it's ugly. Yes, it's needed. The reason is documented in `TODOS.md` and the comments at the bottom of the workspace `Cargo.toml`.
