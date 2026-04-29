// Mirrors crates/cc-connect-tui/src/mention.rs::tests verbatim (each
// test name + scenario), so the TS port can't drift from the Rust source
// of truth.

import { describe, expect, test } from "bun:test";
import { bodyMentionsSelf, completeAt, currentAtToken, mentionCandidates } from "./mention.ts";

describe("currentAtToken", () => {
  test("at_token_at_end", () => {
    expect(currentAtToken("")).toBeNull();
    expect(currentAtToken("hello")).toBeNull();
    expect(currentAtToken("hello @")).toBe("");
    expect(currentAtToken("hello @ali")).toBe("ali");
    expect(currentAtToken("@bob")).toBe("bob");
  });

  test("at_token_finished_by_space", () => {
    expect(currentAtToken("@alice ")).toBeNull();
    expect(currentAtToken("@alice hi")).toBeNull();
  });

  test("no_email_match", () => {
    expect(currentAtToken("foo@bar")).toBeNull();
  });
});

describe("mentionCandidates", () => {
  test("filter_and_dedupe", () => {
    const got = mentionCandidates(["Alice", "BOB", "alice"], "al", null);
    expect(got).toEqual(["Alice", "all"]);
  });

  test("skip_self_but_keep_self_cc", () => {
    const got = mentionCandidates(["me", "me-cc", "peer"], "", "me");
    expect(got).toEqual(["me-cc", "peer", "cc", "claude", "all", "here"]);
  });

  test("own_ai_synthetic_when_recent_empty", () => {
    const got = mentionCandidates([], "", "me");
    expect(got[0]).toBe("me-cc");
  });

  test("own_ai_respects_prefix", () => {
    expect(mentionCandidates([], "me", "Me")).toEqual(["Me-cc"]);
    expect(mentionCandidates([], "zz", "Me")).not.toContain("Me-cc");
  });

  test("broadcast_tokens_appended", () => {
    expect(mentionCandidates([], "c", null)).toEqual(["cc", "claude"]);
  });
});

describe("completeAt", () => {
  test("replaces_partial", () => {
    expect(completeAt("hello @al", "alice")).toBe("hello @alice ");
  });
});

describe("bodyMentionsSelf", () => {
  test("universal_tokens_fire_without_self_nick", () => {
    expect(bodyMentionsSelf("hey @cc what's up", null)).toBe(true);
    expect(bodyMentionsSelf("HEY @CLAUDE", null)).toBe(true);
    expect(bodyMentionsSelf("@all standup", null)).toBe(true);
    expect(bodyMentionsSelf("ping @here", null)).toBe(true);
    expect(bodyMentionsSelf("plain message", null)).toBe(false);
  });

  test("self_nick_is_case_insensitive_and_required", () => {
    expect(bodyMentionsSelf("hi @alice", "alice")).toBe(true);
    expect(bodyMentionsSelf("hi @ALICE!", "alice")).toBe(true);
    expect(bodyMentionsSelf("hi alice", "alice")).toBe(false);
    expect(bodyMentionsSelf("hi @bob", "alice")).toBe(false);
    expect(bodyMentionsSelf("@alice", null)).toBe(false);
    expect(bodyMentionsSelf("@alice", "")).toBe(false);
  });

  test("respects_word_boundary", () => {
    // The user's reported bug: typing `@yj-cc` on YJ's UI MUST NOT
    // light up `(@me)`. Per the user's mental model, YJ-cc is a
    // different entity from YJ even though they share a machine.
    expect(bodyMentionsSelf("@yj-cc 你好", "yj")).toBe(false);
    // Same body IS a mention of `yj-cc` if that's the perceived self.
    expect(bodyMentionsSelf("@yj-cc 你好", "yj-cc")).toBe(true);
    // Broadcast: `@cc-bot` is NOT a mention of broadcast `cc`.
    expect(bodyMentionsSelf("@cc-bot ping", null)).toBe(false);
    // Broadcast: `@cc!` IS — `!` is not a nick-continuation char.
    expect(bodyMentionsSelf("ping @cc!", null)).toBe(true);
    // End-of-string boundary still counts.
    expect(bodyMentionsSelf("over to @cc", null)).toBe(true);
    // `_` is a nick-continuation char — `@alice_2` is NOT mention of `alice`.
    expect(bodyMentionsSelf("@alice_2 hi", "alice")).toBe(false);
    expect(bodyMentionsSelf("@alice_2 hi", "alice_2")).toBe(true);
    // The Rust `mentions_self` matches `@<nick>-cc` (AI mirror form);
    // the UI deliberately does not — assert that divergence here so a
    // future refactor doesn't quietly re-couple them. See the comment
    // at the bottom of `bodyMentionsSelf` for the rationale.
    expect(bodyMentionsSelf("@bob-cc thing", "bob")).toBe(false);
  });
});
