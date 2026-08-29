# PKG-03 03.11 — VSN Agent Windows Service Install Lifecycle Plan v2

## Goal

Complete the already-planned installer integration for the existing `VSN-Agent` runtime without changing the Agent runtime itself: current-user NSIS must remain service-neutral; elevated/per-machine NSIS and MSI/WiX must install/start/health-check/stop/restart/remove the existing service using the single Agent payload owned by 03.10.

## Preserved authorization

This is a Governance V3 migration and fresh-base reconciliation of the already-authorized 03.11 scope from stale PR #125. It does not add new product scope. Canonical PKG-03 continues to mark 03.11 READY with dependencies 03.07 and 03.10 DONE.

## Acceptance sequence

1. Bind planning to refreshed canonical main and the unchanged PKG-03 parent-plan digest.
2. Verify current Agent service invariants and 03.10 single-payload ownership.
3. Add only Tauri-supported extension wiring:
   - NSIS `installerHooks`;
   - WiX `fragmentPaths` plus the minimum required fragment reference.
4. NSIS current-user lifecycle must never mutate machine service state.
5. NSIS per-machine post-install must invoke installed Agent service install/start and fail on non-zero result.
6. NSIS per-machine pre-uninstall must bounded-stop/remove the service before Agent payload deletion.
7. WiX must invoke the already-installed Agent through a task-owned fragment; no second Agent `File`/Component is allowed.
8. MSI service custom actions must be deferred, elevated/non-impersonated, synchronous and return-checked.
9. Build current-user NSIS, per-machine NSIS and MSI/WiX with locked inputs.
10. Certify exact service config: name, display name, automatic start, LocalService account, image path + `--service-run`.
11. Certify RUNNING health with installed CLI ping; bounded stop/start; second health pass.
12. Certify uninstall service removal and owned payload cleanup.
13. Certify zero duplicate Agent ownership and zero tracked repository drift.
14. Bind exact-head evidence to source SHA/run/job/artifact and installer hashes.
15. Only after genuine acceptance, project 03.11 DONE to tracker/master/README from refreshed live main.

## Implementation design

### NSIS
Use `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh` via `bundle.windows.nsis.installerHooks`. Machine-service actions are permitted only for per-machine installer mode.

### MSI/WiX
Use `apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs` via `bundle.windows.wix.fragmentPaths`.

The fragment must use the existing `[INSTALLDIR]bin\vsn-agent.exe` and must not own/copy that file. It schedules:
- install service command after files exist, initial install only;
- start service after install command;
- stop service during full uninstall while the Agent binary still exists;
- remove service after stop and before file removal.

If Tauri CLI 2.11.4 cannot compile/link this bounded fragment without unrelated component/template mutation, stop for change control.

## Expected changed implementation surfaces

- `apps/desktop/src-tauri/tauri.windows.conf.json`
- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`
- `apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs`
- `scripts/ci/pkg03-0311-agent-service-lifecycle.ps1`
- `scripts/ci/validate-pkg03-0311.py`
- `.github/workflows/pkg03-0311-agent-service-lifecycle.yml`

State projection files are not implementation surfaces and may change only after exact-head acceptance.

## Non-goals

No runtime refactor, second service architecture, ACL/state/config change, protected Windows boundary mutation, repair/rollback/reboot behavior, silent deployment, signing, updater, recovery, or full installer-template fork.
