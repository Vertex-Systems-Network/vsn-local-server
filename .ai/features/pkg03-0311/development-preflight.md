# PKG-03 03.11 Development Preflight

Canonical base: `4f33813bec4254107e6027e98b2a4a8878b9198b`
Task: `03.11`
Linear: `ABD-86`

## Dependency/state check

- 03.07 NSIS per-machine elevated install/uninstall lifecycle: DONE
- 03.10 CLI and Agent payload placement/discovery/launch: DONE
- canonical tracker: 10/25 = 40%
- deterministic cursor: 03.11
- 03.11 tracker status: READY
- 03.11 Linear mirror: In Progress
- lane: `service`
- other READY lanes remain independent; no branch/code sharing is permitted.

## Locked inputs

- product: `VSN Dev Platform`
- version: `0.38.1`
- bundle identifier: `dev.vsn.platform`
- publisher: `Vertex Systems Network`
- Agent service: `VSN-Agent`
- Agent display name: `VSN Agent`
- service account: `NT AUTHORITY\LocalService`
- service start type: automatic
- service executable: `bin/vsn-agent.exe --service-run`
- CLI health probe: installed `bin/vsn.exe ping`
- NSIS per-machine overlay: `apps/desktop/src-tauri/tauri.per-machine.conf.json`
- Windows payload overlay: `apps/desktop/src-tauri/tauri.windows.conf.json`
- Tauri CLI evidence version: `2.11.4`
- Node: `22.12.0`
- Rust: `1.97.1`

## Mutation authority

Planning stage may change only this 03.11 planning bundle.

After planning gates pass, implementation may change only:
- `apps/desktop/src-tauri/tauri.windows.conf.json` for supported NSIS hook and WiX fragment wiring;
- task-owned NSIS hook under `apps/desktop/src-tauri/windows/`;
- task-owned WiX fragment under `apps/desktop/src-tauri/windows/fragments/`;
- task-owned `scripts/ci/pkg03-0311-*` validation/certification helpers;
- task-owned `.github/workflows/pkg03-0311-*` exact-head Windows certification;
- `certification/pkg03-windows-installer-v1.json`, `docs/MASTER-EXECUTION-STATUS.json` and `README.md` only after genuine exact-head acceptance.

The accepted Agent runtime (`apps/agent/src/main.rs`) and service-control core (`crates/vsn-system/src/lib.rs`) are read-only inputs for this task. If exact certification proves a runtime defect, stop and amend the plan before changing them.

Prohibited without change control:
- a second copy/owner of `bin/vsn-agent.exe`;
- custom full NSIS or WiX templates;
- current-user machine-service mutation;
- PATH/environment mutation;
- ACL/state/config changes;
- firewall/hosts/DNS/trust changes;
- repair/rollback/reboot/unattended/signing/updater/recovery changes.
