// Read Claude Code's local transcript JSONL files for the current
// workspace, so the Claude pane can list + replay past conversations.
//
// On-disk layout — `~/.claude/projects/<encoded-cwd>/<sessionId>.jsonl`,
// where `<encoded-cwd>` is the absolute workspace path with every `/`
// replaced by `-`. Each session is one JSONL stream of mixed event
// types: `user` / `assistant` / `permission-mode` / `attachment` /
// `file-history-snapshot` / `last-prompt` / `summary`. We pass them
// through to the webview which already knows how to render the
// SDK-shape subset (user/assistant) via `processClaude.ts`.

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

export interface SessionMeta {
  sessionId: string;
  filePath: string;
  mtimeMs: number;
  sizeBytes: number;
  /** First user-authored prompt, used as the conversation title. */
  firstPrompt: string;
  /** Total number of `user` + `assistant` events. Cheap heuristic of length. */
  messageCount: number;
}

const TITLE_LEN = 60;
const MAX_SCAN_LINES = 200;

export function encodeCwd(absCwd: string): string {
  // Claude's encoding is literal `/` → `-`. Trailing slashes get
  // stripped by Claude itself; we follow.
  return absCwd.replace(/\/+$/, '').replace(/\//g, '-');
}

function projectsRoot(): string {
  return path.join(os.homedir(), '.claude', 'projects');
}

export function projectDirFor(cwd: string): string {
  return path.join(projectsRoot(), encodeCwd(cwd));
}

/** List all sessions for the given workspace cwd, newest first.
 *  Returns [] if the project directory doesn't exist (no Claude
 *  history yet). */
export function listSessions(cwd: string): SessionMeta[] {
  const dir = projectDirFor(cwd);
  let names: string[];
  try {
    names = fs.readdirSync(dir);
  } catch {
    return [];
  }
  const out: SessionMeta[] = [];
  for (const name of names) {
    if (!name.endsWith('.jsonl')) continue;
    const fp = path.join(dir, name);
    try {
      const stat = fs.statSync(fp);
      if (!stat.isFile() || stat.size === 0) continue;
      const meta = scanMeta(fp, stat.mtimeMs, stat.size, name.replace(/\.jsonl$/, ''));
      if (meta) out.push(meta);
    } catch {
      // unreadable file — skip
    }
  }
  out.sort((a, b) => b.mtimeMs - a.mtimeMs);
  return out;
}

/** Load every event line from a session file. We don't filter here —
 *  let `processClaude.ts` ignore unknown types as it already does. */
export function loadSession(cwd: string, sessionId: string): unknown[] {
  const fp = path.join(projectDirFor(cwd), `${sessionId}.jsonl`);
  const text = fs.readFileSync(fp, 'utf8');
  const out: unknown[] = [];
  for (const line of text.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      out.push(JSON.parse(trimmed));
    } catch {
      // skip malformed line
    }
  }
  return out;
}

interface RawLine {
  type?: string;
  message?: {
    role?: string;
    content?: string | Array<{ type?: string; text?: string }>;
  };
}

function scanMeta(
  filePath: string,
  mtimeMs: number,
  sizeBytes: number,
  sessionId: string,
): SessionMeta | undefined {
  // Stream the head of the file — first user-prompt is usually in
  // the first ~10 lines and we don't want to load multi-MB transcripts
  // just to get a title.
  const head = readHead(filePath, MAX_SCAN_LINES);
  let firstPrompt = '';
  let messageCount = 0;
  for (const raw of head) {
    let ev: RawLine;
    try {
      ev = JSON.parse(raw) as RawLine;
    } catch {
      continue;
    }
    if (ev.type === 'user' || ev.type === 'assistant') messageCount += 1;
    if (firstPrompt) continue;
    if (ev.type !== 'user' || ev.message?.role !== 'user') continue;
    const c = ev.message.content;
    let candidate = '';
    if (typeof c === 'string') {
      candidate = c;
    } else if (Array.isArray(c)) {
      candidate = c
        .map((b) => (b.type === 'text' ? (b.text ?? '') : ''))
        .filter(Boolean)
        .join(' ');
    }
    const cleaned = stripSystemWrappers(candidate);
    if (cleaned) firstPrompt = cleaned;
  }
  firstPrompt = trimTitle(firstPrompt);
  if (!firstPrompt) firstPrompt = '(no prompt)';
  return { sessionId, filePath, mtimeMs, sizeBytes, firstPrompt, messageCount };
}

/** Strip leading system-context wrappers (`<local-command-caveat>…</…>`,
 *  `<ide_opened_file>…</…>`, `<command-name>…</…>`, `<system-reminder>…
 *  </…>`, etc.) so titles surface what the human actually typed. */
function stripSystemWrappers(raw: string): string {
  let s = raw.trim();
  // Repeatedly strip a leading <tag>…</tag> (or <tag/>) block + any
  // following whitespace until none remain at the head.
  for (let i = 0; i < 8; i++) {
    const m = /^<([a-z][a-z0-9_-]*)[^>]*>[\s\S]*?<\/\1>\s*/i.exec(s);
    if (!m) break;
    s = s.slice(m[0].length);
  }
  return s.trim();
}

function readHead(filePath: string, maxLines: number): string[] {
  // Read up to ~64 KB; for typical first-prompt extraction this is
  // plenty without pulling whole multi-MB sessions.
  const fd = fs.openSync(filePath, 'r');
  try {
    const buf = Buffer.alloc(65_536);
    const bytes = fs.readSync(fd, buf, 0, buf.length, 0);
    const text = buf.subarray(0, bytes).toString('utf8');
    return text.split('\n', maxLines).filter(Boolean);
  } finally {
    fs.closeSync(fd);
  }
}

function trimTitle(s: string): string {
  const flat = s.replace(/\s+/g, ' ').trim();
  if (flat.length <= TITLE_LEN) return flat;
  return flat.slice(0, TITLE_LEN - 1) + '…';
}
