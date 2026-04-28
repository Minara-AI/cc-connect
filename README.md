# cc-connect

A peer-to-peer protocol that lets multiple Claude Code instances share the same chat-and-files context. Each developer keeps their own Claude. The shared substrate (chat history, files) lives over P2P (`iroh-gossip`); each Claude reads from its local replica via a `UserPromptSubmit` hook.

The big idea: don't multiplex one Claude across humans, multiplex shared context across Claudes.

> v0.1 status: feature-complete in commits, 76 tests passing, full protocol drafted in [`PROTOCOL.md`](./PROTOCOL.md). Vendored ed25519 patches block crates.io publish until upstream releases an `ed25519-dalek` against fixed `pkcs8` (see [`TODOS.md`](./TODOS.md)).

---

## How the magic moment works

```
┌──────── Alice's machine ────────┐         ┌──────── Bob's machine ────────┐
│                                  │         │                                │
│  tmux pane L:  $ claude          │         │  tmux pane L:  $ claude        │
│  tmux pane R:  $ cc-connect chat │ gossip  │  tmux pane R:  $ cc-connect    │
│                ── REPL ──        │ ──────► │                  chat <ticket> │
│                                  │ ◄────── │                                │
│  Alice asks her Claude:          │         │  Bob types in his chat REPL:   │
│  "Redis or Postgres?"            │         │  "postgres, we have it"        │
│                                  │         │                                │
│  Hook fires on Alice's next      │         │                                │
│  prompt → injects Bob's message  │         │                                │
│  into Alice's Claude context     │         │                                │
│  Alice's Claude: "going Postgres │         │                                │
│  per the chat"                   │         │                                │
└──────────────────────────────────┘         └────────────────────────────────┘
```

Bob never typed anything special. Alice never copy-pasted anything. The hook reads Bob's messages from a locally-replicated `log.jsonl` and prepends them to Alice's prompt.

Full architecture: [`PROTOCOL.md`](./PROTOCOL.md). Decision rationale: [`docs/adr/`](./docs/adr/).

---

## Setup (per machine)

You need: macOS or Linux, Rust ≥ 1.85, a working Claude Code install.

### 1. Build

```bash
git clone git@github.com:Minara-AI/cc-connect.git
cd cc-connect
cargo build --workspace --release
```

The first build pulls the iroh stack and patched-vendored `ed25519` / `ed25519-dalek` (see `vendored/`); takes ~5-10 minutes.

### 2. Install the hook into Claude Code

cc-connect bridges to Claude Code via a `UserPromptSubmit` hook. Edit `~/.claude/settings.json` and add (merge with any existing `hooks` block):

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "type": "command",
        "command": "/absolute/path/to/cc-connect/target/release/cc-connect-hook"
      }
    ]
  }
}
```

Use the **absolute path** — `cc-connect-hook` will silently fail to inject if Claude Code's `PATH` doesn't include the binary's location. Restart Claude Code after editing.

### 3. Verify

```bash
./target/release/cc-connect doctor
```

Should report `[OK]` for the hook entry, `[--]` (info: not yet created) for the identity key and active-rooms dir, and ideally no `[FAIL]` lines.

---

## Usage

### Host a room

```bash
$ ./target/release/cc-connect host

Room hosted. Share this code out-of-band:

    cc1-vxnqrtpgwvmjxd42zcnajikrl6dmbd4hamdj4twg…

Joiners run:  cc-connect chat <room-code>

Press Ctrl-C to close the room.
```

`host` stays online so joiners have a peer to dial. Share the `cc1-…` code via Slack / paper / whatever.

### Join a room

In a *separate* terminal pane:

```bash
$ ./target/release/cc-connect chat 'cc1-vxnqrtpgwvmjxd42zcnajikrl6dmbd4hamdj4twg…'

Joined room: a1b2c3d4e5f6 (peers: 1)
You are:     hnvcppgow2sc2yvd
[chatroom] (backfilled 7 messages from peer)
Type to send. Ctrl-C / EOF to leave.
```

Type messages. Press enter to send.

### Drop a file (v0.2-alpha)

```
> /drop ./design.svg
[chat] dropped design.svg (148 bytes)
```

`/drop <path>` reads the file inline, broadcasts it via gossip, and saves it locally on every peer's machine. Both peers' Claudes see it as `@file:<path>` on the next prompt — Claude Code reads it via its native file-attach convention.

**v0.2-alpha limit: 32 KB binary** (small images, code snippets, markdown). Larger files will land in v0.2.1 via `iroh-blobs`.

### What Claude sees

While `cc-connect chat …` is running, every prompt you send to Claude Code in another pane has the recent unread chat lines spliced into Claude's context. Claude doesn't know there's a chat — to it, the lines just look like extra prompt context tagged `[chatroom @nick HH:MMZ] body`.

### Configure your displayed name (optional)

Create `~/.cc-connect/nicknames.json`:

```json
{
  "hnvcppgow2sc2yvdvdicu3ynonsteflxdxrehjr2ybekdc2z3iuq": "alice",
  "k7p8mfx9rsa3jzwh4ab5n6tdgfk2tmvc8eyhbjr1ympd5fnl2quz": "bob"
}
```

Maps Pubkey strings (full 52-char base32) to a human-readable nickname. The mapping is local-only — Bob doesn't see what Alice nicknamed him; each peer maintains their own.

---

## Two-laptop demo procedure

For the real magic-moment test:

1. Both machines: complete Setup steps 1-3 above.
2. Alice (machine A): `cc-connect host` in tmux right pane. Copy the printed `cc1-…` code.
3. Alice: in tmux left pane, `claude` (Claude Code).
4. Bob (machine B): paste the code into `cc-connect chat <code>` in tmux right pane.
5. Bob: in tmux left pane, `claude`.
6. **Bob types into his chat pane**: `try sqlite for now`
7. **Alice asks her Claude something** (anything). On submit, the hook reads Bob's message from Alice's local log and injects it as context. Alice's Claude reply should reference Bob's suggestion.

If it doesn't work, see [Troubleshooting](#troubleshooting).

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `cc-connect host` hangs at "binding endpoint" | Firewall blocks n0's relay servers | Try a different network. Real LAN-only mode is v0.2+. |
| `cc-connect chat` says `Joined room … (peers: 1)` but no messages flow | mDNS is blocked (corporate WiFi client isolation) | Try a coffee-shop / home network. |
| Hook silently does nothing | Settings.json hook path is relative, or binary not on PATH | Use absolute path; restart Claude Code; `cc-connect doctor` |
| Late joiner sees `[chatroom] (joined late, no history available)` | Backfill request to first peer timed out (5 s) | Confirm both peers are reachable; v0.1 doesn't retry across peers, that's a v0.2 polish |
| `cargo build` fails on `ed25519-3.0.0-rc.4` | Missing `[patch.crates-io]` (you cloned without `vendored/`) | Re-clone or `git fetch origin main && git reset --hard origin/main` |
| Identity file mode wrong | Drifted from 0600 | `chmod 600 ~/.cc-connect/identity.key` (doctor warns) |
| `/tmp/cc-connect-$UID/active-rooms/` mode wrong | Loose perms | `rm -rf "$TMPDIR/cc-connect-$UID/" && cc-connect chat …` |

If `cc-connect-hook` fired but you suspect it failed, check `~/.cc-connect/hook.log`. The hook always exits 0 (PROTOCOL §7.4) so error don't propagate to Claude.

---

## Project layout

```
cc-connect/
├── PROTOCOL.md              v0.1 wire-and-disk specification
├── CONTEXT.md               Domain glossary (DDD-style)
├── docs/adr/                Architecture decision records (1-4)
├── crates/
│   ├── cc-connect-core/     Protocol primitives library (62 tests)
│   ├── cc-connect/          host / chat / doctor binary
│   └── cc-connect-hook/     UserPromptSubmit hook binary
├── tests/                   FAKE-CLAUDE-CODE integration test
├── vendored/                Patched ed25519 + ed25519-dalek (temporary,
│                            see TODOS.md and curve25519-dalek#901)
└── spike/                   Spike 0 evidence (hook byte-cap probe)
```

---

## Status / contributing

v0.1 is feature-complete in commits but un-released because of the upstream `ed25519` RC issue. See [`TODOS.md`](./TODOS.md) for the upstream tracker and removal procedure.

Current cadence: protocol-first, every wire-format detail in PROTOCOL.md, tests are byte-exact where it matters.

Issues and PRs welcome on the private repo.
