# PKG-03 03.15 — Installer diagnostics execution plan v1

Status: frozen task plan
Task: `03.15`
Linear: `ABD-90`
Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Certify operator-observable logging, deterministic supported exit codes, and pre-commit cancellation behavior for the accepted Windows installer formats without changing product packaging semantics or stealing scope from later PKG-03 tasks.

## Acceptance

The exact-head Windows certification must:

1. build current-user NSIS, per-machine NSIS and MSI/WiX from the accepted canonical product inputs;
2. record exact package SHA-256, source SHA, toolchain and runner metadata;
3. prove visible successful NSIS current-user and per-machine setup executions return `0`, then cleanly uninstall;
4. prove genuine NSIS setup cancellation before commit returns documented code `1` and leaves no accepted install/shortcut state;
5. run genuine visible MSI install/uninstall with `/L*V` task evidence logs and require exit `0`;
6. cancel a genuine visible MSI install before commit and require `1602`;
7. preserve each MSI diagnostic log, record its size/SHA-256, and bind it into `evidence.json`;
8. preserve NSIS UI observations/actions and exact exit-code observations as operator diagnostics;
9. prove cancellation tests leave no committed product state and success tests are cleaned up;
10. verify zero tracked repository drift.

## Boundaries

- No silent/passive flags; 03.21 owns unattended deployment.
- No reboot-code assertions; 03.20 owns reboot semantics.
- No repair/reinstall assertions; 03.16 owns repair.
- No mid-transaction forced failure/rollback; 03.18 owns recovery.
- No running-product coordination; 03.19 owns it.
- No custom NSIS template/hooks or special NSIS build.
- No product/Tauri configuration mutation.
- No reliance on concurrent 03.09/03.10/03.13 branch state.

## Evidence

Artifact name: `pkg03-0315-installer-diagnostics`

Required:
- `evidence.json`
- `evidence.json.sha256`
- MSI success/cancel/uninstall verbose logs
- NSIS UI observations/actions
- package hashes and exact source/toolchain metadata
