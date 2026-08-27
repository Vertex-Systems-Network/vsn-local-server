# PKG-03 03.06 — Development Preflight

Task: `03.06`
Linear: `ABD-81`
Canonical base: `bc8d1403e589fa5f4f9833f6975b5cb53e94e01c`
Parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Authority check

- Parent PKG-03 plan is frozen.
- 03.02, 03.03, 03.04 and 03.05 are DONE on canonical main.
- PKG-03 is `5/25 = 20%`.
- 03.06 is Wave 2 / build lane, dependency-ready and the deterministic cursor.
- Linear ABD-81 is In Progress.
- `03.07–03.10` being READY does not grant this branch authority over those lanes.
- No self-expansion into 03.07–03.25 or PKG-04 is permitted.

## Locked product inputs

- `apps/desktop/src-tauri/tauri.conf.json`
  - `productName = VSN Dev Platform`
  - `version = 0.38.1`
  - `identifier = dev.vsn.platform`
  - `publisher = Vertex Systems Network`
  - `bundle.windows.nsis.installMode = currentUser`
- 03.05 owned paths remain exactly:
  - `VSN Dev Platform.exe`
  - `bin/vsn.exe`
  - `bin/vsn-agent.exe`
- CLI/Agent real installer placement remains 03.10.

## Planned certification surface

After planning-gate acceptance:
- `scripts/ci/validate-pkg03-0306.py`
- `scripts/ci/pkg03-0306-interactive-nsis.ps1`
- `.github/workflows/pkg03-0306-nsis-user-install.yml`

No product file is planned to change.

## Required exact-head gates

Planning head:
- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence

Final implementation/evidence head:
- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence
- PKG-03 03.06 NSIS Current-User Lifecycle

## Interactive-only rule

Task evidence is invalid if setup or uninstall is launched using:
- `/S`;
- `/P`;
- `/UPDATE`;
- an elevation verb such as `runas`;
- any wrapper that suppresses the normal GUI lifecycle.

The evidence harness must observe visible installer and uninstaller windows and record GUI-control progression.

## Stop conditions

Stop rather than widen scope if:
- canonical main or parent-plan bytes drift before mutation;
- any 03.02–03.05 prerequisite is no longer DONE;
- product identity/currentUser/ownership contracts drift;
- genuine interactive automation cannot distinguish itself from passive/silent execution;
- current-user lifecycle requires a custom NSIS template or product change not already frozen;
- successful evidence requires per-machine elevation, MSI, CLI/Agent placement, service registration, ACL mutation, signing secrets, updater behavior or broader cleanup semantics.
