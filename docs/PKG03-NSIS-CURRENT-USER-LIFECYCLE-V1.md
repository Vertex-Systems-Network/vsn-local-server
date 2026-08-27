# PKG-03 NSIS Current-User Interactive Lifecycle Contract v1

Task authority: `03.06` / Linear `ABD-81`.
Parent package: PKG-03 Windows Installer.
Canonical planning base: `bc8d1403e589fa5f4f9833f6975b5cb53e94e01c`.

## Purpose

Certify one genuine, least-privilege, current-user NSIS install and uninstall lifecycle using the stock Tauri-generated Windows installer UI.

## Current-user boundary

The accepted installer remains:
- Tauri NSIS `currentUser`;
- normal user execution level, with no explicit elevation verb;
- default install root `%LOCALAPPDATA%\VSN Dev Platform`;
- package registration in the current-user registry context.

03.06 does not introduce a custom NSIS template or a second installer variant.

## Interactive evidence requirement

A process exit code alone is insufficient.

Valid evidence must show that setup and uninstall each presented a visible top-level NSIS window and that the certification harness progressed the enabled GUI controls of the normal flow.

The following shortcuts are forbidden as 03.06 acceptance evidence:
- NSIS silent `/S`;
- Tauri passive `/P`;
- update `/UPDATE`;
- `Start-Process -Verb RunAs` or equivalent elevation;
- any wrapper whose purpose is to hide or bypass the normal installer/uninstaller UI.

Silent/passive deployment remains task 03.21.

## Installed-state contract

After clean install:
- install root is `%LOCALAPPDATA%\VSN Dev Platform`;
- `VSN Dev Platform.exe` exists;
- `uninstall.exe` exists;
- HKCU uninstall metadata identifies VSN Dev Platform version `0.38.1` published by `Vertex Systems Network`;
- HKLM does not gain the VSN current-user uninstall registration;
- `bin/vsn.exe` and `bin/vsn-agent.exe` remain absent until 03.10.

## Clean uninstall contract

The installed uninstaller is launched interactively and the normal confirmation/uninstall UI is progressed.

For this clean hosted-runner lifecycle:
- HKCU VSN uninstall registration is removed;
- the installed Desktop executable and uninstaller are removed;
- no HKLM package registration is introduced.

Comprehensive preservation of pre-existing mutable user data is explicitly deferred to 03.17.

## Non-actions

This contract does not certify per-machine/UAC behavior, MSI/WiX, Start Menu policy, CLI/Agent placement, Windows service lifecycle, ACL/state separation, repair/reinstall/rollback, running-process/reboot semantics, unattended deployment, Authenticode signing, or updater/recovery.
