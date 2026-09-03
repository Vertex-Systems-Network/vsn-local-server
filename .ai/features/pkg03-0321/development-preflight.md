# PKG-03 03.21 Development Preflight

Canonical base: `3edb4e1dcd2c062e7b2e270cde626c90a2c5459f`
Task: `03.21`
Linear: `ABD-96`

## Dependency/state check

- 03.16 idempotent reinstall/repair: DONE
- 03.17 uninstall cleanup/user-data preservation: DONE
- 03.20 reboot/no-restart semantics: DONE
- canonical tracker: 20/25 = 80%
- 03.21 status: READY
- 03.22 status: READY independently, but production signing configuration is externally blocked
- lane: `automation`
- frozen maximum mutating lanes: 5; 03.21 and 03.22 are independent task surfaces

## Initial mutation authority

Certification-first implementation may add only:
- `.ai/features/pkg03-0321/**`;
- `.ai/plans/pkg03-0321-silent-deployment-v1.md`;
- `.ai/manifests/pkg03-0321-silent-deployment.v1.json`;
- `docs/PKG03-SILENT-DEPLOYMENT-V1.md`;
- `scripts/ci/validate-pkg03-0321.py`;
- `scripts/ci/pkg03-0321-silent-deployment.ps1`;
- `.github/workflows/pkg03-0321-silent-deployment.yml`.

After genuine exact-head acceptance, the same PR may update only the canonical PKG-03 projection files required by repository governance.

Not initially authorized:
- Tauri config, NSIS template/hook, WiX template/hook or product runtime changes;
- service identity/ACL/state-contract changes;
- Authenticode or production signing credentials (03.22);
- SBOM/provenance/PKG-05 release work (03.23);
- PKG-04 updater/recovery work.

## Fail-closed rule

If `/S` or `/quiet` blocks for interaction, emits an installer-family visible titled window, fails accepted state/service/cleanup semantics, initiates a reboot, or returns an unexpected native code, preserve exact diagnostics and classify the failure before authorizing any product mutation. Never add automated button clicking to manufacture a silent pass.
