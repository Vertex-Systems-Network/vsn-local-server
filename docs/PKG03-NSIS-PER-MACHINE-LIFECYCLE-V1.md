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

Certification must prove:
- the runner is Administrator, elevated and high-integrity;
- installer and uninstaller process tokens are elevated/high-integrity;
- the runner's `EnableLUA` value is measured and recorded exactly as observed;
- Program Files/HKLM per-machine state is created and later removed.

No predetermined `EnableLUA` value is required or forced. UAC policy is context evidence only. Certification must not claim that an end-user UAC prompt was displayed unless separately observed under a future authorized environment; this task records `uac_prompt_observed=false` and `uac_prompt_certified=false`.

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

## Diagnostic-run provenance

Run `33027545330` / job `98372313386` successfully built the per-machine NSIS setup but stopped before installer launch because the original task harness incorrectly required `EnableLUA=0`. It is not acceptance evidence and performed no installer/uninstaller lifecycle mutation.

## Explicit nonclaims

03.07 does not certify:
- actual UAC consent/credential UI;
- a fixed hosted-runner UAC policy;
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
