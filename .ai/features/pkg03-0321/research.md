# PKG-03 03.21 Research — unattended and silent deployment

Reviewed: 2026-09-03
Canonical base: `3edb4e1dcd2c062e7b2e270cde626c90a2c5459f`
Task: `03.21`
Linear: `ABD-96`
Change required: **false (certification-first)**

## Canonical findings

- Canonical PKG-03 is `20/25 = 80%`; `03.21` is READY because `03.16`, `03.17`, and `03.20` are canonically DONE.
- Tauri v2 documents that its NSIS `-setup.exe` supports silent installation with uppercase `/S`.
- NSIS documents `/S` as the case-sensitive silent installer/uninstaller switch. Silent mode can still be defeated by script-authored interaction, so acceptance must prove bounded completion with no automation/input and no observed installer-family visible windows.
- Microsoft Windows Installer defines `/quiet` as no-user-interaction mode and `/qn` as its no-UI equivalent. `/passive` intentionally shows progress and is therefore not part of the strict silent acceptance surface.
- Microsoft documents `/norestart` separately. 03.20 already certifies the exact generated MSI mapping to `REBOOT=ReallySuppress`, accepts `0`/`3010`, and forbids reboot-initiated `1641`; 03.21 must preserve that rule during unattended execution rather than create a second reboot contract.
- Current accepted packages contain Desktop, CLI and Agent payloads. Current-user NSIS must keep the machine service absent; per-machine NSIS and MSI must preserve the accepted `VSN-Agent` service lifecycle.
- 03.16 repair/reinstall and 03.17 cleanup/user-data preservation are dependencies. 03.21 must not add destructive data flags or interactive repair choices merely to make silent execution pass.
- 03.19 owns running-resource coordination. 03.21 may stop the accepted service before its uninstall probe to isolate silent command-line behavior; it must not weaken or supersede the 03.19 running-process contract.
- Initial work is certification-only. A product/installer change is authorized only after exact-head evidence proves the generated candidates cannot satisfy the frozen silent contract.

Official references:
- https://v2.tauri.app/distribute/microsoft-store/
- https://nsis.sourceforge.io/Which_command_line_parameters_can_be_used_to_configure_installers
- https://nsis.sourceforge.io/Reference/SilentInstall
- https://nsis.sourceforge.io/Reference/SilentUnInstall
- https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec
- https://learn.microsoft.com/en-us/windows/win32/msi/standard-installer-command-line-options

## Certification-first decision

Build the exact current-user NSIS, per-machine NSIS and MSI/WiX candidates from the source head. Exercise only their documented silent entry points with no UI automation or user input, require bounded native completion, bind install/service/registration/uninstall state to the same source SHA, preserve `/norestart`, clean each lane, and fail closed before authorizing installer mutation.
