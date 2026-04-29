import React from "react";
import { Box, Text } from "ink";
import { theme } from "../theme.ts";

export interface InputBoxProps {
  value: string;
  cursorVisible: boolean;
}

/** Single-line input box. We don't use ink-text-input because we need
 *  the @-mention popup integrated into key handling — App.tsx owns the
 *  global useInput hook and writes back into `value`. */
export function InputBox({ value, cursorVisible }: InputBoxProps) {
  return (
    <Box borderStyle="round" borderColor={theme.border} paddingX={1}>
      <Text>
        <Text color={theme.accent}>{"› "}</Text>
        <Text color={theme.fg}>{value}</Text>
        {cursorVisible ? <Text backgroundColor={theme.accent}> </Text> : null}
      </Text>
    </Box>
  );
}
