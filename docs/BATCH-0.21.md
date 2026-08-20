# Batch 0.21 — close-first sprint

This batch deliberately closes finite phases before opening new work.

## Closed source-scope phases

- **P1 Local Core:** config v3 migration, backup/atomic commit/stale-temp recovery, diagnostics.
- **P2 Secure Agent:** authenticated bounded IPC plus command identifier checks, 5-second connection timeouts and 128 concurrent-connection cap.
- **P7 Database Driver SDK:** advertised capabilities now have typed trait methods for functions/users/permissions/import/export/backup/restore; provider descriptors have conformance validation.
- **P11 CLI:** machine-readable command catalog plus Bash/Zsh/PowerShell completions and version/help discovery.
- **P20 Secret / Team Vault:** local versioned-key recovery lifecycle plus shared Team Vault multi-key keyring and atomic all-secret key rotation.
- **P22 Multi-node / Fleet:** shared group/environment lifecycle including delete plus cross-reference consistency validation.
- **P24 Containers:** inspect/stats/direct-argv exec, image/container/volume/network mutation surface, registry tag/push and complete bounded Compose lifecycle.

## Deepened, not falsely closed

- **P3:** graceful managed-process stop with force escalation, list/status/remove; OS-native service mutation remains provider/elevation specific.
- **P6:** workspace-contained detection/dependency/bootstrap plus bounded direct-process bootstrap; provider extensibility edges remain.

Native multi-OS certification remains P30 evidence, not a reason to pretend source implementation is missing or to pretend certification already passed.
