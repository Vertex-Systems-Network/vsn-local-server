# Security

VSN Devify is built to keep attackers away from your computer and to stop them even if something gets in:

- **Local only by default** — services bind to 127.0.0.1; shares are opt-in, private-network-only and expire.
- **Workspace boundary** — files, terminals, git, transfers and backups only work inside your workspace folders; symlinks that escape are refused.
- **Encrypted vault** — passwords, tokens and API keys are stored in the OS-protected vault and never shown on screen.
- **Space lock** — protect a space with a password.
- **Remote access is deny-by-default** — a remote caller cannot run programs, read space settings, use saved database connections or reach local-only tools.
- **Tamper-evident audit log** — every action is recorded in a signed hash chain; a broken chain is reported by **Diagnose** and new records are never added to a damaged chain.
- **Signed runtimes and updates** — downloads are checksum-verified; the updater refuses redirected control folders and only restores backups whose digest it recorded.
- **No console windows** — tools run hidden on Windows.

Report a security problem privately to Vertex Systems Network.
