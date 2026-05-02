import * as React from 'react';
import { createRoot } from 'react-dom/client';
import { Chat } from './Chat';
import type { Message } from './types';

declare global {
  interface Window {
    acquireVsCodeApi(): {
      postMessage(msg: unknown): void;
      setState(state: unknown): void;
      getState(): unknown;
    };
  }
}

const vscode = window.acquireVsCodeApi();

const MOCK_MESSAGES: Message[] = [
  {
    id: '01J0000000000000000000000A',
    author: 'k7zptbase32example',
    nick: 'alice',
    ts: 1714650000000,
    kind: 'chat',
    body: 'Redis or Postgres for the new service?',
  },
  {
    id: '01J0000000000000000000000B',
    author: 'h6yqwbase32example',
    nick: 'bob',
    ts: 1714650005000,
    kind: 'chat',
    body: 'postgres, we have it everywhere already',
  },
  {
    id: '01J0000000000000000000000C',
    author: 'h6yqwbase32example',
    nick: 'bob',
    ts: 1714650010000,
    kind: 'chat',
    body: '@alice can you double-check the migration plan before we lock it in?',
  },
  {
    id: '01J0000000000000000000000D',
    author: 'k7zptbase32example',
    nick: 'alice',
    ts: 1714650020000,
    kind: 'chat',
    body: 'going Postgres per the chat — drafting the migration now',
  },
];

function App(): React.ReactElement {
  const [status, setStatus] = React.useState('waiting for host…');

  React.useEffect(() => {
    const onMsg = (event: MessageEvent) => {
      const msg = (event.data ?? {}) as { type?: string; body?: unknown };
      if (msg.type === 'host:ready') {
        setStatus('host ready ✓');
      } else if (msg.type === 'echo:reply') {
        setStatus(`host replied: ${String(msg.body)}`);
      }
    };
    window.addEventListener('message', onMsg);
    return () => window.removeEventListener('message', onMsg);
  }, []);

  const onEcho = (): void => {
    vscode.postMessage({
      type: 'echo:request',
      body: `ping at ${new Date().toISOString()}`,
    });
  };

  return (
    <React.Fragment>
      <h1>cc-connect — placeholder</h1>
      <div className="panes">
        <Chat messages={MOCK_MESSAGES} myNick="alice" />
        <div className="pane">
          <h2>claude</h2>
          <div className="muted">(no Claude session — Step 4 will wire SDK)</div>
        </div>
      </div>
      <p className="actions">
        <button onClick={onEcho}>Echo to host</button>
      </p>
      <p className="status">{status}</p>
    </React.Fragment>
  );
}

const container = document.getElementById('root');
if (!container) throw new Error('webview root element missing');
createRoot(container).render(<App />);
