// Per-Room Claude Agent SDK driver. Each Room view owns one runner.
// `enqueue()` accepts a prompt (typically the body of an @-mention);
// the runner serialises turns through a single in-flight `query()`
// at a time, threading `--session-id` so the cc-connect-hook's
// per-(Room, Session) Cursor advances correctly across calls.
//
// SDK call shape:
//   - first turn: `sessionId: <uuid>` to mint the Session with our UUID
//   - subsequent turns: `resume: <uuid>` to pick up the same Session
//   - `env.CC_CONNECT_ROOM = <topic>` so the hook gates injection
//   - `pathToClaudeCodeExecutable` resolves macOS-GUI launch PATH
//   - `includeHookEvents: true` exposes hook lifecycle events in the
//     stream so the Claude panel can render them
//   - per-turn `AbortController` so `interrupt()` can kill the
//     in-flight turn without tearing down the whole runner
//
// Permission UI and MCP cc-connect server registration land in
// subsequent steps.

import { randomUUID } from 'node:crypto';
import { homedir } from 'node:os';
import { join } from 'node:path';
import {
  query,
  type PermissionMode,
  type Query,
} from '@anthropic-ai/claude-agent-sdk';

export interface ClaudeRunnerState {
  busy: boolean;
  queued: number;
  mode: SupportedPermissionMode;
}

/** Subset we expose in the UI. We intentionally don't surface `default`
 *  (which prompts on every tool call) — under the SDK's headless mode
 *  that path can throw ZodError on some tool calls. `dontAsk` / `auto`
 *  are SDK internals not meant for end-user toggling. */
export type SupportedPermissionMode =
  | 'bypassPermissions'
  | 'acceptEdits'
  | 'plan';

export interface ClaudeRunnerOptions {
  topic: string;
  /** Appended to Claude's system prompt every turn — mirrors the
   *  `--append-system-prompt "$(cat auto-reply-prompt.md)"` flag the
   *  TUI passes through `claude-wrap.sh`. Tells Claude it's in a
   *  cc-connect Room + how to use the cc_* MCP tools. */
  systemPromptAppend?: string;
  /** First user prompt of a fresh Session. The TUI feeds Claude the
   *  contents of `bootstrap-prompt.md` here so it auto-greets the
   *  Room and enters the `cc_wait_for_mention` loop without the user
   *  having to type anything. */
  initialPrompt?: string;
  onEvent: (event: unknown) => void;
  onStateChange: (state: ClaudeRunnerState) => void;
}

export interface ClaudeRunnerHandle {
  /** Enqueue a prompt. Runs after any currently-in-flight turn. */
  enqueue(prompt: string): void;
  /** Cancel the currently-running turn. Queued items still run. */
  interrupt(): void;
  /** Mint a fresh Session: drop queue, abort in-flight, rotate the
   *  sessionId so the next turn starts clean. */
  resetSession(): void;
  /** Switch permission mode. Applies to all subsequent turns; if a
   *  turn is in flight, also calls `query.setPermissionMode(mode)` so
   *  the live conversation flips immediately. */
  setPermissionMode(mode: SupportedPermissionMode): void;
  /** Tear the runner down: cancel current + clear queue. Used on
   *  panel dispose. */
  abort(): void;
}

export function createClaudeRunner(
  opts: ClaudeRunnerOptions,
): ClaudeRunnerHandle {
  let sessionUuid = randomUUID();
  const claudeBin = join(homedir(), '.local', 'bin', 'claude');
  let hasStarted = false;
  const queue: string[] = [];
  let processing = false;
  let panelClosed = false;
  let currentTurnAc: AbortController | null = null;
  // The active query handle, if a turn is in flight. Held so
  // setPermissionMode() can call `currentTurnQ.setPermissionMode(mode)`
  // and flip the in-progress conversation without aborting it.
  let currentTurnQ: Query | null = null;
  // Default mode mirrors the original v0 behaviour. The user can flip
  // via the UI pill — pure auto-bypass is the most common ergonomic
  // choice for cc-connect's "trusted Room" model.
  let currentMode: SupportedPermissionMode = 'bypassPermissions';

  // Auto-greet on Room join — mirrors the TUI's launcher-script path
  // (`claude-wrap.sh` invokes claude with bootstrap-prompt.md as the
  // first user message). We deliberately do NOT re-queue this on
  // `resetSession()` — that would re-broadcast a greeting to peers
  // every time the user clicks New chat, which is noisy.
  if (opts.initialPrompt && opts.initialPrompt.trim()) {
    queue.push(opts.initialPrompt.trim());
  }

  function publishState(): void {
    opts.onStateChange({
      busy: processing,
      queued: queue.length,
      mode: currentMode,
    });
  }

  async function runOne(prompt: string): Promise<void> {
    const ac = new AbortController();
    currentTurnAc = ac;
    const sessionOpt = hasStarted
      ? { resume: sessionUuid }
      : { sessionId: sessionUuid };
    const systemPromptOpt =
      opts.systemPromptAppend && opts.systemPromptAppend.trim()
        ? {
            systemPrompt: {
              type: 'preset' as const,
              preset: 'claude_code' as const,
              append: opts.systemPromptAppend,
            },
          }
        : {};
    const q = query({
      prompt,
      options: {
        ...sessionOpt,
        ...systemPromptOpt,
        pathToClaudeCodeExecutable: claudeBin,
        includeHookEvents: true,
        abortController: ac,
        env: { ...process.env, CC_CONNECT_ROOM: opts.topic },
        // The user-selected mode. `bypassPermissions` is the v0
        // default; `acceptEdits` and `plan` are also supported via
        // the pane-head pill. `default` is intentionally hidden —
        // the SDK's headless `canUseTool` path can throw ZodError on
        // some tool calls, and we don't have a webview-side approval
        // UI yet.
        permissionMode: currentMode as PermissionMode,
      },
    });
    currentTurnQ = q;
    hasStarted = true;
    try {
      for await (const evt of q) {
        if (ac.signal.aborted) break;
        opts.onEvent(evt);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (!/abort/i.test(msg)) {
        opts.onEvent({ type: 'sdk:error', error: msg });
      }
    } finally {
      if (currentTurnAc === ac) currentTurnAc = null;
      if (currentTurnQ === q) currentTurnQ = null;
    }
  }

  async function processNext(): Promise<void> {
    if (processing || panelClosed) return;
    const next = queue.shift();
    if (next === undefined) return;
    processing = true;
    publishState();
    try {
      await runOne(next);
    } finally {
      processing = false;
      publishState();
      if (queue.length > 0 && !panelClosed) void processNext();
    }
  }

  // Drain the bootstrap (if any) on the next tick. We can't call
  // processNext() inline here because the caller hasn't received the
  // handle yet — `onEvent` posts may race with the webview registering
  // listeners on the WebviewView. setTimeout(0) is enough.
  if (queue.length > 0) {
    setTimeout(() => {
      if (!panelClosed) void processNext();
    }, 0);
  }
  publishState();

  return {
    enqueue(prompt: string): void {
      if (panelClosed) return;
      const trimmed = prompt.trim();
      if (!trimmed) return;
      queue.push(trimmed);
      publishState();
      void processNext();
    },
    interrupt(): void {
      // Abort only the current turn. The for-await loop in runOne
      // exits, the finally block clears `processing`, and
      // `processNext` advances to the next queued prompt (if any).
      currentTurnAc?.abort();
    },
    setPermissionMode(mode: SupportedPermissionMode): void {
      if (panelClosed) return;
      if (mode === currentMode) return;
      currentMode = mode;
      // Flip the in-flight conversation immediately if there is one.
      // Errors are swallowed: the next turn will pick up the new mode
      // anyway, so this is best-effort.
      const live = currentTurnQ;
      if (live) {
        void live.setPermissionMode(mode as PermissionMode).catch(() => {
          /* SDK may reject if the turn already finished — fine */
        });
      }
      publishState();
    },
    resetSession(): void {
      if (panelClosed) return;
      queue.length = 0;
      currentTurnAc?.abort();
      sessionUuid = randomUUID();
      hasStarted = false;
      publishState();
    },
    abort(): void {
      panelClosed = true;
      queue.length = 0;
      currentTurnAc?.abort();
      publishState();
    },
  };
}
