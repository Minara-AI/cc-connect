import { useEffect, useRef } from 'react';

/** Auto-scrolls a scrollable element to the bottom when its content
 *  changes — but only while the user is already near the bottom. If
 *  the user has scrolled up to read backlog, new content arrives
 *  silently (no yank). Pattern matches Slack / Discord scrollback.
 *
 *  Pass a primitive that changes on every new item (typically the
 *  array length). Returns a ref to attach to the scroll container. */
export function useStickyScroll(
  dep: number,
): React.RefObject<HTMLDivElement> {
  const ref = useRef<HTMLDivElement>(null!);
  const stickyRef = useRef(true);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onScroll = (): void => {
      const dist = el.scrollHeight - el.scrollTop - el.clientHeight;
      stickyRef.current = dist < 16;
    };
    el.addEventListener('scroll', onScroll);
    return () => el.removeEventListener('scroll', onScroll);
  }, []);

  useEffect(() => {
    if (!stickyRef.current) return;
    const el = ref.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }, [dep]);

  return ref;
}
