# PKG-03 03.11 Lifecycle / Architecture / Security / QA Review v2

## Purpose

Certify installer ownership of the already-existing production `VSN-Agent` Windows service using the Agent payload already owned by PKG-03 03.10.

## Architecture

### Existing immutable inputs
- Agent runtime: `apps/agent/src/main.rs`
- service-control core: `crates/vsn-system/src/lib.rs`
- staged Agent destination: `bin/vsn-agent.exe`
- staged CLI destination: `bin/vsn.exe`
- per-machine overlay: `apps/desktop/src-tauri/tauri.per-machine.conf.json`
- Windows payload overlay: `apps/desktop/src-tauri/tauri.windows.conf.json`

### Service identity
- SCM name: `VSN-Agent`
- display name: `VSN Agent`
- executable: installed `<install-root>\bin\vsn-agent.exe --service-run`
- account: `NT AUTHORITY\LocalService`
- start type: automatic
- accepted controls: START/STOP/query/remove through the Agent's existing service interface
- health: installed `bin\vsn.exe ping`

### Integration design
- NSIS: supported Tauri installer hook file only.
- WiX: supported Tauri fragment only; deferred custom actions call the installed Agent service interface.
- No installer owns a second copy of the Agent executable.

## Data flow

Install:
`installer -> installed vsn-agent.exe -> Windows SCM -> VSN-Agent process -> authenticated local IPC -> installed vsn.exe ping`

Uninstall:
`installer -> installed vsn-agent.exe service stop -> service uninstall -> SCM absence -> normal payload removal`

No remote network, external account, user-content, database, firewall, resolver, hosts, trust-store, or secret-material flow is introduced.

## Security

- Machine service mutation is allowed only in elevated/per-machine paths.
- Current-user NSIS must never create/start/stop/delete `VSN-Agent`.
- Service runs as `NT AUTHORITY\LocalService`, not LocalSystem.
- Installer integration delegates service creation to the accepted Agent command and does not construct a second privileged service implementation.
- MSI custom actions must be deferred, non-impersonated and return-checked.
- No signing secrets or external credentials are introduced.
- A service/image-path/account mismatch is a failed acceptance, not an auto-repair opportunity.

## Installer UI / accessibility

No new product UI is introduced. Existing visible NSIS/MSI installer flows remain unchanged; no new dialog, control, copy, responsive layout, or accessibility surface is added by 03.11.

## Failure handling

- non-zero install/start custom action fails the affected installer lifecycle;
- certification harness performs bounded cleanup after failed evidence runs;
- 03.11 does not claim transactional rollback/interrupted-install recovery (03.18);
- inability to express the WiX integration safely through stock fragment extension points is a `STOP_AND_REASSESS` condition.

## Performance

Service transitions and health waits are bounded. No persistent polling loop or background installer process may be left behind. Acceptance uses deterministic timeouts for SCM state transitions and CLI ping.

## QA / acceptance matrix

1. Current-user NSIS install/uninstall: Agent payload exists during install, `VSN-Agent` remains absent throughout.
2. Per-machine NSIS install: exact service name/display/start/account/image path.
3. Per-machine NSIS health: service reaches RUNNING and installed CLI ping exits 0.
4. Per-machine NSIS stop/start: bounded transitions and second health pass.
5. Per-machine NSIS uninstall: stop/remove before payload deletion; service absent afterwards.
6. MSI/WiX: same service configuration, health, bounded stop/start, and uninstall removal.
7. MSI/WiX: no duplicate Agent file ownership/component.
8. Repository: zero tracked drift after certification harness execution.
9. Scope: no mutation outside approved 03.11 surfaces.
10. Evidence: exact source SHA/run/job/artifact and installer hashes recorded.

## Recovery / rollback boundary

Repository change is `SIMPLE_ROLLBACK` by reverting the scoped installer integration before release. This does not certify failed-install rollback or interrupted recovery; those remain PKG-03 03.18.
