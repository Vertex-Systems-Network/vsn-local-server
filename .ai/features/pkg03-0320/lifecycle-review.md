# PKG-03 03.20 Lifecycle Review — Reboot semantics

Reviewed: 2026-09-03
Canonical base: `73de463594650cb2ebc407957cbb010e8a0e4be8`
Task: `03.20`
Linear: `ABD-95`

## Matrix

| Surface | Pending signal | Invocation | Required outcome |
| --- | --- | --- | --- |
| current-user NSIS | deterministic `PendingFileRenameOperations` pair | accepted visible 03.19 lifecycle | no boot-session change; safe coordination/block contract preserved |
| per-machine NSIS | same | accepted visible 03.19 lifecycle | no boot-session change; service/package state coherent |
| MSI/WiX interactive | same | accepted visible 03.19 lifecycle | no boot-session change; Restart Manager semantics remain coherent |
| MSI/WiX no-restart control | same | `/qn /norestart /L*V` install + uninstall | `REBOOT=ReallySuppress`; native exit only 0/3010; 1641 forbidden |

## Pending-reboot probe

1. Capture whether `PendingFileRenameOperations` exists, its registry kind and exact ordered string-array value.
2. Require the existing kind to be `MultiString` when present.
3. Create runner-temp source/destination paths and append exactly one pending rename pair after every pre-existing entry.
4. While certification is active, require the original entries plus the injected pair to remain an ordered prefix. Additional Windows Installer entries may appear and are evidence, not an excuse to destroy the original signal.
5. In a `finally` block restore the exact original registry state: remove the value if originally absent, otherwise restore its exact original ordered strings.

## Reboot contract

- Boot identity is `Win32_OperatingSystem.LastBootUpTime`; it must remain identical from preflight through cleanup.
- MSI `/norestart` is the tested no-restart contract and must yield log evidence for `ReallySuppress`.
- Exit `0` = success without a reboot-required return.
- Exit `3010` = successful operation that reports reboot required; accepted only while the boot identity remains unchanged.
- Exit `1641` = reboot initiated; always fails 03.20.
- `MsiSystemRebootPending=1` must be observed for the injected pending-file-rename signal, but it is explicitly not treated as a universal reboot-condition detector.

## Boundaries

- Reuse 03.19 running-resource coordination; do not weaken its no-pre-kill/protected-state rules.
- `/qn` here is a test control plane only; it does not certify 03.21 silent deployment.
- No product/config/template/service/ACL/signing/provenance/updater mutation is authorized by this review.
