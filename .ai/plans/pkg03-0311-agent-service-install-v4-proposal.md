# PKG-03 03.11 — V4 Change-Control Proposal

Status: **PROPOSED — REQUIRES EXPLICIT SCOPE-EXPANSION APPROVAL**

## Canonical authority
- Canonical `main`: `436dd74ab0a0006d49f6a5ff37cf25c478897248`
- Source PR: `#135`
- Source head inspected: `98454f7103bcbeff2372039bf3b19efe4615b3e6`
- Product version: `0.38.1`
- PKG-03 remains `10/25 = 40%`; active task `03.11` remains `READY`.
- V3 implementation-unpause head `98454f7103bcbeff2372039bf3b19efe4615b3e6` passed all five required governance gates before this new read-only finding.

## New plan-reality mismatch
V3 correctly moved final live-running MSI uninstall coordination to `03.19` and changed `03.11` to certify MSI removal from an explicitly stopped `VSN-Agent`.

The current WiX fragment still schedules `Pkg0311StopService` as a deferred, no-impersonate, return-checked custom action and invokes:

`[INSTALLDIR]bin\vsn-agent.exe service stop`

The Agent `service stop` command delegates directly to `sc.exe stop VSN-Agent`; the Agent wrapper converts any non-zero `sc.exe` result into failure. Windows SCM specifies `ERROR_SERVICE_NOT_ACTIVE (1062)` when a stop control is sent to a stopped service. Therefore, after the V3 certification harness safely pre-stops the service, the MSI deferred stop action can fail on the same already-stopped service. A harness+validator-only mutation cannot truthfully satisfy the amended V3 contract.

## Rejected shortcuts
- Do not mark the MSI custom action `Return="ignore"`; genuine stop failures must remain fatal.
- Do not restart the service after certification pre-stop merely so the MSI stop action succeeds; that falsifies the stopped-service acceptance boundary.
- Do not race the harness around `InstallValidate`.
- Do not mutate Agent/core runtime; that would change the accepted `03.10` payload and broaden recertification.
- Do not move privileged mutation to an immediate pre-`InstallValidate` action.
- Do not implement Restart Manager/process coordination in `03.11`; that remains `03.19`.
- Do not fork the full WiX template or duplicate Agent file/component ownership.

## Preferred bounded correction
Expand `03.11` scope by exactly one task-owned product surface:

`apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs`

Only `Pkg0311StopService` may change. Install/start/remove actions, sequencing, Tauri config, NSIS hook and Agent/core runtime remain frozen.

The stop action must remain:
- deferred;
- `Impersonate="no"`;
- `Return="check"`;
- uninstall-only;
- before `Pkg0311RemoveService`.

The command becomes state-aware and idempotent only for the Windows-native `ERROR_SERVICE_NOT_ACTIVE (1062)` case. Preferred implementation is a Type-34 command-shell wrapper around `sc.exe stop VSN-Agent` that:
1. returns success when `sc.exe` succeeds;
2. returns success when the exact process result is `1062` (already stopped);
3. propagates every other non-zero result unchanged.

No broad failure suppression is allowed.

## Required proof before implementation acceptance
1. Exact-head governance planning gates pass after this proposal is explicitly approved and converted into the active V4 manifest/plan.
2. Windows certification confirms the native stopped-service probe produces the expected `1062` condition before relying on the wrapper.
3. Current-user NSIS service non-mutation remains green.
4. Per-machine NSIS full service lifecycle remains green.
5. MSI install/config/RUNNING health and stop/start remain green.
6. Harness explicitly stops MSI service and records `Stopped` before `msiexec /x`.
7. MSI uninstall succeeds from that stopped state; service and payload are absent afterward.
8. Evidence labels the pre-stop as certification setup and records live-running uninstall coordination as deferred to `03.19`.
9. Zero tracked repository drift from the certification harness.
10. All five required final exact-head gates pass on the implementation head.

## Proposed implementation scope after approval
Product/certification files only:
- `apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs`
- `scripts/ci/pkg03-0311-agent-service-lifecycle.ps1`
- `scripts/ci/validate-pkg03-0311.py`

Governance artifacts may additionally update the active V4 manifest/plan/checkpoint and evidence.

## Frozen boundaries
- No `apps/agent/**` or `crates/vsn-system/**` mutation.
- No Tauri config mutation.
- No NSIS hook mutation.
- No ownership-manifest mutation.
- No accepted `03.01`–`03.10` evidence rewrite.
- No PKG-03 denominator/order/DAG change.
- No `03.19` implementation inside `03.11`.

## Approval boundary
This proposal expands a previously frozen product surface (the WiX fragment) and therefore is not self-approved. Until explicit user change-control approval is recorded, `03.11` product/certification mutation remains blocked and canonical acceptance stays `READY`.
