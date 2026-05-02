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
bun install
bun run compile          # → out/extension.js
```

Then in VSCode, **Run > Start Debugging** (F5) with the
"Run Extension" launch configuration the extension generates on first
F5, or open `vscode-extension/` as the workspace and F5 directly.
The Extension Development Host opens; run `cc-connect: Hello` from
the command palette to verify.

`bun run watch` keeps `tsc` in `--watch` mode for iterative dev.
