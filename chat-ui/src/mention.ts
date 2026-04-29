// @-mention completion logic. Direct port of
// crates/cc-connect-tui/src/mention.rs — keep them in sync.
//
// Pure functions; the popup state machine lives in the InputBox
// component.

/** Special broadcast tokens. Hook (`hook_format::mentions_self`) treats
 * these as "everyone listening" — keep this list in sync with
 * `cc-connect-core/src/hook_format.rs::mentions_self`. */
const BROADCAST_TOKENS = ["cc", "claude", "all", "here"] as const;

/** If `input` ends with an in-progress `@<prefix>` (no whitespace after
 *  the last `@`), return the prefix without the `@`. Returns `null` if
 *  the cursor is not in an at-token. The prefix may be `""` when the
 *  user has just typed `@`. */
export function currentAtToken(input: string): string | null {
  const at = input.lastIndexOf("@");
  if (at < 0) return null;
  const after = input.slice(at + 1);
  if (/\s/.test(after)) return null;
  // Avoid email-like patterns ("foo@bar"): if the char before `@` is
  // alphanumeric, it's not a mention.
  if (at > 0) {
    const prev = input[at - 1]!;
    if (/[A-Za-z0-9]/.test(prev)) return null;
  }
  return after;
}

/** Filter `recent` (most-recent-first) by case-insensitive `startsWith`,
 *  excluding `selfNick`, then synthesise the user's own AI peer
 *  (`<selfNick>-cc`, never lands in `recent` because the listener
 *  filters own-pubkey messages) and finally append the broadcast tokens. */
export function mentionCandidates(
  recent: readonly string[],
  prefix: string,
  selfNick: string | null
): string[] {
  const lower = prefix.toLowerCase();
  const selfLower = selfNick && selfNick.length > 0 ? selfNick.toLowerCase() : null;

  const out: string[] = [];
  const seen = new Set<string>();

  // Synthetic own-AI peer up front (matches the Rust ordering).
  if (selfNick && selfNick.length > 0) {
    const ownAi = `${selfNick}-cc`;
    const ownLower = ownAi.toLowerCase();
    if (ownLower.startsWith(lower) && !seen.has(ownLower)) {
      out.push(ownAi);
      seen.add(ownLower);
    }
  }

  for (const nick of recent) {
    const nLower = nick.toLowerCase();
    if (selfLower !== null && nLower === selfLower) continue;
    if (!nLower.startsWith(lower)) continue;
    if (!seen.has(nLower)) {
      out.push(nick);
      seen.add(nLower);
    }
  }

  for (const tok of BROADCAST_TOKENS) {
    if (!tok.startsWith(lower)) continue;
    const lt = tok.toLowerCase();
    if (!seen.has(lt)) {
      out.push(tok);
      seen.add(lt);
    }
  }
  return out;
}

/** Replace the trailing `@<prefix>` in `input` with `@<full> ` (with a
 *  trailing space so the user can keep typing the body). */
export function completeAt(input: string, fullNick: string): string {
  const at = input.lastIndexOf("@");
  if (at < 0) return input;
  return input.slice(0, at + 1) + fullNick + " ";
}

/** Body-content scan for @-mentions of the receiving user. Mirrors
 *  `cc-connect-core::hook_format::mentions_self`. The match is
 *  word-boundary against nick-continuation chars (`[A-Za-z0-9_-]`), so
 *  `@alice-cc hi` does NOT register as a mention of `alice`. Keep this
 *  and the Rust side in sync — see the `mentions_self_respects_word_boundary`
 *  test in hook_format.rs. */
export function bodyMentionsSelf(body: string, selfNick: string | null): boolean {
  const lower = body.toLowerCase();
  for (const tok of BROADCAST_TOKENS) {
    if (matchAtToken(lower, tok)) return true;
  }
  if (selfNick && selfNick.length > 0) {
    if (matchAtToken(lower, selfNick.toLowerCase())) return true;
  }
  // Note: deliberately NOT matching the AI mirror form `<selfNick>-cc`
  // here. The Rust `mentions_self` does match it (so the for-you tag
  // and `cc_wait_for_mention` wake the local claude), but for the
  // chat-ui's `(@me)` red highlight the user's mental model is
  // "@<self>-cc is addressed to a different entity than me."
  // Highlighting your own outgoing `@<self>-cc` message as `(@me)` is
  // jarring and incorrect from the user's perspective.
  return false;
}

/** Returns true iff `lower` contains `@<target>` where the character
 *  immediately after the token is end-of-string or NOT a
 *  nick-continuation character. `lower` and `target` MUST already be
 *  lowercase. */
function matchAtToken(lower: string, target: string): boolean {
  const needle = `@${target}`;
  let from = 0;
  while (true) {
    const i = lower.indexOf(needle, from);
    if (i < 0) return false;
    const next = lower.charAt(i + needle.length);
    if (next === "" || !isNickCont(next)) return true;
    from = i + 1;
  }
}

function isNickCont(c: string): boolean {
  return /[A-Za-z0-9_-]/.test(c);
}
