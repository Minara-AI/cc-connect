// Compatibility check between the VSCode extension and the
// `cc-connect` binary it shells out to. Without this, an extension
// shipped after a binary feature lands cryptically fails when the
// user has an old binary on disk — e.g. `cc-connect host-bg start`
// returning "unknown subcommand" because the binary predates the
// flag.
//
// The check runs at activate() and gates start/join with a friendly
// "your binary is too old, run upgrade" toast instead of letting
// daemon.ts spawn fail downstream.

import { spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

/** Minimum cc-connect binary version this extension requires.
 *  Bump together with any extension change that depends on a new
 *  binary feature (new subcommand, new flag, new IPC envelope). */
export const MIN_CC_CONNECT_VERSION = '0.5.0';

const CC_BIN = path.join(os.homedir(), '.local', 'bin', 'cc-connect');

export type BinaryHealth =
  | { ok: true; version: string }
  | { ok: false; reason: 'missing' }
  | {
      ok: false;
      reason: 'outdated';
      version: string;
      required: string;
    }
  | { ok: false; reason: 'unreadable'; detail: string };

export async function checkCcConnectBinary(): Promise<BinaryHealth> {
  try {
    fs.accessSync(CC_BIN, fs.constants.X_OK);
  } catch {
    return { ok: false, reason: 'missing' };
  }

  let stdout = '';
  let exitCode = 0;
  try {
    ({ stdout, code: exitCode } = await runVersion());
  } catch (e) {
    return {
      ok: false,
      reason: 'unreadable',
      detail: e instanceof Error ? e.message : String(e),
    };
  }
  if (exitCode !== 0) {
    return {
      ok: false,
      reason: 'unreadable',
      detail: `cc-connect --version exited ${exitCode}: ${stdout.trim()}`,
    };
  }

  // clap's auto-generated --version emits `cc-connect <semver>` on a
  // single line. Match the first semver-shaped token; tolerate
  // `0.5.0`, `0.5.0-alpha`, `0.5.0-rc.1`, `1.0.0+build.5`, …
  const m = /\b(\d+\.\d+\.\d+(?:[-+][\w.]+)*)\b/.exec(stdout);
  if (!m) {
    return {
      ok: false,
      reason: 'unreadable',
      detail: `could not parse version from: ${stdout.trim().slice(0, 200)}`,
    };
  }
  const version = m[1];
  if (semverLt(version, MIN_CC_CONNECT_VERSION)) {
    return {
      ok: false,
      reason: 'outdated',
      version,
      required: MIN_CC_CONNECT_VERSION,
    };
  }
  return { ok: true, version };
}

function runVersion(): Promise<{ stdout: string; code: number }> {
  return new Promise((resolve, reject) => {
    const p = spawn(CC_BIN, ['--version'], {
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    p.stdout.on('data', (c: Buffer) => (stdout += c.toString('utf8')));
    p.stderr.on('data', (c: Buffer) => (stdout += c.toString('utf8')));
    p.on('error', reject);
    p.on('exit', (code) => resolve({ stdout, code: code ?? -1 }));
  });
}

/** Returns true iff `a` is strictly less than `b` by major/minor/patch
 *  comparison. Pre-release suffixes (`-alpha`, `-rc.1`) are treated as
 *  *equal* to the release version for compatibility checks — a user on
 *  `0.5.0-alpha` satisfies a `0.5.0` requirement. The cc-connect
 *  release cadence uses `-alpha` for active development of a version,
 *  not for "less than"; treating them as < the bare version would
 *  lock out alpha users from features that already work for them. */
export function semverLt(a: string, b: string): boolean {
  const parse = (s: string): [number, number, number] => {
    const core = s.split(/[-+]/, 1)[0];
    const parts = core.split('.').map((n) => parseInt(n, 10));
    return [parts[0] ?? 0, parts[1] ?? 0, parts[2] ?? 0];
  };
  const [aMa, aMi, aPa] = parse(a);
  const [bMa, bMi, bPa] = parse(b);
  if (aMa !== bMa) return aMa < bMa;
  if (aMi !== bMi) return aMi < bMi;
  return aPa < bPa;
}
