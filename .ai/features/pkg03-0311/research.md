# PKG-03 03.11 Research — VSN Agent Windows service lifecycle v2

Reviewed: 2026-08-28
Canonical base: `af67f43ac3104eb5dbfd133dff881e79a8ea71f4`
Linear: `ABD-86`
Supersedes stale planning lane: PR #125 / `b3d0b7d3e7f763ffb5847df1f3301c002a1d5a06`
Change classification: `COMPLETION`
Implementation gap: `MISSING_IMPLEMENTATION`

## Live-state reconciliation

- Canonical PKG-03 remains `10/25 = 40%`, active task `03.11`, status `READY`.
- Dependencies `03.07` and `03.10` remain canonically DONE.
- Comparison from old planning base `4f33813bec4254107e6027e98b2a4a8878b9198b` to current main shows Governance V3/live-state changes only; no Agent runtime, service-control core, Windows payload config, or per-machine config mutation occurred.
- Current `tauri.windows.conf.json` still owns exactly one staged Agent payload at `bin/vsn-agent.exe`.
- Current `tauri.per-machine.conf.json` still isolates NSIS `installMode=perMachine`.
- The Agent still exposes `service install/start/stop/status/uninstall` and `--service-run`; install provisions local IPC and creates `VSN-Agent` with automatic start, `NT AUTHORITY\LocalService`, and display name `VSN Agent`.

## Current platform research

Official Tauri v2 Windows installer documentation confirms:
- NSIS supports `installerHooks` with `NSIS_HOOK_POSTINSTALL` after files are copied and `NSIS_HOOK_PREUNINSTALL` before files are removed.
- WiX extensions are supported through `bundle.windows.wix.fragmentPaths` plus explicit fragment/component/feature references.
- Replacing the entire NSIS or WiX template is unnecessary when supported extension points are sufficient.

Official WiX v3 documentation confirms:
- a `CustomAction` using `Directory` + `ExeCommand` can execute an installed executable;
- deferred custom actions can run in-script with elevated privileges;
- `Return="check"` makes a non-zero result fail the action;
- `Impersonate="no"` is the appropriate elevated execution boundary for a per-machine deferred custom action.

## Implementation decision

The existing service-management CLI is the single service lifecycle authority. The installer integration will invoke that accepted interface rather than duplicate service-install logic.

### NSIS
- current-user lifecycle: service macros are compile-time/runtime no-ops for machine service ownership;
- per-machine POSTINSTALL: invoke installed `bin\vsn-agent.exe service install`, then `service start`;
- per-machine PREUNINSTALL: bounded `service stop`, then `service uninstall` before payload deletion;
- no full NSIS template.

### MSI/WiX
Use a task-owned WiX fragment with deferred, non-impersonated, synchronous custom actions invoking the already-installed Agent from `INSTALLDIR\bin\vsn-agent.exe`.
- install actions are conditioned to initial install, after files exist;
- uninstall stop/remove actions run while the installed Agent still exists and before file removal;
- no second `File`/Component owns `bin/vsn-agent.exe`;
- no full WiX template;
- if Tauri CLI 2.11.4 cannot compile/link/schedule this fragment without unrelated ownership mutation, STOP_AND_REASSESS rather than widening scope.

## Preserved boundaries

No Agent/core runtime mutation, duplicate Agent payload, PATH/environment mutation, ACL/state/config work, firewall/hosts/DNS/trust mutation, repair/rollback/reboot semantics, silent/passive deployment, signing, updater, or recovery scope is authorized.

References:
- https://v2.tauri.app/distribute/windows-installer/
- https://v2.tauri.app/reference/config/
- https://docs.firegiant.com/wix3/xsd/wix/customaction/
