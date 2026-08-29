# PKG-03 03.12 — Research

Task: `03.12 — Installer ACLs, state/config directories and user-data separation`
Linear: `ABD-87`
Canonical base reviewed: `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`
Reviewed: 2026-08-29

## Repository authority

- PKG-03 is canonically `11/25 = 44%`; tasks `03.01–03.11` are DONE.
- `03.12` is READY and depends only on canonical `03.07` and `03.10`, both DONE.
- The frozen 03.05 ownership contract keeps executable payload ownership limited to `VSN Dev Platform.exe`, `bin/vsn.exe`, and `bin/vsn-agent.exe` under the selected install root.
- 03.10 owns CLI/Agent placement; 03.11 owns Windows service identity/lifecycle; 03.12 owns ACL/state/config/user-data separation.
- Comprehensive dirty-data uninstall cleanup/preservation remains 03.17 and must not be pre-implemented here.

## Current source authority

Three storage classes exist and must not be conflated:

1. **Installer-owned immutable payload** — selected install root (`%LOCALAPPDATA%\VSN Dev Platform` for current-user NSIS, `%ProgramFiles%\VSN Dev Platform` for per-machine NSIS/MSI) containing only accepted package payload.
2. **Machine-shared IPC security** — `vsn-security` resolves `%PROGRAMDATA%\VSN\security\ipc.key`. The existing runtime owns secret creation and ACL tightening.
3. **Process-context mutable data/config** — `ProjectDirs::from("dev","VSN","VSN Platform")` resolves `data_local_dir()` for VSN data and `config_dir()/config.json` for configuration. Audit and managed runtime state derive from that data root.

The Agent Windows service runs as `NT AUTHORITY\LocalService`. Therefore ProjectDirs paths observed through the running Agent are service-process-context paths and must not be guessed to equal an interactive user's profile. Certification must record actual resolved paths.

## Existing IPC ACL contract

The existing Windows security implementation is authoritative:
- `%PROGRAMDATA%\VSN\security` inheritance removed;
- SYSTEM (`S-1-5-18`) Full Control;
- Builtin Administrators (`S-1-5-32-544`) Full Control;
- LocalService (`S-1-5-19`) Read;
- creating/current SID Full Control on the directory;
- `ipc.key` is tightened to SYSTEM/Admins Full Control and LocalService/current SID Read.

03.12 must not duplicate or fork that security implementation.

## Platform delta

Current Tauri v2 installer hooks and WiX v3 fragment extension points are sufficient to add bounded directory/ACL integration without a full installer-template fork. Windows ACL verification can use native SID-based security descriptor inspection / `icacls` semantics. No platform delta requires changing the frozen package DAG.

## Decision

`change_required=true`.

The repository has accepted runtime storage/ACL semantics but lacks a task-owned installer integration and exact-head Windows certification proving:
- immutable payload remains separated from mutable data/config;
- current-user package installation does not create machine-wide security state merely by installing;
- per-machine service/runtime use results in the existing ProgramData IPC contract with exact ACLs;
- NSIS and MSI paths do not broaden rights beyond the accepted security model;
- actual service-context ProjectDirs paths are outside the install root and are recorded rather than guessed.

No `vsn-security`, `vsn-config`, `vsn-core`, or Agent runtime mutation is authorized by this plan. If genuine evidence proves such a change necessary, stop and use change control before crossing that boundary.
