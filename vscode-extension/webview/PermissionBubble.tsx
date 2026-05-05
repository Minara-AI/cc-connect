// Inline approval prompt rendered when the user is in the
// `default` permission mode and Claude attempts a tool call. Three
// buttons: Allow (one-shot), Deny (one-shot, interrupts the turn),
// Always allow (echoes the SDK's `ctx.suggestions` back as a
// session-scoped rule so the same tool/input shape won't prompt
// again).
//
// Mounted inside the Claude log via `BlockRow` when the
// `kind: 'permission'` block lands. Stays read-only after resolution
// (status flips to 'allowed' / 'denied' / 'always-allowed').

import * as React from 'react';
import type { ClaudeBlock } from './processClaude';

type PermissionBlock = Extract<ClaudeBlock, { kind: 'permission' }>;

export function PermissionBubble({
  block,
  onRespond,
}: {
  block: PermissionBlock;
  onRespond?: (
    requestId: string,
    behavior: 'allow' | 'deny' | 'always-allow',
  ) => void;
}): React.ReactElement {
  const headline =
    block.title ??
    `Claude wants to use ${block.toolName}${
      block.summary ? ` · ${block.summary}` : ''
    }`;
  const settled = block.status !== 'pending';
  const settledLabel =
    block.status === 'allowed'
      ? 'allowed'
      : block.status === 'always-allowed'
        ? 'always allowed'
        : block.status === 'denied'
          ? 'denied'
          : '';
  const tsLabel = block.ts
    ? new Date(block.ts).toTimeString().slice(0, 5)
    : '';
  return (
    <div className={`permission-bubble permission-${block.status}`}>
      <div className="permission-bubble-head">
        <i className="codicon codicon-shield" />
        <span className="permission-bubble-title">{headline}</span>
        {tsLabel && <span className="permission-bubble-ts">{tsLabel}</span>}
        {settled && (
          <span className="permission-bubble-state">{settledLabel}</span>
        )}
      </div>
      {block.description && (
        <div className="permission-bubble-desc">{block.description}</div>
      )}
      {block.blockedPath && (
        <div className="permission-bubble-meta">
          <span>blocked path:</span>
          <code>{block.blockedPath}</code>
        </div>
      )}
      {block.decisionReason && (
        <div className="permission-bubble-meta">
          <span>reason:</span>
          <code>{block.decisionReason}</code>
        </div>
      )}
      {block.summary && !block.title && (
        <div className="permission-bubble-summary">{block.summary}</div>
      )}
      {!settled && onRespond && (
        <div className="permission-bubble-actions">
          <button
            type="button"
            className="permission-btn permission-btn-deny"
            onClick={() => onRespond(block.requestId, 'deny')}
          >
            <i className="codicon codicon-circle-slash" />
            <span>Deny</span>
          </button>
          {block.canAlwaysAllow && (
            <button
              type="button"
              className="permission-btn permission-btn-always"
              onClick={() => onRespond(block.requestId, 'always-allow')}
              title="Add an SDK-suggested rule so this tool/input shape doesn't prompt again this session."
            >
              <i className="codicon codicon-shield" />
              <span>Always allow</span>
            </button>
          )}
          <button
            type="button"
            className="permission-btn permission-btn-allow"
            onClick={() => onRespond(block.requestId, 'allow')}
          >
            <i className="codicon codicon-check" />
            <span>Allow</span>
          </button>
        </div>
      )}
    </div>
  );
}
