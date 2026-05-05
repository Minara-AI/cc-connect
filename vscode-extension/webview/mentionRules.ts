// Single source of truth for `@<token>` parsing on the webview side.
// All four sites that care about mentions should import from here:
//   - highlightMentions.tsx   (chat row rendering)
//   - mentionAutocomplete.ts  (popup state)
//   - Any future hover / preview component
//
// The host has its own copy in `src/host/mention.ts` — the two TS
// configs (rootDir webview/ vs src/) prevent cross-import. If you
// change anything here, mirror it there. The host file links back
// for confirmation.

/** Tokens that address every AI in the room at once. */
export const BROADCAST_TOKENS: ReadonlySet<string> = new Set([
  'cc',
  'claude',
  'all',
  'here',
]);

/** Single-character class for the "still inside the nick token" check.
 *  `@yjj-cc` boundary check uses this: a `c` after `@yjj-cc` would
 *  extend the token, but a space / punctuation ends it. */
export const NICK_CHARS = /[A-Za-z0-9_-]/;

/** Global match for `@<word>` tokens. Word characters are `[A-Za-z0-9_-]`,
 *  matching the host's `isNickCont`. Use `new RegExp(MENTION_RE.source,
 *  'g')` per call to avoid lastIndex sharing. */
export const MENTION_RE: RegExp = /@[A-Za-z0-9_-]+/g;
