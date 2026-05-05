// Detect file paths inside a free-form string and split it into
// alternating text / path tokens so the renderer can swap each path
// for a clickable chip.
//
// Heuristic — three permissive shapes, ranked by specificity:
//   1. Anchored: `./foo`, `../foo`, `~/foo`, `/Users/...` (absolute)
//   2. Multi-segment: `foo/bar/baz.ts` (≥1 slash, optional extension)
//   3. Single-segment with known extension: `README.md`, `package.json`
//
// We deliberately avoid matching bare words; the user can always
// retype with a slash or extension if they want a chip.

export interface FileToken {
  kind: 'text' | 'path';
  value: string;
}

const KNOWN_EXTS =
  'ts|tsx|js|jsx|mjs|cjs|json|md|mdx|rs|toml|yaml|yml|kdl|lock|py|go|sh|bash|zsh|fish|css|scss|sass|less|html|svg|png|jpg|jpeg|gif|webp|ico|txt|env|sock|jsonl|log|sql|proto|graphql|gql';

// Order of alternatives matters — anchored paths must win over the
// multi-segment fallback, otherwise `~/foo/bar.ts` would match only
// from `foo/bar.ts` onward.
const FILE_RE = new RegExp(
  [
    // anchored absolute / home / relative
    `(?:\\.\\.?\\/|~\\/|\\/)[\\w@./+\\-]+`,
    // multi-segment relative (no leading slash)
    `[\\w@\\-]+(?:\\/[\\w@.+\\-]+)+`,
    // single-segment with known extension
    `\\b[\\w@\\-]+\\.(?:${KNOWN_EXTS})\\b`,
  ].join('|'),
  'g',
);

export function splitForFileRefs(text: string): FileToken[] {
  const out: FileToken[] = [];
  let last = 0;
  for (const m of text.matchAll(FILE_RE)) {
    const start = m.index ?? 0;
    if (start > last) {
      out.push({ kind: 'text', value: text.slice(last, start) });
    }
    out.push({ kind: 'path', value: m[0] });
    last = start + m[0].length;
  }
  if (last < text.length) {
    out.push({ kind: 'text', value: text.slice(last) });
  }
  if (out.length === 0) {
    out.push({ kind: 'text', value: text });
  }
  return out;
}

export function fileBasename(path: string): string {
  const i = path.lastIndexOf('/');
  return i < 0 ? path : path.slice(i + 1);
}
