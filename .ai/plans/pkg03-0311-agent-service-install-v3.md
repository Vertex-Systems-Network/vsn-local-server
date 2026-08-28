# PKG-03 03.11 — VSN Agent Windows Service Install Lifecycle Plan v3

## Change-control decision

Approved on 2026-08-28 by the user continuing after the documented 03.11 STOP_AND_REASSESS recommendation.

Genuine exact-head Windows evidence on `4f92157a5b04a129c859967b45d23c0f03346f57` proved that stock WiX sequencing cannot securely stop a running privileged Agent service before `InstallValidate` while retaining the approved deferred, non-impersonated machine-mutation model. The previous plan therefore had a plan/reality mismatch for the MSI live-running uninstall case.

This v3 amendment preserves the PKG-03 denominator, task order, dependency DAG, Agent runtime, single-payload ownership, current-user NSIS boundary, and existing 03.19 ownership of running-process/Restart Manager coordination.

## Goal

Complete 03.11 without weakening Windows Installer privilege boundaries:

- current-user NSIS remains machine-service neutral;
- per-machine NSIS continues to prove complete install/start/health/stop/start/uninstall service lifecycle;
- MSI/WiX continues to prove secure deferred/no-impersonate service install/start/remove authoring, exact service identity, health, and cleanup;
- for the 03.11 MSI uninstall acceptance lane, the certification operator safely stops the already-verified Agent service before invoking MSI uninstall;
- MSI uninstall must then remove the stopped service before payload deletion and leave no service/payload residue;
- 03.19 retains ownership of running Desktop/CLI/Agent coordination, Restart Manager/service coordination, and the final live-running MSI uninstall proof.

## Preserved authorization

The approved change is an acceptance-boundary correction, not new product scope.

Preserve:

- `bin/vsn-agent.exe` remains solely owned by 03.10;
- service name `VSN-Agent`;
- display name `VSN Agent`;
- automatic start;
- `NT AUTHORITY\LocalService`;
- `--service-run`;
- current-user NSIS never mutates machine service state;
- no Agent/core runtime change;
- no second Agent File/Component;
- no full NSIS/WiX template fork;
- no immediate privileged pre-`InstallValidate` custom action;
- no Restart Manager implementation in 03.11;
- PKG-03 remains 25 tasks and 03.19 remains dependent on 03.11 and 03.15.

## Amended acceptance sequence

1. Bind the amended plan to live canonical main and the existing PKG-03 tracker.
2. Verify 03.07 and 03.10 remain DONE and 03.11 remains READY.
3. Verify current Agent service identity and single-payload ownership are unchanged.
4. Reuse the existing Tauri-supported NSIS hook and WiX fragment integration.
5. Current-user NSIS install/uninstall must keep `VSN-Agent` absent.
6. Per-machine NSIS must install/start the Agent service, prove exact configuration and CLI health, prove bounded stop/start and second health, then uninstall and remove service/payload cleanly.
7. MSI/WiX install must create/start the exact Agent service using deferred, non-impersonated, synchronous, return-checked custom actions.
8. MSI install must prove exact service configuration, CLI health, bounded service stop/start and second health.
9. Before the 03.11 MSI uninstall invocation, the certification operator must stop the service through the installed Agent's accepted service interface and prove state `Stopped`.
10. MSI uninstall must execute with the service already stopped, remove the service through the existing secure deferred/no-impersonate action, remove owned payload, return success, and leave service/payload absent.
11. Evidence must explicitly record that the pre-uninstall stop was certification setup and is **not** claimed as installer-owned live-running coordination.
12. 03.11 must not claim Restart Manager behavior or live-running MSI uninstall support.
13. 03.19 must inherit the remaining proof obligation: coordinate running Desktop/CLI/Agent processes and prove live-running uninstall behavior under its Restart Manager/service-coordination scope.
14. Certify zero duplicate Agent ownership and zero tracked repository drift.
15. Bind exact-head evidence to source SHA/run/job/artifact and installer hashes.
16. Only after all amended 03.11 assertions pass may tracker/master project 03.11 DONE.

## Implementation design

### NSIS

No acceptance change. Use `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh` through Tauri `installerHooks`. Current-user remains a compile-time service no-op; per-machine retains full installer-owned service lifecycle.

### MSI/WiX

Keep `apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs` and the existing secure custom-action privilege model:

- deferred;
- `Impersonate="no"`;
- synchronous;
- `Return="check"`;
- existing installed Agent executable;
- no second File/Component.

Do not move service-stop to an immediate pre-`InstallValidate` custom action.

For 03.11 certification only, after MSI install/health/stop-start proof and immediately before `msiexec /x`, the harness calls the installed Agent `service stop`, waits for SCM `Stopped`, records this as `certification_pre_uninstall_stop`, and then invokes MSI uninstall. MSI itself must remove the stopped service and payload successfully.

### 03.19 handoff

03.19 remains the frozen task:

`Running Desktop, CLI and Agent handling with Restart Manager/service coordination`

Its later acceptance must include the proof intentionally deferred here: uninstall coordination when Agent/Desktop/CLI are live, including service/process coordination and Windows Installer Restart Manager behavior as applicable.

## Expected changed surfaces after approval

Planning/change-control:

- `.ai/plans/pkg03-0311-agent-service-install-v3.md`
- `.ai/manifests/pkg03-0311-agent-service-install.v3.json`
- `.ai/current-work.json`

Implementation/certification:

- `scripts/ci/pkg03-0311-agent-service-lifecycle.ps1`
- `scripts/ci/validate-pkg03-0311.py`

No Tauri config, NSIS hook, WiX fragment, Agent runtime, payload ownership, tracker state, master status, or accepted 03.01–03.10 evidence change is required by this amendment.

## Security rationale

The amendment avoids replacing the approved privileged deferred/no-impersonate MSI model with an immediate custom action that could change privilege semantics. It also avoids faking live-running installer ownership: evidence must state clearly that 03.11's MSI removal proof begins from a safely stopped service, while 03.19 owns the live-running coordination requirement.

## Definition of done for this amendment

03.11 may become DONE only when:

- all five required exact-head governance gates succeed;
- dedicated 03.11 Windows lifecycle succeeds;
- current-user NSIS negative boundary passes;
- per-machine NSIS full service lifecycle passes;
- MSI install/start/health/stop-start passes;
- certification pre-uninstall stop is explicit and succeeds;
- MSI removes the stopped service and payload successfully;
- live-running MSI uninstall is explicitly deferred to 03.19 with no false claim;
- single Agent ownership and zero tracked drift are proven;
- exact evidence is source/run/job/artifact bound.
