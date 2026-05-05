# Verify the install

`cc-connect doctor` runs a sanity check across the install surface:

- The hook entry in `~/.claude/settings.json` exists and points at
  the right binary.
- The MCP entry in `~/.claude.json` exists and points at the right
  binary.
- `~/.cc-connect/identity.key` exists with mode 0600.
- All four binaries exist and are recent.

Sample healthy output:

```
✓ cc-connect-hook entry: /Users/you/.local/bin/cc-connect-hook
✓ cc-connect-mcp entry: /Users/you/.local/bin/cc-connect-mcp
✓ identity ok (mode 600)
✓ binaries up-to-date
```

If anything is red:

- Re-run `bootstrap.sh` (idempotent — fixes drift).
- For a deeper reset: `cc-connect uninstall --purge`, then
  `bootstrap.sh` again.

Once the doctor is green, the **Refresh** button on the Rooms tree
title bar (or this command link) flips the welcome view from
"needs setup" to "no rooms yet — start one".
