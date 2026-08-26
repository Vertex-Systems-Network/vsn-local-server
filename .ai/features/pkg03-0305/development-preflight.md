# PKG-03 03.05 — Development Preflight

Task: `03.05`
Linear: `ABD-80`
Canonical base: `7cd671de8af410ee348083c42c716cce1dd22543`
Parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Authority check

- Parent PKG-03 plan is frozen.
- 03.01–03.04 are DONE on canonical main.
- 03.05 is Wave 1 / ownership lane and dependency-ready.
- Canonical pre-state is `4/25 = 16%`, 03.05 READY, cursor 03.05.
- Linear ABD-80 is the task mirror.
- No self-expansion into 03.06–03.12, 03.14–03.18, 03.22 or PKG-04 is permitted.

## Planned product-contract surface

After the planning head passes required governance gates:

- `installer/windows/owned-payload.v1.json`
  - exactly three durable executable entries;
  - exact root-relative paths:
    - `VSN Dev Platform.exe`
    - `bin/vsn.exe`
    - `bin/vsn-agent.exe`
  - explicit source authority and downstream placement-owner metadata;
  - fail-closed Windows path-containment policy;
  - explicit excluded mutable/user/updater classes.

No Tauri configuration or installer template mutation is planned by 03.05.

## Certification surface

- `.ai/features/pkg03-0305/*`
- `.ai/plans/pkg03-0305-owned-payload-v1.md`
- `.ai/manifests/pkg03-0305-owned-payload.v1.json`
- `docs/PKG03-WINDOWS-OWNED-PAYLOAD-V1.md`
- `installer/windows/owned-payload.v1.json`
- `scripts/ci/validate-pkg03-0305.py`
- `.github/workflows/pkg03-0305-owned-payload.yml`

## Required exact-head gates

- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence
- PKG-03 03.05 Owned Payload + Containment

## Stop conditions

Stop rather than widen scope if:
- parent-plan bytes drift;
- canonical 03.01–03.04 are not DONE;
- product identity/install-scope metadata drifts;
- an owned path requires an absolute machine/user-specific root;
- the manifest would need updater-helper or mutable user/state data ownership;
- actual CLI/Agent installer placement is required to satisfy this task;
- any task would require install/uninstall execution, service registration, ACL mutation, signing secrets, updater behavior or privileged mutation.
