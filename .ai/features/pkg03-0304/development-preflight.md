# PKG-03 03.04 — Development Preflight

Task: `03.04`
Linear: `ABD-79`
Authoritative PR: `#112`
Base: `8f2919923005ba29b1475bd646a3f6953100ca9e`
Parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Authority check

- Parent PKG-03 plan is frozen.
- 03.01, 03.02 and 03.03 are DONE on canonical main.
- 03.04 is Wave 1 / scope lane and dependency-ready.
- Canonical PKG-03 pre-state is `3/25 = 12%`, with 03.04–03.05 READY and cursor 03.04.
- Linear ABD-79 is the task mirror.
- PR #112 is the single authoritative 03.04 implementation/certification PR.
- No self-expansion into 03.05, 03.06–03.08, 03.11–03.12, 03.22 or PKG-04 is permitted.

## Planned changed product surface

After the planning head passes the required governance gates:

- `apps/desktop/src-tauri/tauri.conf.json`
  - add `bundle.windows.nsis.installMode = "currentUser"` as the explicit least-privilege default;
- `apps/desktop/src-tauri/tauri.per-machine.conf.json`
  - task-owned overlay with `bundle.windows.nsis.installMode = "perMachine"`.

Existing `productName`, `version`, `identifier`, publisher, downgrade policy and WiX UpgradeCode must not change.

## Certification surface

- `.ai/features/pkg03-0304/*`
- `.ai/plans/pkg03-0304-install-scope-elevation-v1.md`
- `.ai/manifests/pkg03-0304-install-scope-elevation.v1.json`
- `docs/PKG03-WINDOWS-INSTALL-SCOPE-ELEVATION-V1.md`
- `scripts/ci/validate-pkg03-0304.py`
- `.github/workflows/pkg03-0304-install-scope-elevation.yml`

## Required exact-head gates

- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence
- PKG-03 03.04 Install Scope + Elevation

## Stop conditions

Stop rather than widen scope if:
- parent-plan bytes drift;
- canonical 03.01–03.03 are not DONE;
- accepted 03.03 identity metadata drifts;
- Tauri rejects either scoped config path;
- the current-user path requires elevation;
- supporting MSI per-user mode would require a custom WiX template;
- any task would need installer execution, service registration, payload ownership, ACL mutation, signing secrets, updater behavior or privileged system mutation.
