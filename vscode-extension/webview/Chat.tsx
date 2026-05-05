import * as React from 'react';
import { highlightMentions } from './highlightMentions';
import type { Message } from './types';
import { useStickyScroll } from './useStickyScroll';

interface ChatProps {
  messages: Message[];
  myNick: string;
  onSend?: (body: string) => void;
}

export function Chat({
  messages,
  myNick,
  onSend,
}: ChatProps): React.ReactElement {
  const [draft, setDraft] = React.useState('');
  const scrollRef = useStickyScroll(messages.length);

  const submit = (): void => {
    const trimmed = draft.trim();
    if (!trimmed || !onSend) return;
    onSend(trimmed);
    setDraft('');
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>): void => {
    if (e.key === 'Enter' && !e.shiftKey && !e.metaKey && !e.ctrlKey) {
      e.preventDefault();
      submit();
    }
  };

  return (
    <div className="pane">
      <div className="pane-head">chat</div>
      <div className="chat-log" ref={scrollRef}>
        {messages.length === 0 ? (
          <div className="muted">(no messages yet)</div>
        ) : (
          messages.map((m) => (
            <ChatBubble key={m.id} message={m} myNick={myNick} />
          ))
        )}
      </div>
      {onSend && (
        <div className="pane-input">
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Message — Enter to send · Shift+Enter for newline · /drop <path>"
            rows={1}
          />
        </div>
      )}
    </div>
  );
}

function ChatBubble({
  message,
  myNick,
}: {
  message: Message;
  myNick: string;
}): React.ReactElement {
  const isMe = message.nick === myNick;
  const time = new Date(message.ts).toISOString().slice(11, 16);
  const nick = message.nick ?? 'anon';
  const initial = nick.charAt(0).toUpperCase() || '?';
  const avatarColor = colorForNick(nick);
  return (
    <div className={`chat-bubble ${isMe ? 'me' : 'peer'}`}>
      <div
        className="chat-avatar"
        style={{ background: avatarColor }}
        title={nick}
      >
        {initial}
      </div>
      <div className="chat-content">
        <div className="chat-meta">
          {isMe ? `${time} · ${nick}` : `${nick} · ${time}`}
        </div>
        <div className="chat-text">
          {highlightMentions(message.body, myNick)}
        </div>
      </div>
    </div>
  );
}

/** Cheap deterministic colour from nick — picks one of a small palette
 *  so peers' avatars stay distinguishable but consistent across
 *  sessions. */
function colorForNick(nick: string): string {
  const palette = [
    '#5fa8d3',
    '#6ec07b',
    '#d39f5f',
    '#c46f6f',
    '#a56fc4',
    '#5fc4b9',
    '#d3c45f',
    '#7e88c4',
  ];
  let h = 0;
  for (let i = 0; i < nick.length; i++) {
    h = (h * 31 + nick.charCodeAt(i)) | 0;
  }
  return palette[Math.abs(h) % palette.length];
}
