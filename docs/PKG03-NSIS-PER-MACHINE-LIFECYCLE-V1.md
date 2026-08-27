# PKG-03 NSIS Per-Machine Lifecycle Contract v1

Task authority: `03.07` / Linear `ABD-82`.
Canonical base: `a5c7781767d9bf5870f66085de7f3c247b943b87`.

## Purpose

Certify the existing stock Tauri NSIS `perMachine` variant through a real clean elevated install/uninstall lifecycle without changing the product or widening into later PKG-03 tasks.

## Configuration contract

- Default product config remains `currentUser`.
- `apps/desktop/src-tauri/tauri.per-machine.conf.json` is the only mode overlay and remains `perMachine`.
- `both` remains forbidden by the accepted 03.04 architecture.
- No custom NSIS template is introduced.
- Product identity and owned-payload contracts remain unchanged.

## Per-machine installed state

Successful clean install must produce:
- install root `%ProgramFiles%\VSN Dev Platform`;
- `VSN Dev Platform.exe`;
- `uninstall.exe`;
- VSN uninstall metadata under HKLM;
- no corresponding VSN uninstall metadata under HKCU;
- no `bin/vsn.exe` or `bin/vsn-agent.exe` before 03.10.

The current-user `%LOCALAPPDATA%\VSN Dev Platform` root must remain absent.

## Privilege contract

The per-machine installer requires Administrator privilege.

On GitHub-hosted Windows:
- the runner already executes as Administrator;
- UAC is disabled by the environment;
- certification must prove the runner, installer and uninstaller execute with elevated/high-integrity tokens;
- certification must not claim that an end-user UAC prompt was displayed.

No explicit `RunAs` verb is used by the harness.

## Interactive contract

Setup and uninstall must:
- be launched with empty argument vectors;
- not use `/S`, `/P`, `/UPDATE`, or other silent/passive wrappers;
- expose visible NSIS windows;
- be progressed through normal enabled GUI controls.

Terminal-page native fallback is allowed only as a task-local UI-driving mechanism against the same visible NSIS controls/window when UIAutomation invocation is ignored; it may not bypass installer logic or turn the lifecycle into silent execution.

## Clean uninstall contract

After interactive uninstall:
- HKLM VSN uninstall registration is gone;
- Program Files `VSN Dev Platform.exe` is gone;
- Program Files `uninstall.exe` is gone;
- LocalAppData VSN install root remains absent.

Comprehensive dirty user-data preservation remains 03.17.

## Explicit nonclaims

03.07 does not certify:
- actual UAC consent/credential UI;
- standard-user denial behavior;
- MSI/WiX lifecycle;
- Start Menu/desktop shortcut semantics;
- CLI/Agent placement or launch;
- service lifecycle;
- ACL/data-separation behavior;
- repair/rollback/reboot behavior;
- silent/passive deployment;
- signing;
- updater/recovery.
