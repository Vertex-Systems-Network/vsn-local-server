# PKG-03 03.20 Research — Reboot-required, no-restart and pending-reboot semantics

Reviewed: 2026-09-03
Canonical base: `73de463594650cb2ebc407957cbb010e8a0e4be8`
Task: `03.20`
Linear: `ABD-95`
Change required: **false (certification-first)**

## Canonical findings

- Canonical PKG-03 is `19/25 = 76%`; `03.20` is READY because `03.15` and `03.19` are canonically DONE.
- `msiexec /norestart` prevents a device restart after installation. Windows Installer standard-option documentation maps `/norestart` to `REBOOT=ReallySuppress`; the `REBOOT` property documentation says `ReallySuppress` suppresses installer-initiated restart prompts and restarts.
- Windows Installer success code `3010` means the operation succeeded but a restart is required. `1641` means the operation succeeded and initiated a restart. 03.20 must permit an evidence-bound `3010` while rejecting `1641` under the no-restart contract.
- `MsiSystemRebootPending=1` reports a pending file-rename operation detected by Windows Installer. Microsoft explicitly documents that this property does not represent every system condition that may require a reboot, so 03.20 must not turn it into a universal reboot detector.
- Windows Installer 4+ integrates Restart Manager to reduce reboots. 03.19 already certifies the live Desktop/CLI/Agent coordination boundary; 03.20 should reuse that accepted lifecycle under a deterministic pending-reboot signal rather than inventing a second process-coordination implementation.
- A deterministic test signal can be created by preserving the current `PendingFileRenameOperations` value, appending one runner-temp rename pair, exercising the installers, then restoring the exact original value in `finally` cleanup. This is certification state only and must never ship as product behavior.
- Quiet MSI invocation is acceptable only as a control plane for the `/norestart` property/log probe. It cannot claim 03.21 unattended/silent deployment acceptance.

Official references:
- https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec
- https://learn.microsoft.com/en-us/windows/win32/msi/standard-installer-command-line-options
- https://learn.microsoft.com/en-us/windows/win32/msi/reboot
- https://learn.microsoft.com/en-us/windows/win32/msi/system-reboots
- https://learn.microsoft.com/en-us/windows/win32/msi/msisystemrebootpending
- https://learn.microsoft.com/en-us/windows/win32/msi/error-codes
- https://learn.microsoft.com/en-us/windows/win32/msi/using-windows-installer-with-restart-manager

## Certification-first decision

No product/config/template mutation is authorized initially. Build the exact three Windows packages, inject only a bounded runner-side pending-file-rename signal, reuse accepted 03.19 lifecycle evidence, exercise MSI `/norestart`, bind native exit/log/boot-session evidence, restore the exact pending registry state, and fail closed if the generated packages violate the frozen reboot contract.
