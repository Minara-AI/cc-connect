import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand('cc-connect.hello', () => {
      vscode.window.showInformationMessage('cc-connect: hello');
    }),
    vscode.commands.registerCommand('cc-connect.openRoom', () => {
      openRoomPanel(context);
    }),
  );
}

export function deactivate(): void {}

function openRoomPanel(context: vscode.ExtensionContext): void {
  const distRoot = vscode.Uri.joinPath(context.extensionUri, 'dist', 'webview');

  const panel = vscode.window.createWebviewPanel(
    'cc-connect.room',
    'cc-connect — Room',
    vscode.ViewColumn.One,
    {
      enableScripts: true,
      retainContextWhenHidden: true,
      localResourceRoots: [distRoot],
    },
  );

  panel.webview.html = getRoomHtml(panel.webview, distRoot);

  panel.webview.onDidReceiveMessage(
    (msg: { type?: string; body?: unknown }) => {
      if (msg.type === 'echo:request') {
        vscode.window.showInformationMessage(
          `cc-connect: webview said "${String(msg.body)}"`,
        );
        panel.webview.postMessage({ type: 'echo:reply', body: 'pong' });
      }
    },
    undefined,
    context.subscriptions,
  );

  panel.webview.postMessage({ type: 'host:ready' });
}

function getRoomHtml(webview: vscode.Webview, distRoot: vscode.Uri): string {
  const scriptUri = webview.asWebviewUri(
    vscode.Uri.joinPath(distRoot, 'main.js'),
  );
  const csp = [
    "default-src 'none'",
    `script-src ${webview.cspSource}`,
    `style-src ${webview.cspSource} 'unsafe-inline'`,
    `font-src ${webview.cspSource}`,
  ].join('; ');

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta http-equiv="Content-Security-Policy" content="${csp}">
  <title>cc-connect — Room</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; padding: 16px; color: var(--vscode-foreground); background: var(--vscode-editor-background); }
    h1 { font-size: 14px; margin: 0 0 12px; font-weight: 600; }
    h2 { margin: 0 0 8px; font-size: 13px; opacity: 0.7; font-weight: 500; }
    .panes { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
    .pane { border: 1px solid var(--vscode-panel-border); border-radius: 6px; padding: 12px; min-height: 240px; }
    .muted { font-family: var(--vscode-editor-font-family, monospace); font-size: 12px; opacity: 0.6; }
    .chat-log { font-family: var(--vscode-editor-font-family, monospace); font-size: 12px; line-height: 1.55; }
    .chat-line { display: grid; grid-template-columns: 60px 80px 1fr; gap: 8px; padding: 2px 0; align-items: baseline; }
    .chat-line .ts { opacity: 0.4; font-variant-numeric: tabular-nums; }
    .chat-line .nick { font-weight: 600; opacity: 0.85; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .chat-line.me .nick { color: var(--vscode-textLink-foreground); }
    .chat-line .body { opacity: 0.95; word-wrap: break-word; }
    button { font: inherit; padding: 4px 10px; background: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; border-radius: 3px; cursor: pointer; }
    button:hover { background: var(--vscode-button-hoverBackground); }
    .actions { margin-top: 16px; }
    .status { opacity: 0.7; font-size: 12px; }
  </style>
</head>
<body>
  <div id="root"></div>
  <script type="module" src="${scriptUri}"></script>
</body>
</html>`;
}
