# PKG-03 03.08 — Research

Task: `03.08 — MSI/WiX enterprise install, uninstall and Add/Remove Programs lifecycle`
Linear: `ABD-83`
Canonical base reviewed: `0ac71c6392c19ad070a9ec442323c46f3c0e08b9`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`
Reviewed: 2026-08-27

## Repository authority

- Canonical PKG-03 is `7/25 = 28%`; `03.01–03.07=DONE`; deterministic cursor is `03.08`.
- `03.08–03.10` are dependency-ready Wave 2 tasks. This branch owns only `03.08`.
- 03.08 depends on canonical 03.02 bundle determinism, 03.03 identity/version/publisher/UpgradeCode metadata, 03.04 MSI per-machine scope, and 03.05 ownership/containment.
- Current product config already enables the stock Tauri Windows bundle target and WiX `upgradeCode = 157f304f-1d1b-55e0-b89c-0610ea27c645`.
- 03.08 does not own shortcut/application-registration semantics (03.09), CLI/Agent placement (03.10), service lifecycle (03.11), ACL/data separation (03.12), repair/integrity (03.14/03.16), rollback (03.18), unattended/silent deployment (03.21), signing (03.22), or updater/recovery (PKG-04).

## Current Tauri / Windows Installer review

Official Tauri v2 and Microsoft Windows Installer documentation was rechecked on 2026-08-27.

- Tauri v2 builds Windows `.msi` packages with WiX Toolset v3, and MSI creation requires Windows.
- The stock Tauri WiX template declares `InstallScope="perMachine"` and installs under the architecture-appropriate Program Files folder.
- Windows Installer `msiexec /i <package>` performs a normal install and `/x <package-or-ProductCode>` performs uninstall.
- Microsoft documents `/quiet`, `/passive`, and `/q...` as reduced/no-UI deployment options. Those are intentionally outside 03.08 because unattended/silent deployment is task 03.21.
- For per-machine Windows Installer products, Add/Remove Programs metadata is registered for all users. Microsoft maps Windows Installer product metadata to `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\{ProductCode}`.
- The stock Tauri WiX source also uses HKCU vendor bookkeeping for install-directory and shortcut components. Therefore 03.08 must not make a blanket claim that HKCU is untouched; the scope claim is specifically that the Windows Installer ARP product registration is the ProductCode-keyed HKLM entry.

Primary sources:
- https://v2.tauri.app/distribute/windows-installer/
- https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec
- https://learn.microsoft.com/en-us/windows/win32/msi/configuring-add-remove-programs-with-windows-installer
- https://learn.microsoft.com/en-us/windows/win32/msi/uninstall-registry-key
- upstream stock WiX template: `tauri-apps/tauri` `crates/tauri-bundler/src/bundle/windows/msi/main.wxs`

## Identity and registration model

03.03 remains authoritative for:
- product name `VSN Dev Platform`;
- version `0.38.1`;
- identifier `dev.vsn.platform`;
- manufacturer/publisher `Vertex Systems Network`;
- stable WiX UpgradeCode `157f304f-1d1b-55e0-b89c-0610ea27c645`;
- `allowDowngrades=false`.

The MSI `ProductCode` is generated for the concrete package and is evidence, not a new source-controlled identity field. 03.08 must extract the built MSI ProductCode at runtime and bind all ARP/uninstall assertions to that exact value.

## Decision

`change_required=false`.

No product or Tauri configuration change is required. 03.08 is a certification task around the already-accepted stock Tauri/WiX MSI output.

## Genuine 03.08 evidence model

Exact-head GitHub-hosted `windows-2025` evidence must:

1. build only the MSI target from the locked source/toolchain without modifying Tauri configuration or introducing a custom WiX template;
2. identify exactly one produced `.msi`, record its path, size and SHA-256, and extract ProductCode, ProductVersion, ProductName and Manufacturer from the package;
3. prove ProductName/version/manufacturer match the frozen source identity and ProductCode is a valid GUID;
4. record the runner Administrator/elevated/high-integrity state before per-machine execution;
5. capture clean pre-state for the expected Program Files install root and exact ProductCode-keyed HKLM ARP entry;
6. launch a normal full/default-UI MSI install through `msiexec /i <package>` without `/quiet`, `/passive`, `/qn`, `/qb`, `/qr`, `/qf`, or other silent/passive wrappers;
7. observe and progress the visible Windows Installer UI rather than bypassing installer logic;
8. prove successful install into Program Files and presence of `VSN Dev Platform.exe`;
9. prove the exact ProductCode-keyed HKLM ARP record exists and matches expected DisplayName, DisplayVersion and Publisher/Manufacturer semantics;
10. prove the ARP entry exposes a Windows Installer uninstall path tied to the same ProductCode;
11. prove `bin/vsn.exe` and `bin/vsn-agent.exe` remain absent, preserving 03.10 authority;
12. uninstall the same product through a normal visible Windows Installer UI path tied to the exact ProductCode/package, without silent/passive flags;
13. prove the exact HKLM ARP ProductCode key and installed Desktop executable disappear after uninstall;
14. prove zero tracked repository drift and emit exact-source machine-readable evidence.

## Evidence boundary

03.08 certifies one clean stock MSI/WiX per-machine install/ARP/uninstall lifecycle on the exact GitHub-hosted Windows environment. It does not certify:
- silent/passive enterprise deployment;
- Group Policy/SCCM/Intune distribution;
- repair/reinstall/upgrade/downgrade behavior beyond preserving the frozen metadata;
- shortcut correctness;
- CLI/Agent placement;
- service/ACL behavior;
- dirty-data preservation;
- signing;
- updater/recovery.
