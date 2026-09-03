# PKG-03 03.20 — Installer Reboot Semantics Contract v1

Status: frozen lifecycle contract  
Task: `03.20`  
Linear: `ABD-95`  
Canonical base: `73de463594650cb2ebc407957cbb010e8a0e4be8`

## Purpose

Certify reboot-required, no-restart and pending-reboot behavior for the accepted Windows installer candidates without expanding into 03.21 unattended deployment or changing product/installer implementation before evidence requires it.

## Required behavior

1. Build the exact current-user NSIS, per-machine NSIS and MSI/WiX candidates from the task source head.
2. Record the Windows boot-session identity before installer activity and require the same identity after every tested operation.
3. Snapshot `PendingFileRenameOperations` exactly, inject one deterministic test-only pending-rename pair while preserving any pre-existing entries, and restore the original registry value exactly at cleanup.
4. Exercise the accepted 03.19 visible running-resource lifecycle while that pending-reboot signal exists. The signal must not cause an unexpected reboot, partial package state, silent force termination or tracked repository drift.
5. Exercise MSI install and uninstall with `/norestart` and verbose logging. Accepted native exit codes are `0` and `3010`; `1641` is forbidden because it represents a reboot initiated by Windows Installer.
6. Evidence must bind `/norestart` to `REBOOT=ReallySuppress` behavior and observe `MsiSystemRebootPending=1` for the synthetic pending-file-rename condition.
7. `MsiSystemRebootPending` is treated only as the documented pending-file-rename signal, never as a universal detector for every Windows reboot condition.
8. Pending-reboot test state is runner-only evidence state. No product, installer template, service, ACL, signing, provenance, updater or cross-platform behavior may be mutated by this certification-first task.

## Nonclaims

- Quiet MSI invocation used to isolate reboot-control semantics does **not** certify 03.21 silent/unattended deployment.
- 03.20 does not create or configure production signing credentials.
- 03.20 does not implement PKG-04 updater/recovery or PKG-05 release handoff.
- A successful no-restart probe does not claim that every future Windows reboot cause is represented by `MsiSystemRebootPending`.

## Evidence

The exact-head artifact is `pkg03-0320-reboot-semantics`. It must include package hashes, boot identity, injected/restored pending state, inherited 03.19 lifecycle evidence, MSI verbose logs, native exit codes, and zero tracked repository drift.
