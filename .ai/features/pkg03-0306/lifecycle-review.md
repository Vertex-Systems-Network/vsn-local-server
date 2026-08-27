# PKG-03 03.06 — Lifecycle Review

Task: `03.06`
Linear: `ABD-81`
Canonical base: `bc8d1403e589fa5f4f9833f6975b5cb53e94e01c`

## Lifecycle position

03.06 is Wave 2 / build lane. Canonical prerequisites `03.02–03.05` are DONE. PKG-03 is `5/25`, 03.06 is the deterministic cursor, and 03.06–03.10 are READY in parallel subject to the five-lane cap.

## Entry invariants

- PKG-03 denominator/order remains exactly 25 tasks (`03.01`–`03.25`).
- 03.01–03.05 are canonically DONE.
- 03.06 is READY or IN_PROGRESS and depends on 03.02, 03.03, 03.04 and 03.05.
- canonical cursor is 03.06 at branch start;
- `apps/desktop/src-tauri/tauri.conf.json` remains product/version `0.38.1`, identifier `dev.vsn.platform`, publisher `Vertex Systems Network`, NSIS mode `currentUser`;
- owned-payload contract remains exactly `VSN Dev Platform.exe`, `bin/vsn.exe`, `bin/vsn-agent.exe`;
- CLI/Agent placement remains declared-not-yet-packaged and owned by 03.10.

## Planning mutation boundary

The planning head may add only:
- `.ai/features/pkg03-0306/*`;
- `.ai/plans/pkg03-0306-nsis-user-install-v1.md`;
- `.ai/manifests/pkg03-0306-nsis-user-install.v1.json`;
- `docs/PKG03-NSIS-CURRENT-USER-LIFECYCLE-V1.md`.

No product configuration, NSIS/WiX template, registry, filesystem, service, ACL, signing, updater or canonical tracker/master-state mutation is authorized before planning gates pass.

## Post-planning certification authority

After exact planning gates pass, 03.06 may add only:
- task-local validator code;
- task-local Windows UI automation / evidence builder code;
- one task-specific GitHub-hosted Windows workflow.

The workflow may execute only the current-user NSIS installer/uninstaller it builds from the exact task head and only inside the hosted runner's ephemeral user profile.

## Acceptance lifecycle

1. Revalidate parent plan digest and canonical 03.02–03.05 DONE evidence.
2. Revalidate immutable 03.03 identity, 03.04 `currentUser` scope and 03.05 owned-payload contracts.
3. Build the exact-head NSIS setup using locked Node/Rust inputs.
4. Capture pre-install HKCU/HKLM uninstall keys and current-user install-root absence.
5. Launch setup with no passive/silent/update/elevation arguments.
6. Observe the visible NSIS GUI and advance normal interactive controls.
7. Verify LocalAppData install root, expected HKCU uninstall metadata and Desktop executable/uninstaller presence.
8. Verify no HKLM package registration and no CLI/Agent placement.
9. Launch `uninstall.exe` with no arguments, observe its visible GUI and complete normal uninstall without opting into app-data deletion.
10. Verify current-user registration and clean installed executable payload are removed.
11. Verify zero tracked repository drift and emit exact-source evidence.
12. Only after genuine evidence passes may 03.06 become DONE.

## State reconciliation

Pre-evidence:
- `done=5`, `percent=20.0`;
- 03.06 IN_PROGRESS/READY;
- cursor 03.06;
- 03.07–03.10 READY.

After genuine 03.06 evidence:
- `done=6`, `percent=24.0`;
- 03.06 DONE;
- 03.07–03.10 remain READY;
- deterministic cursor advances to 03.07.

03.13 and 03.15 remain blocked because they also require 03.07 and 03.08. 03.14 remains blocked because it also requires 03.07, 03.08 and 03.10.

## Explicit non-actions

No per-machine/UAC test, no MSI install, no custom installer template, no CLI/Agent placement, no service install, no ACL contract, no repair/reinstall/rollback acceptance, no comprehensive dirty-data preservation acceptance, no silent/passive deployment certification, no signing and no updater mutation.
