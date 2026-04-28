//! v0.1 release-gate integration test (PROTOCOL.md §11.4 + design doc).
//!
//! Drives two in-process Peers + a real `cc-connect-hook` subprocess,
//! asserting that a Message published by one Peer arrives in the other
//! Peer's hook stdout.
//!
//! NOT YET IMPLEMENTED — depends on the iroh integration in `cc-connect`
//! (Week 2-3 of the v0.1 plan).

#[test]
#[ignore = "v0.1 release-gate; pending iroh integration (PROTOCOL.md §11.4)"]
fn fake_claude_code_magic_moment_end_to_end() {
    // Outline:
    //   1. tempdir for both peers' ~/.cc-connect/.
    //   2. Spawn Peer A as `cc-connect host`, capture ticket.
    //   3. Spawn Peer B as `cc-connect chat <ticket>`.
    //   4. Wait for B's active-rooms PID file to appear.
    //   5. Send Message via B's chat REPL stdin.
    //   6. Spawn `cc-connect-hook` with a synthetic UserPromptSubmit JSON
    //      on stdin pointing at A's tempdir / a fake session_id.
    //   7. Assert the hook's stdout contains B's Message body, the chat
    //      tag, and the UTC `HH:MM` derived from the Message's ts.
    //   8. Assert A's cursor file advanced to the Message's ULID.
    todo!("Week 2-3 plan: implement after iroh-gossip wiring lands")
}
