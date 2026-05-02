# cc-connect — VSCode extension

Native VSCode integration of [cc-connect](../README.md). Open the
extension → start or join a Room → side-by-side chat panel + Claude
panel inside VSCode, no terminal involved.

> **Status: pre-alpha scaffold.** v0 only registers a hello command;
> Room runtime, webview, and Claude orchestration land in subsequent
> commits. Design doc: [`../docs/vscode-extension-design.md`](../docs/vscode-extension-design.md).

## Develop

```bash
cd vscode-extension
bun install              # one-time
```

Then open this directory in VSCode (`code .` from inside
`vscode-extension/`, or `code vscode-extension/` from the repo root)
and press **F5**. The bundled launch config picks "Run cc-connect
Extension", runs `bun run compile` as a pre-launch task, and opens
the Extension Development Host.

In the dev host window, Cmd-Shift-P → run either:
- `cc-connect: Hello` — toast smoke test
- `cc-connect: Open Room (placeholder)` — opens the two-pane webview
  (chat / claude placeholders) and lets you exercise the
  postMessage round-trip via the "Echo to host" button

`bun run watch` keeps `tsc` in `--watch` mode for iterative dev (the
watch task is also available from VSCode → Terminal → Run Task →
"watch" if you prefer in-editor).
