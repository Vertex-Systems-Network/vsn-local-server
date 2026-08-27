# PKG-03 03.08 — Development Preflight

Task: `03.08`
Linear: `ABD-83`
Canonical base: `0ac71c6392c19ad070a9ec442323c46f3c0e08b9`
Parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Authority check

- Parent PKG-03 plan is frozen.
- 03.01–03.07 are DONE on canonical main.
- PKG-03 is `7/25 = 28%`.
- 03.08 is Wave 2 / enterprise lane, dependency-ready and the deterministic cursor.
- 03.09–03.10 are also READY but grant no authority to this branch.
- Linear ABD-83 is In Progress.
- No self-expansion into 03.09–03.25 or PKG-04 is permitted.

## Locked product inputs

- `apps/desktop/src-tauri/tauri.conf.json`
  - accepted SHA-256 `172cf6110e58a15442bcf97e9db6a8bdbeb6cbfd2f631d91a3031603ed474180`
  - `productName = VSN Dev Platform`
  - `mainBinaryName = VSN Dev Platform`
  - `version = 0.38.1`
  - `identifier = dev.vsn.platform`
  - `publisher = Vertex Systems Network`
  - WiX `upgradeCode = 157f304f-1d1b-55e0-b89c-0610ea27c645`
  - `allowDowngrades = false`
- `apps/desktop/src-tauri/tauri.per-machine.conf.json`
  - accepted SHA-256 `48fd4eb22ffe99a884ce5f4770de83e29ad919650d7c254b5d180fca3add7429`
  - NSIS-only scope overlay; 03.08 must not use it to alter MSI semantics.
- `installer/windows/owned-payload.v1.json`
  - accepted SHA-256 `5292a1ec1ae0a48d76f80258c9e00ba17b1466dab5c5e0cdda60caf4658dabd1`
  - owned paths remain exactly:
    - `VSN Dev Platform.exe`
    - `bin/vsn.exe`
    - `bin/vsn-agent.exe`
- CLI/Agent real installer placement remains 03.10.

## MSI/ARP invariants

- Tauri MSI is stock WiX Toolset v3 output and per-machine.
- Stable UpgradeCode is source-controlled; concrete ProductCode must be extracted from the built MSI.
- ARP acceptance is exact ProductCode-keyed HKLM registration.
- Do not assert all HKCU paths are untouched; stock WiX may write vendor bookkeeping under HKCU.
- Full/default MSI UI is required for 03.08. `/quiet`, `/passive`, `/qn`, `/qb`, `/qr`, `/qf` and equivalent UI-suppression switches are not authorized by this task.

## Required exact-head gates

Planning head:
- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence

Final implementation/evidence head:
- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence
- PKG-03 03.08 MSI Enterprise Lifecycle

## Stop conditions

Stop rather than widen scope if:
- canonical main or parent-plan bytes drift before mutation;
- any 03.02–03.05 prerequisite is no longer DONE;
- identity/UpgradeCode/ownership contracts drift;
- MSI certification requires a custom WiX template or Tauri product change;
- exact ProductCode cannot be extracted and bound to the built MSI;
- per-machine Program Files + ProductCode-keyed HKLM ARP state cannot be proven;
- visible normal Windows Installer UI cannot be exercised without silent/passive flags;
- successful evidence requires shortcut semantics, CLI/Agent placement, service registration, ACL mutation, repair/rollback behavior, signing secrets, updater behavior or broader deployment semantics.
