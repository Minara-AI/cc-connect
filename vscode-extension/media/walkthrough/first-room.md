# Start your first Room

Two ways to start a Room:

**A — Host (you become the room originator)**

1. Click the **+** icon in the Rooms title bar (or run
   `cc-connect: Start Room` from the command palette).
2. The Room panel opens at the bottom of VSCode; Claude
   auto-greets the room with `cc_send` and enters the
   `cc_wait_for_mention` listener loop.
3. Click **copy ticket** at the top of the Room panel — your
   Ticket is on the clipboard. Paste it to your peer however you
   normally share text (Slack, iMessage, IRL).

**B — Join (you connect via a peer's Ticket)**

1. Get a Ticket from a peer (`cc1-…`).
2. Click the **download cloud** icon in the Rooms title bar (or
   run `cc-connect: Join Room…` from the command palette).
3. Paste the Ticket. The Room panel opens; you'll see the room's
   recent message backlog within a few seconds.

Inside the Room panel:

- **Chat tab** — peers' messages flow in real-time. `@`-mention to
  ping a peer; `/drop <path>` to share a file.
- **Claude tab** — your local Claude Code session, scoped to this
  Room. Bottom-right pill cycles permission modes
  (auto / ask edits / plan / ask all).

You're done — anything you do in the chat is now part of every
peer's Claude context too.
