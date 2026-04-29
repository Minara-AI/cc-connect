#!/bin/sh
# cc-connect claude wrapper.
#
# Spawned in place of `claude` by the room launcher (zellij KDL or tmux
# script). Two responsibilities:
#
#   1. If the first arg is a 64-char hex topic, export it as
#      CC_CONNECT_ROOM. This is how zellij's `action new-tab` path
#      passes the topic into a freshly spawned pane — env vars from
#      the parent invocation don't propagate through zellij's daemon.
#
#   2. If $CC_CONNECT_AUTO_REPLY_FILE points at an existing file —
#      room.rs writes that file unless CC_CONNECT_NO_AUTO_REPLY=1 —
#      claude starts with that file's content appended to its system
#      prompt, which arms the auto-reply listener loop. Otherwise
#      falls through to plain claude.
#
# Embedded into the cc-connect binary via include_str! and written to
# /tmp/cc-connect-$UID/claude-wrap.sh at launch time.

# If first arg looks like a topic hex (64 chars, lowercase hex), consume
# it as CC_CONNECT_ROOM. Anything else falls through to claude verbatim.
#
# Length + content split (rather than `[0-9a-f]` × 64 in a single case
# pattern) so we can't quietly miscount the brackets. POSIX-portable.
if [ "$#" -gt 0 ] && [ "${#1}" -eq 64 ]; then
  case "$1" in
    *[!0-9a-f]*) ;;  # contains a non-hex char — not a topic
    *)
      export CC_CONNECT_ROOM="$1"
      shift
      ;;
  esac
fi

PROMPT_FILE="${CC_CONNECT_AUTO_REPLY_FILE:-${TMPDIR:-/tmp}/cc-connect-$(id -u)/auto-reply.md}"
CLAUDE="${CC_CONNECT_CLAUDE_BIN:-claude}"

# Initial user prompt that boots claude straight into the listener
# loop. Claude Code doesn't auto-execute its system prompt — it sits
# idle until something arrives in the user channel — so we hand it a
# tiny "begin" message. Visible in the first turn of the transcript;
# the system prompt installed by --append-system-prompt explains how
# to interpret it.
BOOTSTRAP_PROMPT="cc-connect ambient mode: enter the listener loop now (call cc_wait_for_mention with no since_id, follow the directive in your system prompt)."

if [ -z "${CC_CONNECT_NO_AUTO_REPLY:-}" ] && [ -f "$PROMPT_FILE" ]; then
  exec "$CLAUDE" --append-system-prompt "$(cat "$PROMPT_FILE")" "$@" "$BOOTSTRAP_PROMPT"
fi
exec "$CLAUDE" "$@"
