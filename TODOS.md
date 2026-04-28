# TODOS

## WORKAROUND ACTIVE: vendored ed25519 + ed25519-dalek (pending upstream PR)

**Status:** Resolved locally via `[patch.crates-io]` pointing at vendored copies of `ed25519 3.0.0-rc.4` and `ed25519-dalek 3.0.0-pre.1` with three+two-line fixes. cc-connect-core, cc-connect bin (`host` / `doctor` / placeholder `chat`), and cc-connect-hook bin all build and test on stable Rust 1.95.

**Root cause** (for the historical record): `pkcs8::Error::KeyMalformed` was changed from a unit variant to a tuple variant `KeyMalformed(KeyError)`. Both `ed25519 3.0.0-rc.4` and `ed25519-dalek 3.0.0-pre.1` still reference it as a unit variant. Bare `Error::KeyMalformed` is therefore a `fn(KeyError) -> Error` function pointer, not an `Error` value, so `?` and `return Err(...)` both fail to type-check.

**Local fix in this repo:**
- `vendored/ed25519/src/pkcs8.rs` — three sites (lines 172, 173, 179) updated to `Error::KeyMalformed(KeyError::Invalid)` plus a `KeyError` import.
- `vendored/ed25519-dalek/src/signing.rs` — two sites (lines 714, 717) updated similarly.
- Workspace `Cargo.toml`: `[patch.crates-io] ed25519 = { path = "vendored/ed25519" }`, `ed25519-dalek = { path = "vendored/ed25519-dalek" }`.

**Upstream PRs / issues filed (2026-04-28):**
- `n0-computer/iroh#4192` — comment with full root-cause + workaround diff.
- `RustCrypto/signatures#1315` — root-cause issue. (Note: `ed25519` master is *already* fixed in RustCrypto/signatures master, just unreleased; ed25519-dalek is the only crate still carrying the bug.)
- `dalek-cryptography/curve25519-dalek#901` — PR with the actual semantic fix to `ed25519-dalek/src/signing.rs`.

**Removal trigger:** when PR#901 merges and a new `ed25519-dalek` ships against an `ed25519` that re-exports `KeyError`, iroh's `=3.0.0-pre.x` pin can be bumped. At that point:
1. `cargo update` to pull the released ed25519-dalek.
2. Delete `vendored/ed25519/` and `vendored/ed25519-dalek/`.
3. Remove the two `[patch.crates-io]` entries for them in workspace `Cargo.toml`.
4. Verify `cargo test --workspace` and `cc-connect host` still work.
5. Commit "chore: drop vendored ed25519 patches now that upstream is unblocked."

---

## v0.1 implementation

### Bootstrap race UX

**What:** `cc-connect chat <ticket>` must not accept user input until gossip is joined and Backfill is complete (or has timed out).

**Why:** If the user types a question to Claude before bootstrap finishes, the Hook fires with no active Room and silently injects nothing. The user blames cc-connect.

**Pros:** Eliminates the most embarrassing first-run failure mode. Trivial: print `Connecting…` then `Joined.` and gate the readline on a ready-flag.

**Cons:** None.

**Context:** Identified during /plan-eng-review (Section 1, Architecture observation #1). Tied to ADR-0003 (PID file is written at end of bootstrap) but the user-facing message is missing.

**Depends on:** none.

---

## v0.2

### Skill packaging investigation

**What:** Spike a Claude Code skill named `/cc-connect-join` that auto-installs the hook entry into `~/.claude/settings.json` and starts a `cc-connect chat` process in the user's tmux pane.

**Why:** Manual settings.json snippet is the v0.1 install UX. It's brittle (path issues, JSON merge errors). A skill is one slash command and done.

**Pros:** Clean install. Demonstrates cc-connect as a Claude Code-native artifact, not just an external CLI.

**Cons:** Skill ecosystem is still moving in 2026; v0.2 timing matters. Adding a skill adds the v0.1 → v0.2 surface area.

**Context:** Original design doc Open Question 6.

**Depends on:** v0.1 ships first.

---

### Cursor format extension: byte_offset for log scan

**What:** Cursor file stores `{ulid, byte_offset}` instead of just `ulid`. Hook seeks to byte_offset, verifies the next record's ULID matches, then continues forward.

**Why:** As log.jsonl grows (heavy daily users could see 100k+ messages over months), reading "since cursor" via linear scan becomes O(N). With byte offset, it's O(unread).

**Pros:** Performance ceiling raised. ~20 lines of code. Cursor format change is additive (clients that ignore byte_offset still work).

**Cons:** Small protocol surface increase.

**Context:** Performance review Section 4. Explicitly deferred from v0.1 to keep scope tight; flag for inclusion in v0.2 (or v0.1 if scope room appears).

**Depends on:** PROTOCOL.md cursor section.
