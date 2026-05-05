import * as React from 'react';
import { BROADCAST_TOKENS, MENTION_RE } from './mentionRules';

/** Splits `text` on @-mentions and wraps each token in a styled <span>.
 *  Returns an array of strings + spans suitable for rendering as
 *  `{...children}`. */
export function highlightMentions(
  text: string,
  myNick: string,
): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  let lastIdx = 0;
  const me = myNick.toLowerCase();
  const meAi = me ? `${me}-cc` : '';

  // Reset state on each call (RegExp.exec with /g maintains lastIndex
  // across the whole RegExp object).
  const re = new RegExp(MENTION_RE.source, 'g');
  let match: RegExpExecArray | null;
  while ((match = re.exec(text)) !== null) {
    const start = match.index;
    if (start > lastIdx) {
      parts.push(text.slice(lastIdx, start));
    }
    const tok = match[0];
    const tokLower = tok.slice(1).toLowerCase();
    const isMeTarget =
      me.length > 0 && (tokLower === me || tokLower === meAi);
    const isBroadcast = BROADCAST_TOKENS.has(tokLower);
    let cls = 'mention';
    if (isMeTarget) cls += ' me';
    else if (isBroadcast) cls += ' broadcast';
    parts.push(
      <span key={`${start}-${tok}`} className={cls}>
        {tok}
      </span>,
    );
    lastIdx = start + tok.length;
  }
  if (lastIdx < text.length) parts.push(text.slice(lastIdx));
  return parts;
}
