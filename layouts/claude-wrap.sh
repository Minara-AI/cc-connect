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

# Initial user prompt that boots claude straight into "say hello,
# then listen". Claude Code doesn't auto-execute its system prompt —
# it sits idle until something arrives on the user channel — so we
# hand it a tiny first turn. Two steps:
#   1. Send a brief greeting via cc_send so peers see this AI just
#      came online (peers are humans + other AIs in the room).
#   2. Enter the ambient listener loop per the system prompt.
# The wording stays prescriptive but leaves the greeting itself to
# claude's discretion — it knows the room's tone better than we do.
BOOTSTRAP_PROMPT="You just joined the cc-connect room. Do these two things in order:

1. Send a single brief greeting line to the room via the \`cc_send\` MCP tool. Terse, dev-to-dev, one sentence. Don't introduce yourself with a long bio; the room already sees your nick. Skip if you've been told elsewhere not to greet.

2. Immediately call \`cc_wait_for_mention\` (no \`since_id\` on this first call). Follow the listener-loop directive in your system prompt from there: re-arm on \`null\`, reply via \`cc_send\`/\`cc_at\` on a hit, then re-arm with \`since_id = id\`."

if [ -z "${CC_CONNECT_NO_AUTO_REPLY:-}" ] && [ -f "$PROMPT_FILE" ]; then
  exec "$CLAUDE" --append-system-prompt "$(cat "$PROMPT_FILE")" "$@" "$BOOTSTRAP_PROMPT"
fi
exec "$CLAUDE" "$@"
