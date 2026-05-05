# Restart Claude Code

The hook + MCP entries are registered in:

- `~/.claude/settings.json` — the UserPromptSubmit hook entry
- `~/.claude.json` — the `mcpServers.cc-connect` entry

Both are read **at Claude Code startup**, so a running Claude Code
process won't pick them up until the next launch.

Quit fully — on macOS that's **Cmd-Q**, not just closing the window
(closing the window leaves Claude Code running in the dock /
background).

Once you reopen Claude Code, the cc-connect MCP server is auto-
spawned per Claude Code session, and the hook fires on every
prompt you submit while inside a Room.

The VSCode extension itself doesn't need a restart — it spawns a
**fresh** Claude Code subprocess per Room view it opens, with the
right environment baked in.
