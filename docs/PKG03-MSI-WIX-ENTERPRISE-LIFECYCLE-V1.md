# PKG-03 MSI/WiX Enterprise Lifecycle Contract v1

Task authority: `03.08` / Linear `ABD-83`.
Canonical base: `0ac71c6392c19ad070a9ec442323c46f3c0e08b9`.

## Purpose

Certify the existing stock Tauri/WiX MSI through one real clean per-machine install/Add-Remove-Programs/uninstall lifecycle without changing product packaging semantics or widening into later PKG-03 tasks.

## Configuration and identity contract

- MSI remains stock Tauri WiX Toolset v3 output.
- MSI install scope remains per-machine.
- Product identity remains `VSN Dev Platform` / `0.38.1` / `dev.vsn.platform` / `Vertex Systems Network`.
- WiX UpgradeCode remains `157f304f-1d1b-55e0-b89c-0610ea27c645`.
- `allowDowngrades=false` remains unchanged.
- No custom WiX template or fragment is introduced by 03.08.
- Concrete MSI ProductCode is generated package evidence and must be extracted from the exact built MSI.

## ARP contract

Successful clean install must produce a Windows Installer Add/Remove Programs registration for the exact ProductCode under:

`HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\{ProductCode}`

The record must prove the accepted product name, version and publisher/manufacturer semantics and expose Windows Installer uninstall behavior tied to the same product.

03.08 does not claim that HKCU is globally untouched. The stock Tauri WiX template contains HKCU vendor bookkeeping for install-directory/shortcut components; that does not change the ProductCode-keyed HKLM ARP scope assertion.

## Installed payload boundary

03.08 certifies only the already-packaged Desktop payload:
- Program Files `VSN Dev Platform.exe` must exist after install;
- `bin/vsn.exe` and `bin/vsn-agent.exe` must remain absent until 03.10.

Shortcut creation/registration semantics are not accepted by this task even if stock WiX creates shortcuts incidentally; 03.09 owns those assertions.

## Interactive/default-UI contract

Install and uninstall must use normal Windows Installer UI semantics:
- install starts from `msiexec /i <exact-msi>`;
- uninstall is tied to the exact ProductCode/package;
- `/quiet`, `/passive`, `/qn`, `/qb`, `/qr`, `/qf` and equivalent UI-suppression wrappers are forbidden;
- visible Windows Installer UI must be observed and progressed through normal enabled controls.

Unattended/silent enterprise deployment remains 03.21.

## Clean uninstall contract

After normal visible uninstall:
- the exact ProductCode-keyed HKLM ARP registration is gone;
- installed `VSN Dev Platform.exe` is gone from the clean install root;
- no 03.10 CLI/Agent payload was introduced.

Comprehensive dirty user-data preservation remains 03.17.

## Explicit nonclaims

03.08 does not certify:
- Group Policy, SCCM, Intune or other fleet deployment;
- quiet/passive/no-UI deployment;
- shortcut correctness;
- CLI/Agent placement or launch;
- service lifecycle;
- ACL/data-separation behavior;
- repair/reinstall/rollback/reboot behavior;
- signing;
- updater/recovery.
