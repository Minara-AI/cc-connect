// Inline overlay shown above the Claude log when the user clicks
// the history button in the pane head. Lists past Claude
// conversations for this workspace (read from
// ~/.claude/projects/<encoded-cwd>/*.jsonl by the host); clicking
// one replays the transcript into the pane in read-only mode.
//
// Self-contained — owns the SessionMetaLite shape so the host's
// `transcripts.ts` mapping in `RoomPanelProvider.ts::history:list`
// has a single import target.

import * as React from 'react';

export interface SessionMetaLite {
  sessionId: string;
  firstPrompt: string;
  mtimeMs: number;
  messageCount: number;
}

export function HistoryPicker({
  sessions,
  viewing,
  onPick,
  onClose,
}: {
  sessions: SessionMetaLite[];
  viewing?: string;
  onPick: (sessionId: string) => void;
  onClose: () => void;
}): React.ReactElement {
  return (
    <div
      className="history-picker"
      role="dialog"
      aria-label="Past conversations"
    >
      <div className="history-picker-head">
        <span>past conversations</span>
        <button
          type="button"
          className="head-btn"
          onClick={onClose}
          aria-label="Close history"
          title="Close"
        >
          <i className="codicon codicon-close" />
        </button>
      </div>
      <div className="history-list">
        {sessions.length === 0 ? (
          <div className="muted">no past conversations in this workspace</div>
        ) : (
          sessions.map((s) => (
            <button
              key={s.sessionId}
              type="button"
              className={`history-item ${
                s.sessionId === viewing ? 'active' : ''
              }`}
              onClick={() => onPick(s.sessionId)}
              title={s.firstPrompt}
            >
              <div className="history-item-title">{s.firstPrompt}</div>
              <div className="history-item-meta">
                <span>{relativeTime(s.mtimeMs)}</span>
                <span>·</span>
                <span>{s.messageCount} msgs</span>
                <span>·</span>
                <span className="history-item-sid">
                  {s.sessionId.slice(0, 8)}
                </span>
              </div>
            </button>
          ))
        )}
      </div>
    </div>
  );
}

function relativeTime(ms: number): string {
  const delta = Date.now() - ms;
  const sec = Math.max(0, Math.floor(delta / 1000));
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  if (day < 7) return `${day}d ago`;
  // YYYY-MM-DD in local time. (toISOString would give UTC; for the
  // history list a date drift of ±1 day vs. when the user actually
  // ran the session would be confusing.)
  const d = new Date(ms);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day0 = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day0}`;
}
