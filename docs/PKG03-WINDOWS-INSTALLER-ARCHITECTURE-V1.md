# PKG-03 Windows Installer Architecture Contract v1

Task authority: `03.01` / Linear `ABD-76`.
Parent package: PKG-03 Windows Installer.

## Purpose

Freeze the Windows installer architecture boundary before any bundle implementation begins.

## Packaging boundary

VSN Windows distribution is authored through the existing Tauri v2 Desktop packaging boundary. PKG-03 supports two package families:

- NSIS setup executable;
- MSI produced through Tauri's Windows MSI/WiX path.

No third custom installer framework is authorized by this contract.

## Identity boundary

Canonical application identity is sourced from `apps/desktop/src-tauri/tauri.conf.json`.

Observed task-base values:
- product name: `VSN Dev Platform`;
- version: `0.38.1`;
- application identifier: `dev.vsn.platform`.

03.01 does not independently invent publisher, upgrade code/GUID or channel metadata. Those values are owned by 03.03 and must remain consistent with this identity source.

## Ownership boundary

Installer-owned artifact classes may include, when introduced by their owning tasks:
- packaged Desktop application binaries/resources;
- packaged `vsn` CLI and `vsn-agent` binaries;
- application registration and shortcuts;
- Windows service registration metadata for the VSN Agent;
- installer/package metadata, logs or repair records explicitly declared as owned.

The following are not installer-owned by default:
- user projects/workspaces;
- mutable user configuration;
- machine/project runtime state;
- database content;
- logs or data outside declared installer-owned locations;
- user-generated or externally managed certificates/credentials.

Exact path-level ownership is frozen by 03.05 before install/uninstall cleanup acceptance.

## Privilege boundary

Current-user installation is expected to remain non-elevated where supported. Per-machine installation requires an explicit elevated boundary. Detailed mode selection, UAC behavior and path rules belong to 03.04.

## Host mutation boundary

Installation must not silently modify:
- Windows Firewall rules;
- hosts file;
- DNS resolver configuration;
- root/intermediate trust stores.

Any future need for such mutation requires separately approved change control.

## Update and signing boundary

PKG-04 owns updater/apply/rollback orchestration. PKG-03 may prepare installed payloads for later update compatibility but does not implement automatic update behavior.

Authenticode integration is owned by 03.22. Signing secrets/private keys are never repository content or evidence payloads.

## Running-process boundary

Windows Installer/Restart Manager semantics are the future coordination baseline for files/services in use. Exact running-process and reboot behavior is accepted by 03.19 and 03.20.

## Downstream ownership map

- 03.02 — deterministic Windows build/artifact manifest;
- 03.03 — publisher/version/upgrade metadata;
- 03.04 — install scope/elevation;
- 03.05 — exact payload/resource ownership manifest;
- 03.06–03.08 — real NSIS/MSI lifecycle;
- 03.11 — Agent Windows service lifecycle;
- 03.22 — Authenticode verification;
- 03.24–03.25 — final Windows acceptance.

This contract is architecture authority only and does not claim those downstream acceptance results.
