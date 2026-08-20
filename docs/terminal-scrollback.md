# Durable PTY scrollback — VSN 0.15

New PTY/ConPTY sessions write a bounded raw-output journal under the VSN data directory. Each session journal is capped at 64 MiB. The active in-memory PTY buffer remains bounded separately.

`terminal.pty.scrollback.list` lists journals, `terminal.pty.scrollback.read` provides offset-based base64 chunks up to 256 KiB, and `terminal.pty.scrollback.remove` deletes only inactive journals.

VSN does not recreate a PTY after Agent restart. That is intentional: the previous shell may already have executed side effects whose exact completion state is unknown. Durable scrollback preserves evidence/output without re-running commands.
