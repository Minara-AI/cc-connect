// Sidebar tree of Rooms. Lists every directory under
// ~/.cc-connect/rooms/<topic-hex>/ and marks each "alive" or "dormant"
// based on whether its chat-daemon.pid file points at a process that
// still responds to a signal-0 probe.
//
// Refresh is on-demand (Refresh button on the view title + after the
// extension's Start / Join / Stop commands fire). Could be replaced
// by an fs.watch on ~/.cc-connect/rooms/ later if the manual refresh
// gets old.

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';

interface RoomEntry {
  topic: string;
  alive: boolean;
  pid?: number;
  ticket?: string;
}

export class RoomsProvider implements vscode.TreeDataProvider<RoomEntry> {
  private readonly _onDidChange = new vscode.EventEmitter<
    RoomEntry | undefined
  >();
  readonly onDidChangeTreeData = this._onDidChange.event;

  refresh(): void {
    this._onDidChange.fire(undefined);
  }

  getTreeItem(entry: RoomEntry): vscode.TreeItem {
    const ti = new vscode.TreeItem(`${entry.topic.slice(0, 12)}…`);
    ti.id = entry.topic;
    ti.tooltip = entry.topic;
    ti.contextValue = entry.alive ? 'room.alive' : 'room.dormant';
    ti.iconPath = new vscode.ThemeIcon(
      entry.alive ? 'circle-filled' : 'circle-outline',
      entry.alive
        ? new vscode.ThemeColor('charts.green')
        : new vscode.ThemeColor('disabledForeground'),
    );
    ti.description = entry.alive ? 'running' : 'dormant';
    ti.command = {
      command: 'cc-connect.openRoom',
      title: 'Open Room',
      arguments: [entry.topic],
    };
    return ti;
  }

  getChildren(): RoomEntry[] {
    const roomsDir = path.join(os.homedir(), '.cc-connect', 'rooms');
    let names: string[] = [];
    try {
      names = fs.readdirSync(roomsDir);
    } catch {
      return [];
    }
    const entries: RoomEntry[] = [];
    for (const name of names) {
      // Topic hex is 64 lowercase hex chars (per protocol).
      if (!/^[0-9a-f]{64}$/.test(name)) continue;
      const pidPath = path.join(roomsDir, name, 'chat-daemon.pid');
      let alive = false;
      let pid: number | undefined;
      let ticket: string | undefined;
      try {
        const raw = fs.readFileSync(pidPath, 'utf8');
        const parsed = JSON.parse(raw) as { pid?: number; ticket?: string };
        pid = parsed.pid;
        ticket = parsed.ticket;
        if (typeof pid === 'number') {
          try {
            // signal 0 is a no-op that throws ESRCH if the pid is gone.
            process.kill(pid, 0);
            alive = true;
          } catch {
            alive = false;
          }
        }
      } catch {
        // PID file missing = dormant Room (history exists, daemon doesn't).
      }
      entries.push({ topic: name, alive, pid, ticket });
    }
    // Alive first, then alphabetical.
    entries.sort(
      (a, b) =>
        Number(b.alive) - Number(a.alive) || a.topic.localeCompare(b.topic),
    );
    return entries;
  }
}
