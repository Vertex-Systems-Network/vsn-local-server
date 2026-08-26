# PKG-03 03.04 — Research

Task: `03.04 — Install-scope and elevation contract for per-user and per-machine modes`
Linear: `ABD-79`
Canonical base reviewed: `8f2919923005ba29b1475bd646a3f6953100ca9e`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`
Reviewed: 2026-08-26

## Repository authority

- PKG-03 architecture authority requires current-user installation to remain non-elevated where supported and per-machine installation to cross an explicit elevated boundary.
- Canonical 03.01, 03.02 and 03.03 are DONE at this base; 03.04 and 03.05 are READY, with deterministic cursor 03.04.
- 03.04 owns install-scope/elevation policy only. Exact installed payload paths are 03.05; install/uninstall execution is 03.06–03.08; service lifecycle is 03.11; ACL/state separation is 03.12.
- Existing package identity remains `VSN Dev Platform` / `0.38.1` / `dev.vsn.platform`, publisher `Vertex Systems Network`, WiX UpgradeCode `157f304f-1d1b-55e0-b89c-0610ea27c645`.

## Current platform review

Official Tauri v2 and Microsoft Windows Installer sources were rechecked on 2026-08-26.

- Tauri NSIS `currentUser` installs to a location that does not require Administrator access and records installer metadata under HKCU.
- Tauri NSIS `perMachine` installs under Program Files, requires Administrator access and records installer metadata under HKLM.
- Tauri NSIS `both` allows a choice but requires Administrator privileges even when the user selects current-user mode, so it violates the least-privilege default required by the architecture contract.
- Tauri supports config overlays with `--config`; later lifecycle tasks can therefore build an explicit per-machine NSIS variant without making every current-user installation elevated.
- Tauri's stock WiX/MSI template is per-machine; current Tauri `WixConfig` exposes no first-class install-scope option. MSI is therefore frozen as the enterprise/per-machine package family unless a later approved change replaces the template.
- Microsoft Windows Installer distinguishes per-user and per-machine contexts and redirects registration/folder state accordingly; per-machine context can require elevation for standard users.

Primary sources:
- https://v2.tauri.app/distribute/windows-installer/
- https://v2.tauri.app/reference/config/
- https://learn.microsoft.com/en-us/windows/win32/msi/installation-context
- https://learn.microsoft.com/en-us/windows/win32/msi/allusers
- https://learn.microsoft.com/en-us/windows/win32/msi/msiinstallperuser
- https://github.com/tauri-apps/tauri/issues/13792

## Decision

`change_required=false`.

Freeze the least-privilege scope model:
- default NSIS mode: `currentUser`;
- explicit machine NSIS overlay: `perMachine`;
- forbid `both` in the accepted VSN installer contract because it elevates even a current-user choice;
- MSI/WiX remains per-machine/elevated under the stock Tauri template;
- no installer execution, UAC prompt, service registration, ACL mutation, payload-path ownership, signing or updater behavior is authorized by 03.04.
