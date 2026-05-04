// @-mention detection — mirrors the Rust
// `cc-connect-core::hook_format::mentions_self` semantics: we want
// `@<self>`, `@<self>-cc`, and broadcast tokens (`cc`, `claude`, `all`,
// `here`) to all wake the embedded Claude.
//
// This is a wider match than chat-ui's `bodyMentionsSelf` (which
// deliberately omits `-cc` for the human-facing `(@me)` highlight). The
// extension's purpose is to fire `query()` whenever the rust hook would
// inject a `for-you` directive, so we follow the rust contract.

const BROADCAST_TOKENS = ['cc', 'claude', 'all', 'here'] as const;

/** Returns true iff `body` mentions the local user (any of the
 *  forms above) with word-boundary semantics — `@alice-cc hi` does
 *  NOT register as a mention of `alice`. */
export function shouldWakeClaude(body: string, myNick: string): boolean {
  if (!myNick || myNick.length === 0) return false;
  const lower = body.toLowerCase();
  const self = myNick.toLowerCase();
  if (matchAtToken(lower, self)) return true;
  if (matchAtToken(lower, `${self}-cc`)) return true;
  for (const tok of BROADCAST_TOKENS) {
    if (matchAtToken(lower, tok)) return true;
  }
  return false;
}

function matchAtToken(lower: string, target: string): boolean {
  const needle = `@${target}`;
  let from = 0;
  for (;;) {
    const i = lower.indexOf(needle, from);
    if (i < 0) return false;
    const next = lower.charAt(i + needle.length);
    if (next === '' || !isNickCont(next)) return true;
    from = i + 1;
  }
}

function isNickCont(c: string): boolean {
  return /[A-Za-z0-9_-]/.test(c);
}
