// Tool-call rendering for the Claude pane. Two stacked components:
//
// * `ToolCard` — header (per-tool codicon + name + pending indicator)
//   over an IN/OUT split. The IN row shows a one-line input summary;
//   the OUT row delegates to `ToolOutputView` for tool-specific
//   rendering (diff for Edit/Write/MultiEdit, ANSI-friendly preformat
//   for Bash, generic expandable text for everything else).
//
// * Helpers `shortenToolName` / `shortenHookName` are also used by
//   `Claude.tsx::BlockRow` for hook rows; they're exported here.

import * as React from 'react';
import type { ClaudeBlock } from './processClaude';
import {
  BashResultView,
  EditDiffView,
  ExpandableText,
} from './ToolCardBody';

type ToolBlock = Extract<ClaudeBlock, { kind: 'tool' }>;

export function ToolCard({
  block,
}: {
  block: ToolBlock;
}): React.ReactElement {
  const cls = block.result?.isError
    ? 'claude-tool-card claude-tool-error'
    : 'claude-tool-card';
  const short = shortenToolName(block.name);
  const isEdit = short === 'Edit' || short === 'Write' || short === 'MultiEdit';
  const isBash = short === 'Bash';
  const inputSummary = summarizeInput(block.name, block.input);
  const icon = iconForTool(short);
  return (
    <div className={cls}>
      <div className="claude-tool-head">
        <i className={`codicon codicon-${icon}`} />
        <span className="claude-tool-name">{short}</span>
        {!block.result && <span className="claude-tool-pending">⏳</span>}
      </div>
      <div className="claude-tool-block claude-tool-in">
        <span className="claude-tool-label">IN</span>
        <span className="claude-tool-input">{inputSummary || '(no args)'}</span>
      </div>
      <div className="claude-tool-block claude-tool-out">
        <span className="claude-tool-label">OUT</span>
        <div className="claude-tool-out-body">
          <ToolOutputView block={block} isEdit={isEdit} isBash={isBash} />
        </div>
      </div>
    </div>
  );
}

function ToolOutputView({
  block,
  isEdit,
  isBash,
}: {
  block: ToolBlock;
  isEdit: boolean;
  isBash: boolean;
}): React.ReactElement {
  if (isEdit) {
    return <EditDiffView input={block.input} />;
  }
  if (!block.result) {
    return <span className="claude-tool-pending-out">running…</span>;
  }
  const { fullText, isError } = block.result;
  if (!fullText) {
    return <span className="claude-tool-empty">(empty)</span>;
  }
  if (isBash) {
    return <BashResultView text={fullText} isError={isError} />;
  }
  return <ExpandableText text={fullText} isError={isError} />;
}

/** Pick a VSCode codicon for a given tool name so the tool card head
 *  reads at a glance (file = read/edit/write, terminal = bash, etc.).
 *  Anything unrecognised gets the generic "tools" icon. */
function iconForTool(short: string): string {
  switch (short) {
    case 'Read':
      return 'file-code';
    case 'Edit':
    case 'MultiEdit':
      return 'edit';
    case 'Write':
      return 'new-file';
    case 'Bash':
    case 'BashOutput':
      return 'terminal';
    case 'Grep':
      return 'search';
    case 'Glob':
      return 'file-submodule';
    case 'WebFetch':
    case 'WebSearch':
      return 'globe';
    case 'TodoWrite':
      return 'checklist';
    case 'Task':
    case 'Agent':
      return 'rocket';
    case 'cc_send':
    case 'cc_at':
      return 'comment';
    case 'cc_drop':
      return 'cloud-upload';
    case 'cc_recent':
    case 'cc_list_files':
      return 'list-unordered';
    case 'cc_wait_for_mention':
      return 'bell';
    case 'cc_save_summary':
      return 'note';
    default:
      return 'tools';
  }
}

/** Strip the `mcp__<server>__` prefix that the SDK adds to MCP tools so
 *  cards show `cc_send` instead of `mcp__cc-connect__cc_send`. */
export function shortenToolName(name: string): string {
  const m = /^mcp__[^_]+(?:[^_]|_[^_])*?__(.+)$/.exec(name);
  return m ? m[1] : name;
}

/** Compose a hook label like `PreToolUse · Read` from `<phase>:<tool>`,
 *  reusing `shortenToolName` so MCP-tool hooks read consistently. */
export function shortenHookName(name: string): string {
  const colon = name.indexOf(':');
  if (colon < 0) return name;
  const phase = name.slice(0, colon);
  const tool = shortenToolName(name.slice(colon + 1));
  return `${phase} · ${tool}`;
}

function summarizeInput(name: string, input: Record<string, unknown>): string {
  const short = shortenToolName(name);
  const candidates: Record<string, string[]> = {
    Read: ['file_path', 'path'],
    Edit: ['file_path', 'path'],
    Write: ['file_path', 'path'],
    Bash: ['command'],
    Grep: ['pattern'],
    Glob: ['pattern'],
    cc_send: ['body'],
    cc_at: ['nick', 'body'],
    cc_drop: ['path'],
    cc_recent: ['limit'],
    cc_save_summary: ['body'],
    cc_wait_for_mention: ['timeout_seconds'],
  };
  const keys = candidates[short] ?? Object.keys(input);
  const parts: string[] = [];
  for (const k of keys.slice(0, 2)) {
    const v = input[k];
    if (v === undefined) continue;
    const s = typeof v === 'string' ? v : JSON.stringify(v);
    parts.push(s.length > 60 ? s.slice(0, 57) + '…' : s);
  }
  return parts.join(' · ');
}
