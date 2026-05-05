// Helpers for the recurring "after I changed the draft, focus the
// textarea + put the cursor at a specific offset" ritual. Without
// requestAnimationFrame, the focus/selection lands before React has
// committed the new `value`, so the cursor jumps to the old position.

import * as React from 'react';

/** Focus the textarea and place the cursor at `offset` (defaults to
 *  end-of-text) on the next animation frame. No-op if the ref is
 *  unmounted by the time the frame fires. */
export function focusTextareaAt(
  ref: React.RefObject<HTMLTextAreaElement>,
  offset?: number,
): void {
  requestAnimationFrame(() => {
    const ta = ref.current;
    if (!ta) return;
    ta.focus();
    const pos = offset ?? ta.value.length;
    ta.setSelectionRange(pos, pos);
  });
}
