# PKG-03 03.03 — Development Preflight

Task: `03.03`
Linear: `ABD-78`
Base: `d1d3e6997878aa16b8d4ad05f094754b5b1699b2`
Parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Authority check

- Parent PKG-03 plan is frozen.
- 03.01 and 03.02 are DONE on canonical main.
- 03.03 is Wave 1 / identity lane and dependency-ready.
- Canonical PKG-03 pre-state is `2/25 = 8%`, with 03.03–03.05 READY and cursor 03.03.
- Linear ABD-78 is the task mirror.
- No self-expansion into 03.04, 03.05, 03.06+, 03.22 or PKG-04 is permitted.

## Planned changed product surface

`apps/desktop/src-tauri/tauri.conf.json`

Allowed metadata:
- `bundle.publisher = "Vertex Systems Network"`
- `bundle.windows.allowDowngrades = false`
- `bundle.windows.wix.upgradeCode = "157f304f-1d1b-55e0-b89c-0610ea27c645"`

Existing `productName`, `version`, and `identifier` must not change.

## Certification surface

- `.ai/features/pkg03-0303/*`
- `.ai/plans/pkg03-0303-package-identity-v1.md`
- `.ai/manifests/pkg03-0303-package-identity.v1.json`
- `docs/PKG03-WINDOWS-PACKAGE-IDENTITY-V1.md`
- `scripts/ci/validate-pkg03-0303.py`
- `.github/workflows/pkg03-0303-package-identity.yml`

## Required exact-head gates

- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence
- PKG-03 03.03 Package Identity

## Stop conditions

Stop rather than widen scope if:
- canonical identity differs from the frozen 03.01 authority;
- parent-plan bytes drift;
- canonical 03.02 is not DONE;
- Tauri rejects the metadata keys;
- `tauri inspect wix-upgrade-code` does not resolve to the pinned GUID;
- any task would need signing secrets, install execution, elevation, payload-path ownership, updater behavior or privileged mutation.
