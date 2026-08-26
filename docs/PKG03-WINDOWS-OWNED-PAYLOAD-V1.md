# PKG-03 Windows Owned Payload and Install-Root Containment v1

Task authority: `03.05` / Linear `ABD-80`.
Parent package: PKG-03 Windows Installer.

## Purpose

Define exactly which durable Windows executable paths the installer may own, before install, uninstall, repair and integrity tasks are allowed to act on them.

## Root model

All owned paths are relative to one logical `${INSTALL_ROOT}` selected by the installer mode. The ownership contract does not hard-code `%LOCALAPPDATA%`, `%ProgramFiles%`, a drive letter or a user profile.

03.04 remains authoritative for current-user versus per-machine scope. 03.05 is authoritative only for paths below the selected root.

## Exact durable executable ownership

The v1 ownership set is exactly:

- `${INSTALL_ROOT}/VSN Dev Platform.exe`
- `${INSTALL_ROOT}/bin/vsn.exe`
- `${INSTALL_ROOT}/bin/vsn-agent.exe`

No wildcard ownership is permitted.

The Desktop executable is already produced by the accepted Tauri bundle lane. CLI and Agent path ownership is reserved here; actual installer placement, discovery and launch are owned by 03.10.

## Explicitly excluded ownership

This contract does not make the installer owner of:
- `apps/updater-helper` or updater/recovery payloads;
- user projects or workspaces;
- mutable user configuration;
- machine/project runtime state;
- database content;
- user-generated or externally managed certificates, keys or credentials;
- arbitrary logs, caches or data outside separately declared installer-owned locations;
- files merely found beneath a directory unless their path is explicitly listed by an accepted ownership revision.

## Canonical path rules

Manifest paths:
- are non-empty relative paths;
- use forward slash `/` only;
- have no empty, `.` or `..` segments;
- have no drive prefix, UNC prefix, device prefix or leading separator;
- contain no `:` alternate-data-stream syntax;
- contain no NUL/control characters;
- have no segment ending in a space or dot;
- do not use Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, including names with extensions);
- are unique under Windows case-insensitive comparison.

## Fail-closed containment

A downstream lifecycle may operate on an owned file only when:
1. the requested relative path exactly matches an accepted manifest entry under case-insensitive Windows comparison;
2. lexical normalization remains beneath `${INSTALL_ROOT}`;
3. the resolved filesystem path does not escape `${INSTALL_ROOT}` through a reparse point/junction/symlink.

Any ambiguity or mismatch is a denial, not an ownership expansion.

## Separation of task authority

03.05 does not install these files. 03.06–03.08 own real installer lifecycles, 03.09 owns Desktop registration/shortcuts, 03.10 owns CLI/Agent placement/discovery/launch, 03.11 owns Agent service registration, 03.12 owns ACL/state separation, 03.14–03.18 own integrity/repair/uninstall behavior, and PKG-04 owns updater/recovery.
