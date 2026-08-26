# PKG-03 03.01 — Research

Task: `03.01 — Activate PKG-03 execution authority and freeze Windows installer architecture, format, identity and ownership contract`.

Canonical base: `4606579e07ae57785d1bc1dc12073ea1d036ab4d`.
Package plan: `.ai/plans/pkg03-windows-installer-v1.md` (`sha256:9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`).

## Repository baseline

- Desktop packaging is Tauri v2 with `bundle.active=true` and current product identity `VSN Dev Platform`, version `0.38.1`, identifier `dev.vsn.platform`.
- Accepted local payloads are Desktop, `vsn` CLI and `vsn-agent`; updater/recovery behavior remains PKG-04.
- PKG-03 tracker is frozen at exactly 25 tasks and is dormant before this task.

## Current official-source delta review — 2026-08-26

Reviewed:
- https://v2.tauri.app/distribute/windows-installer/
- https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec
- https://learn.microsoft.com/en-us/windows/win32/rstmgr/about-restart-manager
- https://learn.microsoft.com/en-us/windows/win32/msi/using-windows-installer-with-restart-manager

Findings:
- Tauri v2 still supports Windows NSIS setup executables and MSI/WiX packaging.
- Tauri NSIS still supports current-user, per-machine and user-selectable install modes; per-machine requires elevation.
- Windows Installer still defines quiet/passive, restart-control and logging semantics through `msiexec`.
- Restart Manager remains the Windows mechanism for coordinating files/services in use and reducing reboots.

Material market delta: **none**.

## 03.01 boundary

This task freezes architecture ownership and format boundaries only. It does not pre-implement later tasks:
- `03.02` owns deterministic bundle builds/artifact manifests;
- `03.03` owns publisher/upgrade metadata values;
- `03.04` owns detailed elevation/install-scope behavior;
- `03.05` owns the exact owned-file/resource manifest.

No privileged host mutation, signing secret, updater implementation, Linux/macOS packaging, or pentest work is authorized here.
