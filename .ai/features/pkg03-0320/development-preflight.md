# PKG-03 03.20 Development Preflight

Canonical base: `73de463594650cb2ebc407957cbb010e8a0e4be8`
Task: `03.20`
Linear: `ABD-95`

## Dependency/state check

- 03.15 installer diagnostics/exit semantics: DONE
- 03.19 running-process/Restart Manager coordination: DONE
- canonical tracker: 19/25 = 76%
- 03.20 status: READY
- 03.22 status: READY independently
- lane: `reboot`
- frozen maximum mutating lanes: 5; current independent write lanes are 03.20 and 03.22

## Initial mutation authority

Certification-first implementation may add only:
- `.ai/features/pkg03-0320/**`;
- `.ai/plans/pkg03-0320-reboot-semantics-v1.md`;
- `.ai/manifests/pkg03-0320-reboot-semantics.v1.json`;
- `docs/PKG03-INSTALLER-REBOOT-SEMANTICS-V1.md`;
- `scripts/ci/validate-pkg03-0320.py`;
- `scripts/ci/pkg03-0320-reboot-semantics.ps1`;
- `.github/workflows/pkg03-0320-reboot-semantics.yml`.

Not initially authorized:
- Tauri config or installer template/hook changes;
- product runtime/service/ACL changes;
- silent deployment acceptance (03.21);
- Authenticode/signing-secret changes (03.22);
- SBOM/provenance/PKG-05 implementation (03.23);
- updater/recovery or later-package work.

## Fail-closed rule

If exact generated installers reboot the runner, initiate restart code 1641, lose the injected/pre-existing pending marker, corrupt package state, or fail to expose the documented no-restart/pending semantics, retain exact diagnostics and classify the failure before authorizing any product mutation. Do not weaken the acceptance contract to manufacture a pass.
