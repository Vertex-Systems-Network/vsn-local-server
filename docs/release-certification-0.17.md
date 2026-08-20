# Release certification matrix — 0.17

| Target | Build source | Service integration | Signing source | Artifact status |
|---|---|---|---|---|
| Windows x64 | WiX/MSI | `VSNAgent` LocalService | SignTool script | source-defined, not certified |
| Linux | deb builder | user systemd unit | distribution-specific later | source-defined, not certified |
| macOS | pkg builder | LaunchAgent | productsign + optional notarytool | source-defined, not certified |

Certification requires real runner build/install/uninstall/service restart/updater rollback testing plus signature verification. This repository does not embed private signing credentials.
