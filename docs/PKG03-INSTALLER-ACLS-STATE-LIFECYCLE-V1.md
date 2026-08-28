# PKG-03 03.12 — Installer ACL / State Lifecycle Contract v1

This document is the durable operator-facing contract for 03.12.

## Storage classes

| Class | Authority | Purpose | Installer ownership |
|---|---|---|---|
| Install payload | selected VSN install root | immutable Desktop/CLI/Agent payload | yes, only frozen owned files |
| Shared IPC security | `%PROGRAMDATA%\VSN\security\ipc.key` | machine-shared authenticated IPC secret | runtime security authority; installer may only support compatible directory integration |
| Mutable data | `ProjectDirs(...).data_local_dir()` | audit, runtimes, managed state, VSN data | no executable payload ownership |
| Config | `ProjectDirs(...).config_dir()/config.json` | VSN configuration | no executable payload ownership |

## Windows IPC ACL

Directory: inheritance disabled; SYSTEM/Admins Full Control; LocalService Read; creating/current SID Full Control.

Secret file: inheritance disabled; SYSTEM/Admins Full Control; LocalService Read; creating/current SID Read.

The existing `vsn-security` code is authoritative. 03.12 may not weaken, duplicate or fork it.

## Context rule

ProjectDirs is resolved by the executing identity. The Agent service executes as LocalService, so service-resolved data/config paths must be measured from that context. An interactive user's LocalAppData/config path is not a valid substitute.

## Package-mode rules

- **Current-user NSIS:** package install/uninstall alone must not create machine-wide IPC security state.
- **Per-machine NSIS:** accepted service/runtime may create/use ProgramData IPC state; ACLs and separation must be verified.
- **MSI/WiX:** same machine-shared security contract and separation; no duplicate payload ownership or full-template fork.

## Uninstall boundary

03.12 establishes classification and non-destructive boundaries. Comprehensive dirty-data preservation/cleanup remains task 03.17.

## Evidence

Exact-head evidence must include source SHA/run/job/artifact, package hashes, observed install roots, observed service account/state, ProgramData path, ACL entries by SID, resolved data/config roots, current-user negative state, per-machine NSIS/MSI assertions, 03.10/03.11 regressions and zero tracked drift.
