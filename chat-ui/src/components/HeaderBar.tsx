import React from "react";
import { Box, Text } from "ink";
import { theme } from "../theme.ts";

export interface HeaderBarProps {
  topicShort: string;
  selfNick: string | null;
  daemonAlive: boolean;
}

/** Top strip: room id + you-are-X + connection status. The ticket goes
 *  in a separate line/overlay so it doesn't fight the chat scrollback for
 *  width on narrow panes. */
export function HeaderBar({ topicShort, selfNick, daemonAlive }: HeaderBarProps) {
  return (
    <Box flexDirection="row" justifyContent="space-between" paddingX={1}>
      <Text>
        <Text color={theme.accent}>◆ </Text>
        <Text color={theme.mute}>room </Text>
        <Text color={theme.fg}>{topicShort}</Text>
      </Text>
      <Text>
        <Text color={theme.mute}>you </Text>
        <Text color={theme.accent}>{selfNick ?? "(no nick)"}</Text>
        <Text color={theme.mute}>{"  ·  "}</Text>
        <Text color={daemonAlive ? theme.success : theme.danger}>
          {daemonAlive ? "● daemon up" : "○ daemon down"}
        </Text>
      </Text>
    </Box>
  );
}
