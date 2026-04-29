import React from "react";
import { Box, Text } from "ink";
import { theme } from "../theme.ts";

export interface MentionPopupProps {
  candidates: readonly string[];
  selectedIdx: number;
}

/** @-mention completion popup. Renders inline above the input box;
 *  parent shows/hides based on `currentAtToken` + non-empty candidates. */
export function MentionPopup({ candidates, selectedIdx }: MentionPopupProps) {
  if (candidates.length === 0) return null;
  return (
    <Box flexDirection="column" borderStyle="round" borderColor={theme.accent} paddingX={1}>
      {candidates.map((nick, i) => {
        const sel = i === selectedIdx;
        return (
          <Text
            key={`${nick}-${i}`}
            color={sel ? theme.fg : theme.mute}
            backgroundColor={sel ? theme.accent : undefined}
          >
            {sel ? "▶ " : "  "}@{nick}
          </Text>
        );
      })}
      <Text color={theme.mute}>↑↓ select · Tab/Enter accept · Esc cancel</Text>
    </Box>
  );
}
