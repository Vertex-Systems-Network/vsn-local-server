# PKG-03 03.15 Development Preflight

Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Task: `03.15`
Linear: `ABD-90`

## Dependency/state check

- 03.06 NSIS current-user lifecycle: DONE
- 03.07 NSIS per-machine lifecycle: DONE
- 03.08 MSI/WiX enterprise lifecycle: DONE
- canonical tracker: 8/25 = 32%
- 03.15 status: READY
- lane: `diagnostics`
- concurrent 03.09 (`desktop`), 03.10 (`payload`) and 03.13 (`boundary`) are independent; 03.15 must not consume their branch-local code or projected results.
- active implementation/planning lanes after starting 03.15: 4, within the frozen maximum of 5.

## Locked inputs

- product: `VSN Dev Platform`
- version: `0.38.1`
- accepted formats: NSIS current-user, NSIS per-machine, MSI/WiX per-machine
- runner: `windows-2025`
- Node: `22.12.0`
- Rust: `1.97.1`
- Tauri CLI evidence version: `2.11.4`
- parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Mutation authority

Planning may change only the 03.15 planning/contract bundle.

After planning gates pass, implementation may add only task-owned:
- `scripts/ci/pkg03-0315-*` validator/lifecycle/diagnostic helpers;
- `.github/workflows/pkg03-0315-*` exact-head certification;
- tracker/master state only after genuine accepted evidence.

No Tauri config, installer template/hook, NSIS toolchain, product payload, service, ACL, firewall/hosts/DNS/trust, signing, updater or recovery mutation is authorized.

The harness may invoke accepted packages with documented diagnostic switches, create task-owned evidence/log files, drive visible installer UI, and inspect installer-created state. It must clean up any successful test installation before completion.
