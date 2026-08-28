# PKG-03 03.11 — V4 Approved Change-Control Plan

Status: **APPROVED — PLANNING GATES REQUIRED BEFORE IMPLEMENTATION**

## Approval and authority
- Approval ref: `conversation:user-2026-08-28-continue-v4-scope-expansion`
- Canonical `main`: `436dd74ab0a0006d49f6a5ff37cf25c478897248`
- Source PR: `#135`
- V4 proposal head: `c0b67d91b2a397255dc346ad30b1ace9000e3592`
- Product version: `0.38.1`
- PKG-03 remains `10/25 = 40%`; active task `03.11` remains `READY`.
- This is a task-scoped correction, not new product scope.

## Proven plan-reality mismatch
V3 changed 03.11 MSI uninstall acceptance to begin from an explicitly stopped `VSN-Agent`, while 03.19 retained live-running Restart Manager/service coordination.

Read-only source review then proved a second-stop mismatch:
- Agent `service stop` delegates to `sc.exe stop VSN-Agent`.
- Agent treats every non-zero `sc.exe` exit as failure.
- Windows SCM reports `ERROR_SERVICE_NOT_ACTIVE (1062)` when a stop control targets an already stopped service.
- The WiX `Pkg0311StopService` action is deferred, non-impersonated and `Return="check"`.
- Therefore a truthful V3 certification pre-stop can be followed by a return-checked second stop that fails only because the service is already stopped.

Evidence:
- `.ai/evidence/pkg03-0311-v4-stopped-service-idempotency-mismatch.json`
- `.ai/plans/pkg03-0311-agent-service-install-v4-proposal.md`
- Agent source and current task-owned WiX fragment on PR #135.

## Approved bounded correction
The previously frozen task-owned WiX fragment is now explicitly added to the V4 implementation scope:

`apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs`

Only `Pkg0311StopService` may change in that fragment. The V4 implementation scope is:
1. `apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs`
2. `scripts/ci/pkg03-0311-agent-service-lifecycle.ps1`
3. `scripts/ci/validate-pkg03-0311.py`

Governance artifacts for V4 may additionally update this plan, the V4 manifest, `.ai/current-work.json`, and V4 evidence.

## Required WiX behavior
`Pkg0311StopService` must remain:
- deferred;
- `Impersonate="no"`;
- `Return="check"`;
- uninstall-only;
- before `Pkg0311RemoveService`.

The stop action becomes state-aware only for the native already-stopped result:
- success from `sc.exe stop VSN-Agent` -> success;
- exact `ERROR_SERVICE_NOT_ACTIVE (1062)` -> success;
- every other non-zero result -> propagated failure.

A broad `Return="ignore"` or blanket error suppression is forbidden.

Preferred implementation is a Type-34 command-shell wrapper around native `sc.exe` that captures the exact exit code and normalizes only `1062` to zero. The implementation must be validated on the Windows runner before acceptance; if the native observed exit semantics differ, stop and reassess rather than broadening suppression.

## Certification behavior
The V4 harness must:
1. preserve current-user NSIS machine-service non-mutation proof;
2. preserve per-machine NSIS install/config/health/stop-start/uninstall proof;
3. preserve MSI install/config/RUNNING health and bounded stop/start proof;
4. explicitly stop the MSI Agent before uninstall as certification setup;
5. record and assert `Stopped` before invoking `msiexec /x`;
6. record a native stopped-service probe proving the exact already-stopped result used by the WiX wrapper;
7. run MSI uninstall from the stopped state and require exit `0`;
8. require `VSN-Agent` absent and payload removed after uninstall;
9. label the pre-stop as certification setup, not installer-owned live-running coordination;
10. record that final live-running coordination remains owned by task `03.19`;
11. preserve zero tracked repository drift.

## Validator requirements
The V4 validator must fail closed unless:
- the active V4 plan/manifest are present and approved;
- the implementation head descends from the V4 planning authorization head once established;
- only approved V4 implementation surfaces changed after authorization;
- `Pkg0311StopService` remains deferred/no-impersonate/return-checked and uninstall-only;
- only exact `1062` is normalized;
- install/start/remove WiX actions and their sequencing remain unchanged;
- Tauri config and NSIS hook remain unchanged;
- Agent/core and the accepted 03.10 payload ownership/evidence remain unchanged;
- the harness records native 1062 proof, certification pre-stop, stopped state, successful MSI uninstall, and 03.19 deferral;
- canonical tracker/master status remains `03.11 READY / 10 of 25` until genuine final evidence passes.

## Frozen boundaries
No mutation to:
- `apps/agent/**`
- `crates/vsn-system/**`
- `apps/desktop/src-tauri/tauri.windows.conf.json`
- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`
- `installer/windows/owned-payload.v1.json`
- accepted 03.01–03.10 evidence
- PKG-03 denominator/order/DAG
- 03.19 implementation.

No immediate privileged pre-`InstallValidate` action, no full WiX template fork, no duplicate Agent payload ownership, no Restart Manager implementation in 03.11.

## Gate sequence
1. Persist active V4 plan + manifest + checkpoint.
2. Run exact-head:
   - AI Planning Governance
   - Repository Governance
   - PKG-03 Acceptance Sequence
   - Engineering Contract Governance
   - Operational Governance
3. Implement only the three approved V4 product/certification files after all five pass.
4. Re-run all five exact-head gates on implementation head.
5. Run `PKG-03 03.11 Agent Service Lifecycle` on the same exact head.
6. Inspect machine-readable evidence and installer logs.
7. Only after genuine green evidence may canonical tracker/master status and Linear be promoted.

## Definition of done
03.11 can become `DONE` only when:
- all amended current-user, per-machine NSIS and MSI assertions pass on one exact head;
- native stopped-service semantics are proven rather than assumed;
- MSI stopped-service uninstall succeeds with service and payload absent afterward;
- live-running uninstall coordination is explicitly deferred to 03.19;
- no frozen surface drift occurred;
- final five governance gates and dedicated Windows certification are green;
- exact run/job/artifact/source hashes are durably bound.
