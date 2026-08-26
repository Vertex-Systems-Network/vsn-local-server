# PKG-03 Windows Install Scope and Elevation Contract v1

Task authority: `03.04` / Linear `ABD-79`.
Parent package: PKG-03 Windows Installer.

## Purpose

Freeze a least-privilege Windows installation-scope model before real install/uninstall lifecycle tasks begin.

## NSIS current-user boundary

The normal/default VSN NSIS package is a current-user installer:
- Tauri NSIS mode: `currentUser`;
- no Administrator privilege is required by the installer mode;
- installer registration is scoped to HKCU;
- install root is in the current user's LocalAppData class rather than Program Files.

This is the default interactive path and must not silently escalate.

## NSIS per-machine boundary

Machine-wide installation is a separate explicit variant:
- Tauri NSIS mode: `perMachine`;
- Administrator/UAC elevation is required;
- installer registration is scoped to HKLM;
- install root is in the Program Files class.

The per-machine variant is selected by an explicit Tauri config overlay. It is not the default current-user path.

## Why `both` is rejected

Tauri documents that NSIS `both` requires Administrator privileges even when the user ultimately chooses current-user installation. That would cause an avoidable elevation boundary on the least-privilege path, so `both` is not an accepted VSN mode.

## MSI/WiX boundary

Tauri's stock WiX/MSI template is per-machine and current `WixConfig` does not expose a first-class install-scope option. PKG-03 therefore treats MSI as the enterprise/per-machine package family.

03.04 does not authorize a custom WiX template merely to create per-user MSI behavior. A material change to that architecture would require explicit change control.

## Configuration ownership

- Default source: `apps/desktop/src-tauri/tauri.conf.json`.
- Per-machine NSIS overlay: `apps/desktop/src-tauri/tauri.per-machine.conf.json`.
- Existing product identity/publisher/upgrade metadata remains owned by 03.03 and must not change.

## Non-actions

03.04 freezes and certifies configuration behavior only. It does not:
- execute an installer or uninstaller;
- show/respond to a UAC prompt;
- install a Windows service;
- define exact payload ownership paths;
- mutate ACLs, Firewall, hosts, resolver or trust stores;
- sign packages;
- implement updater/recovery behavior.

Those behaviors remain with their frozen downstream tasks.
