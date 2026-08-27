# PKG-03 03.07 — Research

Task: `03.07 — NSIS per-machine elevated install and uninstall lifecycle`
Linear: `ABD-82`
Canonical base reviewed: `a5c7781767d9bf5870f66085de7f3c247b943b87`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`
Reviewed: 2026-08-27

## Repository authority

- Canonical PKG-03 is `6/25 = 24%`; `03.01–03.06=DONE`; deterministic cursor is `03.07`.
- `03.07–03.10` are dependency-ready Wave 2 tasks. This branch owns only `03.07`.
- 03.07 depends on canonical 03.02 bundle determinism, 03.03 identity/version/publisher metadata, 03.04 per-machine scope/elevation, and 03.05 ownership/containment.
- The default product config remains `currentUser`; the already-accepted overlay `apps/desktop/src-tauri/tauri.per-machine.conf.json` selects `perMachine`.
- 03.07 does not own MSI/WiX lifecycle (03.08), shortcut/application-registration semantics (03.09), CLI/Agent placement (03.10), service lifecycle (03.11), ACL/data separation (03.12), comprehensive cleanup/preservation (03.17), unattended deployment (03.21), signing (03.22), or updater/recovery (PKG-04).

## Current Tauri / Windows review

Official upstream Tauri v2 documentation was rechecked on 2026-08-27.

- `perMachine` installs system-wide in Program Files and requires Administrator privileges.
- Installer metadata is written under HKLM for `perMachine`.
- `both` still requires Administrator privileges even if a user later chooses the current-user path, so VSN continues to reject it.
- The current VSN per-machine overlay already expresses the required mode; no custom NSIS template or product mutation is required.

Primary source:
- https://v2.tauri.app/distribute/windows-installer/
- https://v2.tauri.app/reference/config/

## Hosted-runner evidence correction

Initial planning assumed the GitHub-hosted `windows-2025` image would expose `EnableLUA=0`. Exact-head run `33027545330` disproved that fixed assumption before installer launch: the stock per-machine NSIS bundle built successfully, but the harness observed a UAC policy value that did not satisfy `uac_disabled=true` and stopped before `Start-Process`.

This is an environment-observation correction, not a product/configuration change.

The durable certification rule is therefore:
- measure and record the runner UAC policy (`EnableLUA`) exactly as observed;
- do not require a predetermined `EnableLUA` value;
- require the current runner token to be Administrator, elevated and high-integrity;
- require installer and uninstaller process tokens to be elevated/high-integrity;
- require Program Files placement and HKLM registration;
- explicitly set `uac_prompt_observed=false` and `uac_prompt_certified=false`;
- do not use an explicit `RunAs` verb.

A hosted-runner UAC policy value is context evidence only. It is not accepted as a substitute for the actual elevated process-token and per-machine filesystem/registry assertions.

## Decision

`change_required=false`.

No product or Tauri configuration change is required. The task-local planning/evidence harness must only remove the invalid fixed `EnableLUA=0` expectation and retain all privilege, scope, GUI and cleanup assertions.

## Genuine 03.07 evidence model

Exact-head GitHub-hosted `windows-2025` evidence must:

1. build the NSIS target with the accepted per-machine overlay and no product mutation;
2. prove the runner is Administrator/elevated/high-integrity and record the observed `EnableLUA` value;
3. launch setup with an empty argument vector — no `/S`, `/P`, `/UPDATE`, or `RunAs`;
4. observe a visible NSIS installer window and progress normal enabled GUI controls;
5. prove the resulting install root is `%ProgramFiles%\VSN Dev Platform`, not LocalAppData;
6. prove package registration exists at `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\VSN Dev Platform`;
7. prove the corresponding HKCU package registration is absent;
8. prove `VSN Dev Platform.exe` and `uninstall.exe` exist after install;
9. prove `bin/vsn.exe` and `bin/vsn-agent.exe` remain absent, preserving 03.10 authority;
10. prove the installer process token is elevated/high-integrity;
11. launch the generated uninstaller interactively with an empty argument vector and prove its process token is elevated/high-integrity;
12. complete the visible uninstall flow and prove HKLM registration and clean installed executable payload disappear;
13. prove the current-user LocalAppData install root remains absent;
14. emit exact-source machine-readable evidence and prove tracked repository drift remains zero.

## Evidence boundary

03.07 certifies one clean elevated per-machine NSIS lifecycle on the exact observed GitHub-hosted Windows environment. It does not certify:
- an actual UAC consent/credential prompt;
- a fixed GitHub-hosted `EnableLUA` policy;
- behavior from a standard non-admin Windows account;
- MSI/WiX enterprise lifecycle;
- shortcut semantics;
- CLI/Agent placement;
- service/ACL behavior;
- dirty-data preservation;
- unattended deployment;
- Authenticode signing;
- updater/recovery.
