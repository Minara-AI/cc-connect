import * as React from 'react';
import { createRoot } from 'react-dom/client';
import { Chat } from './Chat';
import { Claude, type ClaudeRunnerState } from './Claude';
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

type Tab = 'chat' | 'claude';

function App(): React.ReactElement {
  const [status, setStatus] = React.useState('waiting…');
  const [messages, setMessages] = React.useState<Message[]>([]);
  const [myNick, setMyNick] = React.useState('(me)');
  const [topic, setTopic] = React.useState('');
  const [claudeEvents, setClaudeEvents] = React.useState<unknown[]>([]);
  const [claudeState, setClaudeState] = React.useState<ClaudeRunnerState>({
    busy: false,
    queued: 0,
  });
  const [activeTab, setActiveTab] = React.useState<Tab>('chat');
  const [chatUnread, setChatUnread] = React.useState(0);
  const [claudeUnread, setClaudeUnread] = React.useState(0);
  // Track active tab via ref so async message handlers see the
  // current value without re-binding the listener on each switch.
  const activeTabRef = React.useRef(activeTab);
  React.useEffect(() => {
    activeTabRef.current = activeTab;
    if (activeTab === 'chat') setChatUnread(0);
    if (activeTab === 'claude') setClaudeUnread(0);
  }, [activeTab]);

  React.useEffect(() => {
    const onMsg = (event: MessageEvent): void => {
      const msg = (event.data ?? {}) as { type?: string; body?: unknown };
      if (msg.type === 'host:ready') {
        setStatus('ready');
      } else if (msg.type === 'room:reset') {
        setMessages([]);
        setClaudeEvents([]);
        setClaudeState({ busy: false, queued: 0 });
        setTopic('');
        setStatus('switching…');
        setChatUnread(0);
        setClaudeUnread(0);
      } else if (msg.type === 'room:state') {
        const b = (msg.body ?? {}) as { topic?: string; myNick?: string };
        if (b.topic) setTopic(b.topic);
        if (b.myNick) setMyNick(b.myNick);
      } else if (msg.type === 'chat:message') {
        const m = msg.body as Message;
        setMessages((prev) => {
          if (prev.some((x) => x.id === m.id)) return prev;
          return [...prev, m].sort((a, b) => a.id.localeCompare(b.id));
        });
        if (activeTabRef.current !== 'chat') {
          setChatUnread((n) => n + 1);
        }
      } else if (msg.type === 'chat:send-error') {
        setStatus(`send failed: ${String(msg.body)}`);
      } else if (msg.type === 'claude:event') {
        const stamped =
          msg.body && typeof msg.body === 'object'
            ? { ...(msg.body as object), _receivedAt: Date.now() }
            : msg.body;
        setClaudeEvents((prev) => [...prev, stamped]);
        if (activeTabRef.current !== 'claude') {
          // Only count assistant-text-bearing events as "unread"; the
          // hook/system spam is too noisy to surface in the badge.
          const ev = msg.body as { type?: string };
          if (ev?.type === 'assistant') {
            setClaudeUnread((n) => n + 1);
          }
        }
      } else if (msg.type === 'claude:state') {
        const s = msg.body as ClaudeRunnerState;
        setClaudeState({ busy: !!s.busy, queued: s.queued ?? 0 });
      }
    };
    window.addEventListener('message', onMsg);
    return () => window.removeEventListener('message', onMsg);
  }, []);

  const onSend = (body: string): void => {
    vscode.postMessage({ type: 'chat:send', body });
  };

  const onAttach = (): void => {
    vscode.postMessage({ type: 'chat:attach' });
  };

  const onPrompt = (body: string): void => {
    vscode.postMessage({ type: 'claude:prompt', body });
  };

  const onInterrupt = (): void => {
    vscode.postMessage({ type: 'claude:interrupt' });
  };

  const onResetSession = (): void => {
    setClaudeEvents([]);
    setClaudeState({ busy: false, queued: 0 });
    vscode.postMessage({ type: 'claude:reset-session' });
  };

  return (
    <React.Fragment>
      <div className="room-meta">
        <span className="room-meta-topic">
          {topic ? `${topic.slice(0, 14)}…` : '(no room)'}
        </span>
        <span className="room-meta-nick">@{myNick}</span>
        <span className="room-meta-status">{status}</span>
      </div>
      <div className="tab-strip" role="tablist">
        <button
          type="button"
          role="tab"
          className={`tab ${activeTab === 'chat' ? 'active' : ''}`}
          aria-selected={activeTab === 'chat'}
          onClick={() => setActiveTab('chat')}
        >
          <i className="codicon codicon-comment-discussion" />
          <span>Chat</span>
          {chatUnread > 0 && (
            <span className="tab-badge">{chatUnread > 99 ? '99+' : chatUnread}</span>
          )}
        </button>
        <button
          type="button"
          role="tab"
          className={`tab ${activeTab === 'claude' ? 'active' : ''}`}
          aria-selected={activeTab === 'claude'}
          onClick={() => setActiveTab('claude')}
        >
          <i className="codicon codicon-sparkle" />
          <span>Claude</span>
          {claudeState.busy && <i className="codicon codicon-loading codicon-modifier-spin tab-busy" />}
          {claudeUnread > 0 && !claudeState.busy && (
            <span className="tab-badge">{claudeUnread > 99 ? '99+' : claudeUnread}</span>
          )}
        </button>
      </div>
      <div className="panes">
        <div
          className={`pane-wrap ${activeTab === 'chat' ? 'active' : 'hidden'}`}
        >
          <Chat
            messages={messages}
            myNick={myNick}
            onSend={onSend}
            onAttach={onAttach}
          />
        </div>
        <div
          className={`pane-wrap ${activeTab === 'claude' ? 'active' : 'hidden'}`}
        >
          <Claude
            events={claudeEvents}
            state={claudeState}
            onPrompt={onPrompt}
            onInterrupt={onInterrupt}
            onResetSession={onResetSession}
          />
        </div>
      </div>
    </React.Fragment>
  );
}

const container = document.getElementById('root');
if (!container) throw new Error('webview root element missing');
createRoot(container).render(<App />);
