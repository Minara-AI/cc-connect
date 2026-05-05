# Install the cc-connect binary

The bootstrap script downloads a **pre-built binary** for your
platform — no Rust toolchain required. Detects macOS arm64 / x86_64
or Linux x86_64.

```
curl -fsSL https://raw.githubusercontent.com/Minara-AI/cc-connect/main/scripts/bootstrap.sh | bash
```

What it does:

1. Downloads the latest release tarball that matches your platform.
2. Verifies the SHA-256 checksum.
3. Symlinks `cc-connect`, `cc-connect-hook`, `cc-connect-mcp`,
   `cc-chat-ui` into `~/.local/bin/`.
4. Backs up `~/.claude/settings.json` and registers the
   UserPromptSubmit hook (this is what injects peers' chat into
   your next prompt).
5. Registers `cc-connect-mcp` so Claude gets `cc_send`,
   `cc_at`, `cc_drop`, `cc_wait_for_mention`.
6. Runs `cc-connect doctor` to verify everything is wired.

If you'd rather build from source, set
`CC_CONNECT_FROM_SOURCE=1` before running the script.
