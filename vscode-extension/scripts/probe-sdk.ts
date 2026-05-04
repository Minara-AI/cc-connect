/**
 * Probe: verify @anthropic-ai/claude-agent-sdk works in our context.
 *
 * Goal: confirm OAuth subscription is picked up automatically (no API key),
 * the SDK spawns the local `claude` binary, hook lifecycle events arrive in
 * the typed event stream, and we can abort early via AbortController without
 * burning subscription quota.
 *
 * Run with `bun run probe:sdk`. Aborts as soon as the first session_id
 * appears, before any API call lands.
 */

import { homedir } from 'os';
import { join } from 'path';
import { query } from '@anthropic-ai/claude-agent-sdk';

async function main(): Promise<void> {
  const ac = new AbortController();
  const captured: { type: string; subtype?: string }[] = [];
  let sessionId: string | undefined;
  let apiKeySource: string | undefined;

  // Point the SDK at the user's installed `claude` (the one cc-connect's
  // install.sh symlinks into ~/.local/bin/). The SDK normally ships a
  // bundled native binary; we reuse the user's instead, which is also
  // what the production extension will do on macOS GUI launches per
  // design-doc §4.4.
  const claudeBin = join(homedir(), '.local', 'bin', 'claude');

  const q = query({
    prompt: 'reply with the single word OK and nothing else',
    options: {
      abortController: ac,
      includeHookEvents: true,
      pathToClaudeCodeExecutable: claudeBin,
      // Skip user-global hooks/MCP for an isolated probe — same trick we
      // used in the §9 Test 2 capture. Lets us run without polluting
      // the user's normal claude session state.
      extraArgs: { 'setting-sources': 'project' },
    },
  });

  try {
    for await (const msg of q) {
      const m = msg as Record<string, unknown>;
      const t = String(m.type ?? 'unknown');
      const s = m.subtype !== undefined ? String(m.subtype) : undefined;
      captured.push({ type: t, subtype: s });

      if (typeof m.session_id === 'string' && !sessionId) {
        sessionId = m.session_id;
      }
      if (s === 'init' && typeof m.apiKeySource === 'string') {
        apiKeySource = m.apiKeySource;
      }

      // Abort right after the init event lands — that's enough to prove
      // the SDK spawned `claude`, OAuth resolved, and the typed stream
      // is flowing. Aborting before `assistant` event = no API call.
      if (s === 'init') {
        ac.abort();
        break;
      }
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (!/abort/i.test(msg)) {
      console.error('Probe failed:', msg);
      process.exit(1);
    }
  }

  console.log('events seen:');
  for (const e of captured) {
    console.log(`  ${e.type}${e.subtype ? `:${e.subtype}` : ''}`);
  }
  console.log('---');
  console.log(`session_id:    ${sessionId ?? '(none)'}`);
  console.log(`apiKeySource:  ${apiKeySource ?? '(unknown)'}`);
  console.log(`outcome:       ${sessionId ? 'OK — SDK spawned claude, OAuth worked' : 'FAIL — no session_id'}`);
}

main();
