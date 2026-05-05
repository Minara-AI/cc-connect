// Shared helper for stripping system-context wrappers off a user-side
// prompt before display. Imported by both `processClaude.ts` (live +
// transcript replay) and `src/host/transcripts.ts` (title extraction).
//
// Claude Code injects wrapper tags (`<local-command-caveat>`,
// `<ide_opened_file>`, `<command-name>`, `<system-reminder>`,
// `<local-command-stdout>`, etc.) around the human's actual prompt
// in transcript JSONL. Stripping them surfaces what the user typed
// instead of the system noise.
//
// The 8-level guard prevents pathological inputs (deeply-nested
// auto-injected wrappers) from looping forever; in practice we see
// at most 3 stacked wrappers.

export function stripSystemWrappers(raw: string): string {
  let s = raw.trim();
  for (let i = 0; i < 8; i++) {
    const m = /^<([a-z][a-z0-9_-]*)[^>]*>[\s\S]*?<\/\1>\s*/i.exec(s);
    if (!m) break;
    s = s.slice(m[0].length);
  }
  return s.trim();
}
