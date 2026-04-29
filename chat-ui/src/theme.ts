/**
 * Visual tokens for cc-chat-ui. Keep components free of literal color
 * names — import from here instead. Rationale + don'ts in DESIGN.md.
 */
export const theme = {
  /** Primary text — peer nicks, message bodies. */
  fg: "#fdfcfc",
  /** Timestamps, hints, scrolled-back marker. */
  mute: "#9a9898",
  /** Box border — input box, future containers. */
  border: "#646262",
  /** "You" / active surface — own nick, input prompt, popup border. */
  accent: "#007aff",
  /** Daemon up / send succeeded. */
  success: "#30d158",
  /** Daemon down / `(@me)` prefix / body of a mention / send failed. */
  danger: "#ff3b30",
  /** Daemon-emitted event warnings (rate limit, broadcast failure). */
  warn: "#ff9f0a",
} as const;

export type ThemeColor = (typeof theme)[keyof typeof theme];
