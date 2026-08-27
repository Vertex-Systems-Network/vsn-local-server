# PKG-03 03.15 Lifecycle Review — Installer diagnostics

Reviewed: 2026-08-27
Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Task: `03.15`
Linear: `ABD-90`

## Lifecycle matrix

| Format / operation | UI | Expected exit | Persistent diagnostic | State assertion |
| --- | --- | ---: | --- | --- |
| NSIS current-user install success | visible | `0` | task UI observations/actions | accepted current-user install state |
| NSIS per-machine install success | visible/elevated | `0` | task UI observations/actions | accepted per-machine install state |
| NSIS setup cancellation before commit | visible | `1` | task UI observations/actions | clean/no installation residue |
| MSI/WiX install success | visible | `0` | `/L*V` log | accepted machine/ARP state |
| MSI/WiX uninstall success | visible | `0` | `/L*V` log | owned install/ARP state removed |
| MSI/WiX install cancellation before commit | visible | `1602` | `/L*V` log | clean/no committed product state |

## Cancellation boundary

Cancellation is exercised only before the installer reaches a committed install state. This task proves user-cancel signalling and operator diagnostics, not transactional rollback after partial execution; 03.18 owns failure rollback/interrupted-install recovery.

For NSIS, the setup executable's documented user-abort code `1` is certified. No deterministic cancellation exit-code claim is made for the generated uninstaller because stock NSIS self-copies before execution and does not reliably propagate the inner uninstaller's error level to the original process.

## Logging boundary

MSI logging uses Windows Installer's native `/L*V` switch. The harness creates the evidence directory first, records log size/SHA-256 and retains the exact file in the workflow artifact.

Stock NSIS persistent install logging is not enabled by default and requires a specially compiled NSIS with `NSIS_CONFIG_LOG`. 03.15 therefore records native UI/control/action diagnostics and process exit codes for NSIS rather than changing the packaging toolchain or claiming a native persistent NSIS log that is not present.

## Nonclaims

03.15 does not certify:
- quiet/passive/silent deployment;
- reboot-required/no-restart behavior;
- repair/reinstall;
- interrupted-install rollback;
- running-process/Restart Manager coordination;
- signing, updater or recovery behavior;
- custom installer templates or a new logging subsystem.
