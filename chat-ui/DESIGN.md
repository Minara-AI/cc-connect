# cc-chat-ui design system

Adapted from the OpenCode-inspired DESIGN.md (`https://getdesign.md/opencode.ai/design-md`) to the constraints of a React + Ink TUI panel that lives next to a `claude` PTY in a multiplexer.

## Why this system

A chat panel pinned in a 60-100 column terminal pane has very different constraints from a web design system:

- **Monospace only.** Every character occupies one cell. Hierarchy via size doesn't exist; only weight, color, alignment, and spacing-in-cells.
- **No shadows, no blur, no gradients.** Depth is borders + color shifts, full stop.
- **Limited palette in practice.** We use 24-bit hex, but assume the terminal renders a warm dark background. Choose colors that read against `#1a1a1a`-ish, not against pure white.
- **Read next to claude.** The chat-ui never gets full attention; signal must be loud enough to catch the eye, never so loud it competes with claude's pane during a long render.

OpenCode's "monospace-everywhere, warm dark, three-stage semantic palette, zero shadow, sharp 4px-equivalent edges" maps almost 1:1 to these constraints.

## Tokens

The values live in `src/theme.ts`. This document is the rationale.

| Token     | Hex       | Role                                                                                                                                          |
| --------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `fg`      | `#fdfcfc` | Primary text — peer nicks, message bodies, headline labels.                                                                                   |
| `mute`    | `#9a9898` | Timestamps, hints, scrolled-back marker, secondary metadata.                                                                                  |
| `border`  | `#646262` | Box borders. Always single-weight, often rounded.                                                                                             |
| `accent`  | `#007aff` | The "you" / interactive signal. Own nick, input prompt + cursor, popup border, "share ticket" anchor. Used surgically — never as a fill area. |
| `success` | `#30d158` | Daemon up indicator. Status flash for successful sends.                                                                                       |
| `danger`  | `#ff3b30` | Daemon down. `(@me)` mention prefix and the body of any message that mentions you. Status flash for failures.                                 |
| `warn`    | `#ff9f0a` | Daemon-emitted event warnings (rate-limit, broadcast failure).                                                                                |

Background is the terminal's own background — we never paint a solid fill.

## Type and weight rules

The chat-ui has no font scale (terminals don't). Weight is the only typographic axis:

- **Bold** = "look at this person" — peer nicks only.
- **Regular** = everything else, including your own nick. You don't need to scan for yourself; right-alignment + accent color does that job.
- **Dim** (Ink's `dimColor`) = "you can ignore this on a busy screen" — timestamps, hint lines, scrolled-back marker.

Weight is not used to indicate importance of body content. A mention turns the body `danger` red; that is enough.

## Borders

Ink offers `single`, `double`, `round`, `bold`, etc. We use only two:

- `round` for **interactive containers** — the input box and the @-mention popup. Round corners are the TUI analogue of OpenCode's 4–6px input radius. They signal "you can type here" without ornament.
- (no border) everywhere else. The header sits flush. Chat bubbles never get borders — alignment + nick color is enough.

Border color is `border` for input, `accent` for the popup (because it's an active overlay, not a passive container).

## Layout rhythm

- Padding inside boxes: 1 cell horizontal, 0 vertical.
- One blank cell between nick bracket and body. No padding around timestamps.
- Right-align own messages; left-align peers and events.
- Row spacing: none. Each `[hh:mm] [nick] body` row is a single line. Long bodies wrap on the terminal's normal soft-wrap.

## Don'ts

- Don't introduce a second accent color. The interface uses **one** accent (`accent`) and three semantic states (`success`, `danger`, `warn`); anything else dilutes the system.
- Don't use color named tokens like `"red"` / `"cyan"` in components. Always import from `theme.ts`. ANSI named colors look different in every terminal theme; hex tokens render predictably.
- Don't bold body text, ever. Body color and prefix do the work.
- Don't add box-shadows, double-borders, or ASCII-art ornaments. The aesthetic is "clean monospace, calm warmth."
- Don't widen the @-mention popup beyond candidate width + 4 cells of padding. It's an overlay, not a panel.

## Iteration guide

When tweaking a component:

1. Reach for an existing token first. If you genuinely need a new color role, add it to `theme.ts` and document why here.
2. Run the chat-ui in a real multiplexer pane (zellij or tmux) before judging colors — terminal themes shift everything ~10% and you'll mis-judge contrast in a screenshot.
3. Test with the daemon both up and down. The "daemon down" state is when the user most needs the UI to be readable; don't let red-on-warm-black collapse into mud.
4. Test with a 60-column-wide pane. If anything wraps oddly there, fix it before shipping.
