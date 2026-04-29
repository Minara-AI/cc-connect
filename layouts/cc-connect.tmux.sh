#!/usr/bin/env bash
# cc-connect tmux launcher. Spawns (or attaches to) one tmux session named
# `cc-connect` with one window per room, claude L + cc-chat-ui R.
#
# The first room creates the session; subsequent rooms add a new window
# via `tmux new-window -t cc-connect:`. The bottom status line at the
# default tmux config shows the window list, so room navigation is built
# in (Ctrl-b n / Ctrl-b p / Ctrl-b w).
#
# Substitutions performed by room.rs at launch time:
#   __CLAUDE_WRAPPER__  → absolute path to claude-wrap.sh (which exports
#                          CC_CONNECT_ROOM from its first argv).
#
# Reads from env (set by room.rs's Command::env):
#   CC_CONNECT_ROOM    — topic hex for the new room
#   CC_CONNECT_TMUX_SESSION (optional, default `cc-connect`)
#   CC_CONNECT_CLAUDE_LAUNCHER (optional override of the wrapper path)
#   CC_CONNECT_CHAT_UI_BIN (optional override of cc-chat-ui binary)
set -euo pipefail

if [ -z "${CC_CONNECT_ROOM:-}" ]; then
  echo "cc-connect.tmux.sh: CC_CONNECT_ROOM env var not set" >&2
  exit 2
fi

SESSION="${CC_CONNECT_TMUX_SESSION:-cc-connect}"
WINDOW="${CC_CONNECT_ROOM:0:12}"
CLAUDE_LAUNCH="${CC_CONNECT_CLAUDE_LAUNCHER:-__CLAUDE_WRAPPER__}"
CHAT_UI_BIN="${CC_CONNECT_CHAT_UI_BIN:-cc-chat-ui}"

cat <<'TIPS'
[room] tmux navigation:
  Ctrl-b o      switch focus between claude (left) and chat-ui (right)
  Ctrl-b ←/→    move focus by direction (or arrow keys)
  Ctrl-b w      list windows (one per room) and pick one
  Ctrl-b n / p  next / previous window
  Ctrl-b z      zoom current pane (toggle full-screen)
  Ctrl-b d      detach (room keeps running; reattach with `tmux a -t cc-connect`)
  Ctrl-b &      kill current window (a single room)
  Ctrl-b x      kill current pane

  Prefer the old single-window TUI hotkeys instead? Quit zellij/tmux
  and re-launch with CC_CONNECT_PREFER_TUI=1 cc-connect room start.
TIPS

apply_session_help() {
  # Pin a one-line help string to the right side of the tmux status bar
  # so the keybinds stay visible after attach. Session-scoped so it
  # doesn't leak to the user's other tmux sessions.
  tmux set-option -t "$SESSION" status-right-length 80 2>/dev/null || true
  tmux set-option -t "$SESSION" status-right \
    "#[fg=yellow]Ctrl-b o:pane Ctrl-b w:windows Ctrl-b d:detach #[default]" \
    2>/dev/null || true
}

if tmux has-session -t "$SESSION" 2>/dev/null; then
  # Existing session — add a new window for this room and switch focus to it.
  tmux new-window -t "$SESSION:" -n "$WINDOW" \
    -e "CC_CONNECT_ROOM=$CC_CONNECT_ROOM" \
    "$CLAUDE_LAUNCH $CC_CONNECT_ROOM"
  tmux split-window -h -t "$SESSION:$WINDOW" -p 40 \
    -e "CC_CONNECT_ROOM=$CC_CONNECT_ROOM" \
    "$CHAT_UI_BIN --topic $CC_CONNECT_ROOM"
  tmux select-pane -t "$SESSION:$WINDOW.0"
  apply_session_help

  # If we're already inside a tmux client (this script invoked from inside
  # the same session), select the new window. Otherwise attach.
  if [ -n "${TMUX:-}" ]; then
    tmux select-window -t "$SESSION:$WINDOW"
  else
    exec tmux attach-session -t "$SESSION"
  fi
else
  # Fresh session.
  tmux new-session -d -s "$SESSION" -n "$WINDOW" -x 220 -y 50 \
    -e "CC_CONNECT_ROOM=$CC_CONNECT_ROOM" \
    "$CLAUDE_LAUNCH $CC_CONNECT_ROOM"
  tmux split-window -h -t "$SESSION:$WINDOW" -p 40 \
    -e "CC_CONNECT_ROOM=$CC_CONNECT_ROOM" \
    "$CHAT_UI_BIN --topic $CC_CONNECT_ROOM"
  tmux select-pane -t "$SESSION:$WINDOW.0"
  apply_session_help
  exec tmux attach-session -t "$SESSION"
fi
