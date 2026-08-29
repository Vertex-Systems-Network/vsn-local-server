# PKG-03 03.13 Development Preflight

Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Task: `03.13`
Linear: `ABD-88`

## Dependency/state check

- 03.06 NSIS current-user lifecycle: DONE
- 03.07 NSIS per-machine lifecycle: DONE
- 03.08 MSI/WiX enterprise lifecycle: DONE
- canonical tracker: 8/25 = 32%
- 03.13 status: READY
- lane: `boundary`
- concurrent 03.09 (`desktop`) and 03.10 (`payload`) are independent; 03.13 must not consume their branch-local code or projected results.

## Locked inputs

- product: `VSN Dev Platform`
- version: `0.38.1`
- accepted installer modes: NSIS current-user, NSIS per-machine, MSI/WiX per-machine
- runner: `windows-2025`
- Node: `22.12.0`
- Rust: `1.97.1`
- Tauri CLI evidence version: `2.11.4`
- parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Mutation authority

Planning may change only the 03.13 planning/contract bundle.

After planning gates pass, implementation may add only task-owned:
- `scripts/ci/pkg03-0313-*` snapshot/validator/lifecycle helpers;
- `.github/workflows/pkg03-0313-*` exact-head certification;
- tracker/master state only after genuine accepted evidence.

03.13 has **read-only system authority** for firewall, hosts, resolver and certificate-store surfaces. It may not add/remove firewall rules, edit hosts, change DNS/NRPT, import/remove certificates, register services, alter ACLs, sign packages, or touch updater/recovery behavior.

No product/Tauri/installer-template mutation is authorized by this task.
