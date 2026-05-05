# cc-connect VSCode extension — agent guide

This directory is the cc-connect VSCode extension. Use it WITH the
top-level [`/CLAUDE.md`](../CLAUDE.md) — that file describes the
Rust + chat-ui side and the protocol; this file describes the
extension itself.

## Mental model

The extension is **purely TypeScript / no native code**. It:

1. **Sidebar** (`src/sidebar/RoomsProvider.ts`) — TreeDataProvider
   listing every directory under `~/.cc-connect/rooms/<topic>/` with
   alive/dormant status. Buttons in the view title trigger the
   commands; right-click items invoke per-room actions.
2. **Bottom panel** (`src/panel/RoomPanelProvider.ts`) —
   `WebviewViewProvider` that owns one Room view at a time. Drives
   the chat ↔ Claude split inside.
3. **Webview React app** (`webview/main.tsx` + components) — chat
   list, Claude event stream, mention highlight, markdown render,
   tool cards. Communicates with the extension host over
   `vscode.postMessage`.
4. **Per-Room runtime** (extension host) — for each open Room view:
   - `src/host/log_tail.ts` tails `log.jsonl` (file-watcher pattern)
   - `src/host/ipc.ts` writes back via the chat-daemon's `chat.sock`
   - `src/host/claude_runner.ts` drives `@anthropic-ai/claude-agent-sdk`
     `query()` per @-mention turn
5. **Daemon orchestration** (`src/host/daemon.ts`) — wraps
   `~/.local/bin/cc-connect host-bg start` and
   `cc-connect chat-daemon start <ticket>` so the extension can
   start/stop Rooms without dropping to a terminal.

The extension **does not** parse the gossip wire protocol or speak
iroh directly. All P2P concerns stay in the Rust crates.

## Read these before deep work

- [`../docs/vscode-extension-design.md`](../docs/vscode-extension-design.md) —
  the design doc. Pinned decisions, validation results, deferred items.
- [`../CONTEXT.md`](../CONTEXT.md) — Ubiquitous Language. Use Room /
  Peer / Substrate / Hook / Cursor / Session verbatim; never drift to
  "channel" / "client" / "user".
- [`../PROTOCOL.md`](../PROTOCOL.md) — wire spec. Read when touching
  anything the extension reads from `~/.cc-connect/`.
- [`../SECURITY.md`](../SECURITY.md) — threat model. Read before
  loosening any webview CSP, opening `chat.sock` from the webview, or
  changing how `CC_CONNECT_ROOM` is set on the spawned `claude`.

## Commands you can't infer from the code

```bash
# One-time install
bun install

# Default dev loop
bun run compile          # tsc → out/extension.js + bun build → dist/webview/main.js
bun run watch            # tsc --watch only (extension host)
bun run probe:sdk        # smoke-test the Claude Agent SDK against ~/.local/bin/claude

# Type-check the webview separately (tsc in the main project excludes it)
bunx tsc -p tsconfig.webview.json
```

F5 from this directory opens the Extension Development Host. The
launch config (`.vscode/launch.json`) auto-runs `bun run compile`
as a pre-launch task and disables user-installed extensions in the
dev host so they don't pollute your test session.

## Non-obvious gotchas

- **Two separate TS configs.** `tsconfig.json` covers
  `src/extension.ts` (extension host, Node target, CommonJS output to
  `out/`). `tsconfig.webview.json` covers `webview/` (DOM target,
  classic React JSX, no emit — bun bundles the webview). Don't merge
  them; the targets are incompatible.
- **Webview is bundled, not transpiled.** `bun build webview/main.tsx`
  produces a single `dist/webview/main.js` ESM bundle. If you add a
  new webview-side dep, it's pulled in here. Bundle size shows up on
  every `bun run compile` output line — keep it under ~2 MB or
  switch to chunk splitting.
- **CSP is strict** (`default-src 'none'; script-src
  ${webview.cspSource}; style-src ${webview.cspSource} 'unsafe-inline'`).
  Inline `<script>` is blocked; only the asWebviewUri-rewritten
  `dist/webview/main.js` runs. Don't add remote origins; per design
  §2.1 peer chat is treated as untrusted content.
- **`process.env.CC_CONNECT_ROOM`** must be set on every `claude`
  child — the cc-connect-hook gates injection on this env. The
  Claude runner does this via `query({ options: { env: { ...,
  CC_CONNECT_ROOM: topic } } })`. Don't pass `env: {}` (would clobber
  PATH / OAuth / HOME).
- **`pathToClaudeCodeExecutable`** must point at
  `~/.local/bin/claude`. macOS GUI launches don't inherit the user's
  shell PATH (launchctl env), and the SDK's bundled native binary
  may fail to extract on some installs (we hit this with
  `@anthropic-ai/claude-agent-sdk-darwin-arm64`).
- **`permissionMode: 'bypassPermissions'`** is the v0 trust posture.
  `canUseTool` callbacks miss some paths and crash the turn with a
  ZodError under headless. Real per-tool approval UI is deferred —
  see design §8.
- **Bare `@<my-nick>` does NOT wake Claude** in this extension. Only
  `@<my-nick>-cc` and broadcast tokens (`@cc`/`@claude`/`@all`/`@here`)
  do. This is a deliberate narrowing of the Rust
  `hook_format::mentions_self` semantics — see
  `src/host/mention.ts`.
- **`Activity Bar SVG`** must use `fill="currentColor"` on a single
  path. Stroke-only or multi-path SVGs sometimes refuse to render
  in monochrome — we hit this once.
- **Two TS package shapes for `Message`** — `src/types.ts` (host),
  `webview/types.ts` (webview). They're hand-kept in sync because
  the webview rootDir excludes `src/`. If you change the shape,
  update both + any tests.

## Workflow

- **Run `bun run compile` after every webview source change.** F5
  doesn't auto-rebuild the webview — it only rebuilds the host TS.
  Stale `dist/webview/main.js` is the most common cause of "my React
  change didn't show up" confusion.
- **Reload the dev host after manifest changes.** Adding a command,
  view, viewsContainers entry, or activationEvent to `package.json`
  requires a fresh dev host launch (Cmd-R / Ctrl-R won't reload the
  manifest).
- **Test against a real Room.** `~/.cc-connect/rooms/<topic>/` must
  exist with a live `chat-daemon.pid` for chat send + Claude wake to
  work. Use `cc-connect: Start Room` from the dev host's command
  palette (which exercises `daemon.ts::startHostBg` +
  `startChatDaemon`).
- **Smoke-test the SDK plumbing without burning quota.**
  `bun run probe:sdk` aborts at the init event so subscription quota
  isn't consumed; useful when iterating on `claude_runner.ts`.

## Repository etiquette

- Commit messages: `feat(vscode-extension): ...`, `fix(vscode-extension): ...`,
  `chore(vscode-extension): ...`. Match the rest of the repo's style.
- Reference design-doc sections explicitly when adding features
  (e.g. "per design D1", "per §4.4"). Keeps the design and the code
  in lockstep.
- Anything that changes the manifest's command/view contributions
  needs a paired update to the README's command list and (if
  user-facing) to the design doc.

## What NOT to do

- Don't open `chat.sock` from the webview. Webview is a sandbox; all
  IPC goes through the extension host. Per design §2.1.
- Don't pull in heavyweight UI libraries. We use `react-markdown` +
  `remark-gfm` and that's already pushing 1.4 MB. No syntax
  highlighter, no styled-components, no design-system kit.
- Don't read leaked Claude Code source for "inspiration". The
  `~/work/claude-code-main` repo (and clones thereof on GitHub) are
  proprietary — using them taints the cc-connect MIT/Apache stance.
- Don't widen the `cc_drop` blocklist or change the hook's trust
  boundary on this side. Those rules live in the Rust crates and the
  PROTOCOL spec.
- Don't bypass `permissionMode` per-tool with a custom `canUseTool`
  unless you've also wired a real webview-side approval UI.
  Half-measures crash the turn with a ZodError.

## Related skills

`.claude/skills/extension-dev/SKILL.md` covers the build / debug
loop step by step — read it before doing anything beyond a small
text fix.

## Recommended MCPs (optional)

None of these are auto-installed (avoids forcing API keys on
contributors). Add to your own `~/.claude.json` or a project-level
`.mcp.json` if useful:

- **context7** — fetches up-to-date library docs by name (no API key
  required). Most useful when looking up VSCode API,
  `@anthropic-ai/claude-agent-sdk`, or React types.
  ```json
  { "mcpServers": { "context7": {
      "command": "npx",
      "args": ["-y", "@upstash/context7-mcp@latest"]
  } } }
  ```
- **filesystem** (official `@modelcontextprotocol/servers/filesystem`)
  — scoped read/write under a chosen root. Useful when iterating
  outside the workspace folder.
- **claude-context** (`@zilliz/claude-context-mcp`) — full-codebase
  semantic search. Powerful but **needs OpenAI key + Milvus/Zilliz
  Cloud**; skip unless you already have those credentials.
- **`tjx666/vscode-mcp`** — exposes VSCode's own LSP (real-time
  diagnostics, hover, refs) to Claude. Skipped here because it uses
  the **Anti 996** non-standard license and requires installing a
  companion VSCode extension. Mention it for awareness; evaluate
  the license before opting in.
