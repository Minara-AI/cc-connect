import * as React from 'react';

export interface ClaudeRunnerState {
  busy: boolean;
  queued: number;
}

interface ClaudeProps {
  events: unknown[];
  state: ClaudeRunnerState;
}

export function Claude({ events, state }: ClaudeProps): React.ReactElement {
  const busyLabel = state.busy
    ? state.queued > 0
      ? `· busy (${state.queued} queued)`
      : '· busy'
    : '';
  return (
    <div className="pane">
      <h2>
        claude {busyLabel && <span className="pane-busy">{busyLabel}</span>}
      </h2>
      <div className="claude-log">
        {events.length === 0 ? (
          <div className="muted">(idle — @-mention me from chat to start)</div>
        ) : (
          events.map((e, i) => <ClaudeRow key={i} event={e} />)
        )}
      </div>
    </div>
  );
}

function ClaudeRow({ event }: { event: unknown }): React.ReactElement | null {
  const ev = event as {
    type?: string;
    subtype?: string;
    session_id?: string;
    hook_name?: string;
    error?: string;
    num_turns?: number;
    total_cost_usd?: number;
    message?: { content?: ContentBlock[] };
  };
  const t = ev.type;
  const sub = ev.subtype;

  if (t === 'system' && sub === 'init') {
    const sid = (ev.session_id ?? '').slice(0, 8);
    return (
      <div className="claude-row claude-system">▸ session start ({sid}…)</div>
    );
  }
  if (t === 'system' && (sub === 'hook_started' || sub === 'hook_response')) {
    return (
      <div className="claude-row claude-hook">
        hook · {ev.hook_name ?? '?'} · {sub}
      </div>
    );
  }
  if (t === 'assistant') {
    const blocks = ev.message?.content ?? [];
    return (
      <React.Fragment>
        {blocks.map((b, j) => renderBlock(b, j))}
      </React.Fragment>
    );
  }
  if (t === 'result') {
    const turns = ev.num_turns ?? 0;
    const cost =
      typeof ev.total_cost_usd === 'number'
        ? ` · $${ev.total_cost_usd.toFixed(3)}`
        : '';
    return (
      <div className="claude-row claude-result">
        ✓ done ({turns} turn{turns === 1 ? '' : 's'}
        {cost})
      </div>
    );
  }
  if (t === 'sdk:error') {
    return (
      <div className="claude-row claude-error">✗ {ev.error ?? 'sdk error'}</div>
    );
  }
  // Other event types (user/tool_result/rate_limit_event/etc.) — small.
  return (
    <div className="claude-row claude-other">
      [{t}
      {sub ? `:${sub}` : ''}]
    </div>
  );
}

interface ContentBlock {
  type?: string;
  text?: string;
  name?: string;
  input?: unknown;
}

function renderBlock(
  b: ContentBlock,
  key: number,
): React.ReactElement | null {
  if (b.type === 'text' && typeof b.text === 'string') {
    return (
      <div key={key} className="claude-row claude-text">
        {b.text}
      </div>
    );
  }
  if (b.type === 'tool_use') {
    const summary = JSON.stringify(b.input ?? {}).slice(0, 80);
    return (
      <div key={key} className="claude-row claude-tool">
        ▸ {b.name ?? 'tool'}({summary})
      </div>
    );
  }
  return null;
}
