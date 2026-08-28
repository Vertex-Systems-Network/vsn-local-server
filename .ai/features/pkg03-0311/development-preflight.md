# PKG-03 03.11 Development Preflight v2

Canonical base: `af67f43ac3104eb5dbfd133dff881e79a8ea71f4`
Task: `03.11`
Linear: `ABD-86`
Governance: Engineering Governance V3 `APPLIED`

## Authority and state

- project state: `ACTIVE_EXISTING_PROJECT`
- active package: `PKG-03`
- canonical progress: `10/25 = 40%`
- deterministic cursor: `03.11`
- canonical task state: `READY`
- dependencies: `03.07 DONE`, `03.10 DONE`
- approval scope: `TASK`
- change classification: `COMPLETION`
- implementation gap: `MISSING_IMPLEMENTATION`
- existing authorized scope does not require retroactive reapproval
- any material scope/privilege/data-flow/acceptance/dependency change requires reapproval

## Locked inputs

- product: `VSN Dev Platform`
- version: `0.38.1`
- identifier: `dev.vsn.platform`
- publisher: `Vertex Systems Network`
- service: `VSN-Agent` / `VSN Agent`
- service account: `NT AUTHORITY\LocalService`
- start type: automatic
- Agent: `bin/vsn-agent.exe --service-run`
- health probe: `bin/vsn.exe ping`
- Tauri CLI: `2.11.4`
- Node: `22.12.0`
- Rust: `1.97.1`

## Expected implementation changes

Allowed product/config surfaces after this planning head passes:
- `apps/desktop/src-tauri/tauri.windows.conf.json`
- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`
- `apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs`
- `scripts/ci/pkg03-0311-*`
- `.github/workflows/pkg03-0311-*`

Post-acceptance canonical projection only:
- `certification/pkg03-windows-installer-v1.json`
- `docs/MASTER-EXECUTION-STATUS.json`
- `README.md`

Read-only product inputs:
- `apps/agent/src/main.rs`
- `crates/vsn-system/src/lib.rs`
- accepted 03.01–03.10 evidence

## Shared surfaces / concurrency

Classification: `COORDINATED_PARALLEL`.

Shared surface: `apps/desktop/src-tauri/tauri.windows.conf.json`.
Collision key: `pkg03-windows-tauri-config`.

03.11 may proceed in parallel only while no other active lane mutates that exact shared surface. A new collision requires `STOP_AND_REASSESS`. Package concurrency remains capped at five by the canonical PKG-03 tracker.

## Scope budget

- max changed files before final state projection: 8
- max new files before final state projection: 6
- max shared surfaces: 1
- exceeding any bound requires `STOP_AND_REASSESS`

## Forbidden boundaries

- Agent/core runtime changes
- duplicate `bin/vsn-agent.exe` ownership
- full custom NSIS/WiX templates
- current-user machine-service mutation
- PATH/environment mutation
- ACL/state/config mutation
- firewall/hosts/DNS/trust mutation
- repair/rollback/reboot/unattended/signing/updater/recovery scope
- accepted 03.01–03.10 evidence rewrite
- PKG-03 denominator/dependency reorder

## Gates

Planning/preflight head must pass:
- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence
- Engineering Contract Governance
- Operational Governance

Development remains paused until these exact-head gates pass and `.ai/current-work.json` records the explicit 03.11 pause lift.
