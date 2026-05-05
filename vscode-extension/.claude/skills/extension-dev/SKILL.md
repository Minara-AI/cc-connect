---
name: extension-dev
description: Build / F5 / debug the cc-connect VSCode extension. Use when you're iterating on src/ or webview/ in this directory and need to know which command to run, how to inspect runtime state, or how to debug specific subsystems (webview rendering, Claude SDK driver, daemon spawn, log tail, IPC over chat.sock).
---

# cc-connect extension dev — operational skill

## When to use this

Invoke whenever the task involves:

- editing `src/` (extension host TS) or `webview/` (React TS) and
  needing to compile + reload the dev host
- diagnosing why a change didn't appear (build cache vs dev-host reload
  vs JS bundle staleness)
- understanding which subsystem owns which behavior (sidebar tree,
  panel webview, Claude runner, log tail, chat.sock client, daemon
  orchestrator)
- debugging the Claude Agent SDK plumbing without burning subscription
  quota

For architecture / design rationale, read `../CLAUDE.md` and
`../docs/vscode-extension-design.md` first — this skill is the
operational layer.

## The dev loop

```bash
# From vscode-extension/
bun install              # one-time
bun run compile          # build host TS + webview bundle
bun run watch            # tsc --watch (host only — webview needs manual rebuild)
```

F5 in VSCode launches the dev host. The bundled launch config
(`vscode-extension/.vscode/launch.json`) auto-runs `bun run compile`
as a pre-launch task and disables user-installed extensions in the
dev host (`--disable-extensions` flag).

**Three layers of "did my change land?"** — when something looks
stale, check in this order:

1. **Host TS** (`src/` → `out/extension.js`): F5 always rebuilds via
   the pre-launch task, but `Cmd-R` reload inside dev host does NOT.
   Close + reopen the dev host window after host changes.
2. **Webview bundle** (`webview/` → `dist/webview/main.js`): NOT
   rebuilt by F5. Re-run `bun run compile` (or
   `bun run compile:webview`) after every TSX change. Then reload
   the webview view in dev host (close + reopen the panel).
3. **Manifest** (`package.json` contributions): adding commands /
   views / viewsContainers / activationEvents requires a fresh dev
   host launch. `Cmd-R` won't pick those up.

## Subsystem map

| File | Owns |
|---|---|
| `src/extension.ts` | `activate()` — registers commands + providers |
| `src/sidebar/RoomsProvider.ts` | Activity-bar tree of rooms |
| `src/panel/RoomPanelProvider.ts` | Bottom-panel webview view (single active room) |
| `src/host/daemon.ts` | Spawns / stops `cc-connect host-bg` + `chat-daemon` |
| `src/host/log_tail.ts` | Tails `~/.cc-connect/rooms/<topic>/log.jsonl` |
| `src/host/ipc.ts` | Sends to chat-daemon over `chat.sock` |
| `src/host/claude_runner.ts` | Drives `@anthropic-ai/claude-agent-sdk` |
| `src/host/mention.ts` | Decides which messages wake Claude |
| `webview/main.tsx` | React root + postMessage bridge |
| `webview/Chat.tsx` | Chat panel (input + scrollback) |
| `webview/Claude.tsx` | Claude panel (typed event stream) |
| `webview/processClaude.ts` | SDK events → typed UI blocks |
| `webview/MarkdownContent.tsx` | react-markdown wrapper |
| `webview/ToolCardBody.tsx` | Diff view + expandable result |
| `webview/highlightMentions.tsx` | @-token regex highlighter |
| `webview/useStickyScroll.ts` | Sticky-bottom scroll hook |

## Smoke tests (zero quota)

```bash
bun run probe:sdk        # confirms SDK + OAuth + ~/.local/bin/claude path; aborts at init event
```

Aborts on the first `system:init` event so no model call lands.
Use this to validate `claude_runner.ts` changes before doing a full
end-to-end test against a real Room.

## Inspecting runtime state

```bash
# Active daemons (per-machine)
~/.local/bin/cc-connect host-bg list
~/.local/bin/cc-connect chat-daemon list

# Per-Room runtime state
ls ~/.cc-connect/rooms/<topic>/
#   log.jsonl       — message log (the tail watches this)
#   chat.sock       — marker file pointing at the actual /tmp/cc-*.sock
#   chat-daemon.pid — JSON: { pid, topic, ticket, started_at, relay }
#   files/          — dropped blobs (iroh-blobs MemStore copy)

# Hook log (per-machine)
tail -f ~/.cc-connect/hook.log

# Doctor
~/.local/bin/cc-connect doctor
```

The dev host's "Webview Developer Tools" (right-click webview →
Inspect, or Cmd-Shift-I) gives you Chrome DevTools for the React
app — use it to inspect post-message traffic, React state, CSS.

## Common pitfalls

- **"My React change didn't show up"** → run `bun run compile` again,
  then reload the panel view (close + reopen). Webview bundle isn't
  rebuilt by F5 alone.
- **"Activity Bar icon doesn't show"** → manifest change requires
  full dev host restart. Also confirm View → Appearance → Activity
  Bar is enabled in the dev host (it inherits the user's setting).
- **"Tool calls fail with ZodError"** → you tried to use `canUseTool`
  instead of `permissionMode: 'bypassPermissions'`. Some tool paths
  bypass canUseTool and crash. Stick with bypass for v0; per-tool
  approval UI is deferred.
- **"Send fails with no chat.sock marker"** → no chat-daemon running
  for that topic. Either `cc-connect: Start Room` (mints a new one)
  or `cc-connect: Join Room…` (joins existing) — both spawn
  chat-daemon as part of the flow.
- **"Claude doesn't wake on `@<my-nick>`"** → that's the design
  (D1). Use `@<my-nick>-cc` to address your own AI explicitly.
- **"Webview is blank after switching rooms"** → `RoomPanelProvider`
  regenerates HTML on `setRoom`. If `bun build` produced a broken
  bundle, you'll see a blank screen. Check the webview DevTools
  console for the actual JS error.

## When the design needs to change

If you're touching the wire format, hook contract, or webview trust
boundary, **stop and read** `../docs/vscode-extension-design.md` §2.1
(Isolation contract) and §9 (Validation results). Anything that
loosens the boundary needs a paired update to the design doc + a
smoke test.

## Related references

- vitest-dev/vscode/CLAUDE.md — well-structured CLAUDE.md from a
  popular VSCode extension; good template (license: MIT)
- sugyan/claude-code-webui — MIT React UI for Claude CLI streaming;
  inspired our `processClaude.ts` architecture (already attributed
  in the source)
