# PKG-03 03.11 — Agent Service Lifecycle Contract v2

Status: planning contract; implementation acceptance is not yet claimed.

## Frozen service contract

| Property | Required value |
|---|---|
| SCM name | `VSN-Agent` |
| Display name | `VSN Agent` |
| Account | `NT AUTHORITY\LocalService` |
| Start mode | Automatic |
| Binary | `<install-root>\bin\vsn-agent.exe --service-run` |
| Health | installed `<install-root>\bin\vsn.exe ping` exits 0 |
| Current-user NSIS | service absent / no machine-service mutation |
| Per-machine NSIS | install + start + health + stop/start + remove |
| MSI/WiX | install + start + health + stop/start + remove |

## Ownership

PKG-03 03.10 is the sole file owner of `bin/vsn-agent.exe`. 03.11 owns only installer service-lifecycle integration and certification. A second `File`/Component for the Agent is prohibited.

## Ordering

Install actions occur only after the Agent file exists. Uninstall service stop/removal occurs while the Agent file still exists and before owned file deletion.

## Security boundary

Only elevated/per-machine installer paths may mutate the machine SCM. Service identity is fixed and least-privilege account remains LocalService. No external secrets are introduced.

## Evidence boundary

A successful installer build is not acceptance. Exact-head Windows evidence must prove service configuration, RUNNING state, CLI health, bounded stop/start, uninstall removal, current-user non-registration, payload cleanup and zero tracked drift.

## Explicit nonclaims

03.11 does not certify ACL/data separation (03.12), protected Windows non-mutation (03.13), integrity/repair (03.14/03.16), general installer diagnostics policy (03.15), rollback/recovery (03.18), Restart Manager/runtime coordination (03.19), reboot semantics (03.20), unattended deployment (03.21), signing (03.22), or PKG-04 updater/recovery.
