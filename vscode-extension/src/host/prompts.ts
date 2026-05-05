// Reads the cc-connect launcher prompt files (`bootstrap-prompt.md` +
// `auto-reply-prompt.md`) from the extension's `dist/layouts/` so the
// VSCode runner has byte-identical behaviour to the TUI's
// `claude-wrap.sh` path:
//
// - `bootstrap-prompt.md` — fed as the first user prompt of a fresh
//   Session, kicks Claude into "say hello + enter `cc_wait_for_mention`
//   loop"
// - `auto-reply-prompt.md` — appended to Claude's system prompt every
//   turn, teaches Claude that it's running inside a Room and the
//   semantics of `cc_send` / `cc_at` / `cc_drop` / `cc_wait_for_mention`
//
// Both files are the canonical source — TUI imports the same files
// via `include_str!`. The compile step copies them into `dist/layouts/`
// so the extension VSIX carries them.

import * as fs from 'node:fs';
import * as path from 'node:path';

let cached:
  | { bootstrap: string; autoReply: string }
  | undefined;

export function loadLauncherPrompts(extensionPath: string): {
  bootstrap: string;
  autoReply: string;
} {
  if (cached) return cached;
  const dir = path.join(extensionPath, 'dist', 'layouts');
  const bootstrap = readOrEmpty(path.join(dir, 'bootstrap-prompt.md'));
  const autoReply = readOrEmpty(path.join(dir, 'auto-reply-prompt.md'));
  cached = { bootstrap, autoReply };
  return cached;
}

function readOrEmpty(filePath: string): string {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    // Returning empty rather than throwing keeps the runner working in
    // the (broken) install case — Claude just behaves like a vanilla
    // headless query, which matches the pre-fix behaviour.
    return '';
  }
}
