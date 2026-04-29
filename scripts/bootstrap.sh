#!/usr/bin/env bash
# bootstrap.sh — true one-liner clone + install for cc-connect.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Minara-AI/minara-connect/main/scripts/bootstrap.sh | bash
#
# Override the install location (default: ~/cc-connect):
#   curl -fsSL <…/bootstrap.sh> | CC_CONNECT_DIR=/opt/cc-connect bash
#
# What this does:
#   1. clone (or git pull) the repo into $CC_CONNECT_DIR
#   2. cd into it and run install.sh (toolchain check, build, hook + mcp
#      registration in ~/.claude/settings.json, doctor smoke check)
#   3. print the next-step command

set -euo pipefail

REPO=https://github.com/Minara-AI/minara-connect.git
DEST="${CC_CONNECT_DIR:-$HOME/cc-connect}"

if ! command -v git >/dev/null 2>&1; then
  echo "bootstrap: git is required" >&2
  exit 1
fi

if [[ -d "$DEST/.git" ]]; then
  echo "[bootstrap] cc-connect already cloned at $DEST — pulling latest"
  git -C "$DEST" pull --ff-only
else
  echo "[bootstrap] cloning into $DEST"
  git clone "$REPO" "$DEST"
fi

cd "$DEST"
# When invoked via `curl … | bash`, our own stdin is the bootstrap script.
# Redirect install.sh's stdin from /dev/tty so its prompts read from the
# terminal instead of consuming bootstrap's heredoc body.
if [[ -r /dev/tty ]]; then
  ./install.sh </dev/tty
else
  ./install.sh
fi

cat <<EOF

──────────────────────────────────────────────────────────────────
✓ cc-connect installed at: $DEST

Recommended start:
  $DEST/target/release/cc-connect room start

The first run prompts for nickname + relay choice. Press Tab / F2 to
switch between the chat and Claude panes; Ctrl-Q quits cleanly.
──────────────────────────────────────────────────────────────────
EOF
