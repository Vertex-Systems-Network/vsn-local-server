# PKG-03 03.06 — Research

Task: `03.06 — NSIS current-user interactive install and uninstall lifecycle`
Linear: `ABD-81`
Canonical base reviewed: `bc8d1403e589fa5f4f9833f6975b5cb53e94e01c`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`
Reviewed: 2026-08-27

## Repository authority

- Canonical PKG-03 is `5/25 = 20%`; `03.01–03.05=DONE`; deterministic cursor is `03.06`.
- `03.06–03.10` are dependency-ready Wave 2 tasks. This branch owns only `03.06`.
- 03.06 depends on canonical 03.02 bundle determinism, 03.03 identity/version/publisher metadata, 03.04 current-user scope/elevation, and 03.05 ownership/containment.
- Current product configuration already sets `bundle.windows.nsis.installMode = currentUser`.
- 03.06 does not own per-machine elevation (03.07), MSI/WiX lifecycle (03.08), Start Menu/application-registration semantics (03.09), CLI/Agent placement (03.10), service lifecycle (03.11), ACL/data separation (03.12), comprehensive cleanup/preservation (03.17), unattended/silent deployment (03.21), signing (03.22), or updater/recovery (PKG-04).

## Current Tauri / NSIS review

Official upstream Tauri v2 NSIS behavior was rechecked on 2026-08-27.

- For `currentUser`, Tauri emits `RequestExecutionLevel user`.
- The stock template selects `$LOCALAPPDATA\${PRODUCTNAME}` as the current-user default install directory.
- Add/Remove Programs metadata is written through `SHCTX`; under current-user mode this resolves to HKCU.
- The stock template exposes normal MUI installer pages and a normal interactive uninstaller confirmation flow.
- `/P` activates Tauri passive mode and skips normal pages.
- NSIS `/S` is silent mode. Neither `/P` nor `/S` is acceptable evidence for this task because unattended/passive deployment is reserved for 03.21.
- Installer hooks are supported by upstream, but no custom hook/template is required for the accepted current VSN configuration.

Primary sources:
- https://v2.tauri.app/distribute/windows-installer/
- https://github.com/tauri-apps/tauri/blob/5e2856e3209d4ab16d21a1f828ff94b46a35a0b6/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi
- https://nsis.sourceforge.io/Docs/Chapter3.html

## Decision

`change_required=false`.

The accepted product configuration already expresses the required current-user NSIS mode. 03.06 therefore does not need a product-configuration or custom-installer-template mutation to prove its lifecycle.

After planning governance passes, 03.06 may add only task-local validation / Windows UI-driving certification code and its workflow.

## Interactive evidence model

Genuine 03.06 evidence must be produced on GitHub-hosted `windows-2025` from the exact source head and must:

1. build the locked NSIS setup executable from source;
2. launch setup with an empty argument vector — no `/S`, `/P`, `/UPDATE`, `/R`, `/ARGS`, or `runas`;
3. observe a visible NSIS top-level window and progress normal enabled GUI controls with Windows UI automation;
4. prove the resulting installation root is exactly in the current user's LocalAppData class and outside Program Files;
5. prove current-user registration exists under `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\VSN Dev Platform` with expected product/version/publisher/install-location/uninstall metadata;
6. prove the corresponding HKLM uninstall registration was not created by this lifecycle;
7. prove `VSN Dev Platform.exe` and `uninstall.exe` exist after install;
8. prove `bin/vsn.exe` and `bin/vsn-agent.exe` are still absent, preserving 03.10 placement authority;
9. launch the generated uninstaller interactively with an empty argument vector, observe its visible GUI, and complete the normal uninstall flow without selecting destructive app-data deletion;
10. prove the HKCU uninstall registration and clean installed executable payload disappear after uninstall;
11. emit exact-source machine-readable evidence and prove tracked repository drift remains zero.

## Evidence boundary

This task certifies one clean current-user interactive lifecycle only. It does not certify:
- elevated/per-machine behavior;
- enterprise MSI semantics;
- reinstall/repair/rollback;
- running-process/reboot semantics;
- comprehensive user-data preservation;
- unattended/passive/silent deployment;
- Authenticode signing;
- updater/recovery behavior.
