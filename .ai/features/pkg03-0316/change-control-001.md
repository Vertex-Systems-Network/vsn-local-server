# PKG-03 03.16 — Bounded Change Control 001

Status: **ACTIVE / evidence-triggered**  
Task: `03.16`  
Linear: `ABD-91`  
Scope: per-machine NSIS same-version reinstall idempotence only

## Trigger evidence

Exact source head `08967f1fa4c5da68cbf0d1f5498bd118222ea051` failed GitHub-hosted Windows run `33281884610`, job `99178308962`, at the first healthy same-version per-machine NSIS maintenance pass:

`nsis-per-machine reinstall-healthy-1 root process did not exit.`

The failure artifact showed that current-user NSIS completed healthy reinstall, missing-file repair, tamper repair and the second healthy pass. Per-machine NSIS also completed its initial install and matched all accepted owned payload hashes before the first maintenance rerun stalled.

Source inspection established a deterministic causal path:

1. `NSIS_HOOK_POSTINSTALL` for `perMachine` unconditionally executes `vsn-agent.exe service install` on every install/reinstall.
2. `vsn-agent service install` executes `sc.exe create VSN-Agent ...` and treats any non-zero `sc.exe` status as failure.
3. On a healthy same-version reinstall the accepted `VSN-Agent` service already exists by definition, so repeating `sc.exe create` is not idempotent.
4. The NSIS hook aborts from the install progress path instead of completing the maintenance lifecycle.

This is a product-installer idempotence defect exposed by the certification-first 03.16 gate, not an acceptance-harness relaxation opportunity.

## Authorized mutation

Exactly one accepted product-input path may change under this control:

- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

The allowed behavior change is limited to making the post-install service registration step idempotent:

- query whether `VSN-Agent` already exists;
- if it exists, do **not** recreate or reconfigure it;
- if it does not exist, execute the already accepted `vsn-agent.exe service install` path;
- always execute the already accepted service start path afterward;
- retain fail-closed behavior for failed fresh service installation or service start.

## Explicitly forbidden

- no service name/account/start-mode/binPath identity change;
- no Agent payload mutation;
- no Tauri configuration/template replacement;
- no ACL/security-state mutation;
- no new repair runtime or repair executable;
- no acceptance weakening, timeout inflation, skipped phase, or fake completion signal;
- no running-process/Restart Manager, dirty-data, rollback, reboot, unattended deployment, signing, updater, PKG-04 or PKG-05 scope.

## Required proof after mutation

The exact changed head must rerun all 03.16 governance plus `PKG-03 03.16 Reinstall Repair`. Acceptance still requires all three installer lifecycles, exact missing/tampered SHA restoration, stable registration/service/ACL identity, cleanup, and zero tracked drift. If the same-version per-machine rerun still fails, this control does not authorize further product mutation; a new evidence-based change-control decision is required.

---

## Amendment 002 — per-machine uninstall service-removal completion

Status: **ACTIVE / evidence-triggered**  
Additional scope: per-machine NSIS uninstall service-removal completion only

### Trigger evidence

Exact source head `5bc2150c74a04dc5f893737b618dd7fa84ad0eb1` failed GitHub-hosted Windows run `33352416537`, job `99368202715`, only in the genuine `nsis-per-machine` uninstall phase:

`nsis-per-machine uninstall did not reach required state.`

All authority/parser checks and all three package builds passed. Failure artifact `9744386201` (`pkg03-0316-reinstall-repair-failure`) is bound to that exact head. GitHub reports SHA-256 `f2a8971ecc3f475f6e65880618e74595a658b90a7170a37717646e7ddab0e658`, independently recomputed from the downloaded ZIP to the same value.

The artifact contains fifty identical per-machine terminal progress probes spanning approximately six minutes. Every probe records:

- genuine NSIS content `Close` present but disabled;
- `VSN-Agent` state `Stopped`;
- machine payload still present;
- HKLM uninstall registration still present;
- no live `vsn-agent.exe` helper PID;
- no live `sc.exe` PID.

Current-user uninstall is the control case: cleanup completes through the same frozen harness and its real content `Close` becomes actionable. The per-machine failure is therefore a product lifecycle defect before terminal completion, not authority to force a disabled UI control or weaken acceptance.

Source inspection at the same exact head establishes:

1. the per-machine `NSIS_HOOK_PREUNINSTALL` in `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh` runs `vsn-agent.exe service stop`, then `vsn-agent.exe service uninstall`, aborting on non-zero status;
2. `apps/agent/src/main.rs` implements `service uninstall` / `service remove` as a thin synchronous wrapper around `sc.exe delete VSN-Agent` and contains no additional removal semantics;
3. run-52 telemetry proves the service has reached `Stopped` and neither wrapper nor `sc.exe` remains running while teardown never advances.

This is the new evidence-based change-control decision required by the original proof clause above.

### Additional authorized mutation

The accepted product-input path remains exactly the same singleton path:

- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

Within that path only, the per-machine `NSIS_HOOK_PREUNINSTALL` service-removal step may be made explicit and completion-aware:

- preserve the accepted service stop operation and fail closed on a real stop failure;
- remove `VSN-Agent` through the Windows Service Control Manager without depending on the installed Agent executable as a second removal wrapper;
- treat only the already-absent service as an idempotent success;
- fail closed on any other service-delete failure;
- verify that `VSN-Agent` is no longer queryable before permitting installer payload/ARP cleanup to continue;
- keep this behavior change bounded to `perMachine` uninstall.

Windows-native `sc.exe`, already relied on by the product, may be used. No new runtime, helper executable, PowerShell dependency, acceptance shim, or signing secret is authorized.

### Additional explicit prohibitions

- no Agent Rust payload mutation, including `apps/agent/src/main.rs`;
- no current-user install/uninstall behavior change;
- no manual payload or HKLM ARP deletion in the certification harness;
- no force-click of disabled installer controls;
- no timeout inflation, skipped cleanup predicate, fake completion signal, or weakened process-exit/exit-code requirement;
- no service identity/configuration/security mutation;
- production signing material remains external and unavailable signing must continue to fail closed.

### Proof required for Amendment 002

The exact implementation head must rerun the frozen 03.16 authority/governance gates and `PKG-03 03.16 Reinstall Repair`. All three installer lifecycles, exact missing/tampered SHA-256 restoration, stable service/registration/ACL identity, service/payload/registration uninstall cleanup, successful root-process exit, and zero tracked drift remain mandatory.

A successful Windows run alone is insufficient. Its exact-head evidence artifact and reported digest must be independently downloaded, recomputed and inspected before `03.16` may be marked DONE or PR #146 may be merged. No dependent canonical task becomes ready from branch-local evidence.

---

## Amendment 003 — certification ServiceController handle lifetime

Status: **EVIDENCE-TESTED / INSUFFICIENT**  
Additional scope: certification-harness resource lifetime only; no additional product mutation

### Trigger evidence

Exact source head `f5947e1445648c368a66c963858f05de09dc56ad` failed GitHub-hosted Windows run `33366890932`, job `99409282570`, only at genuine `nsis-per-machine` uninstall completion:

`nsis-per-machine uninstall did not reach required state.`

The failure artifact is `9749184845` (`pkg03-0316-reinstall-repair-failure`), reported SHA-256 `3b10061a5320f1c933e25d5882fa81b63a82f70c818489902f3e926bdbcf8834`. Inspection proves both NSIS scopes already pass healthy reinstall, exact missing-file restoration, exact tamper restoration and the second healthy pass. During the failing per-machine uninstall, repeated terminal probes record `VSN-Agent=Stopped`, payload present, HKLM registration present, disabled genuine content `Close`, and no live `vsn-agent.exe` or `sc.exe` process.

The accepted product hook now issues bounded SCM-native deletion and waits for the service to stop being queryable. Windows `DeleteService` semantics require the service to remain marked for deletion until all open service handles are closed and the service is stopped. The frozen PowerShell lifecycle uses `Get-Service` / `ServiceController` objects immediately before uninstall, and the diagnostic terminal probe introduced for causality also called `Get-Service` on every retry without explicitly closing the returned controller. Those certification-side handles can therefore keep the service queryable while the product is correctly waiting for SCM deletion completion.

### Amendment-003 result

Exact source head `4222ad2092c4412e03bff8ef8b15d592383c00e4` executed Amendment 003 in GitHub-hosted Windows run `33369117565`, job `99415984298`. All exact package builds and all reinstall/repair phases passed again, but genuine `nsis-per-machine` uninstall still failed. Failure artifact `9750009133` was independently downloaded and recomputed to GitHub's exact SHA-256 `72c6fab2eb21c89440d460f46bf575b03fdc6069d59bcb98319a79fe139c695e`.

The artifact records `service-controller-finalizer-drain` at the first disabled content `Close`, followed by **292 consecutive** CIM-only progress probes. Every probe still reports `VSN-Agent=Stopped`, machine payload present, HKLM registration present, and no `vsn-agent.exe` or `sc.exe` process. Therefore collection/finalization of only unreachable controllers is not sufficient and must not be represented as accepted remediation.

---

## Amendment 004 — deterministic certification ServiceController ownership

Status: **ACTIVE / evidence-triggered**  
Additional scope: certification-harness ServiceController lifetime only; no additional product mutation

### Causal refinement

Inspection of the immutable canonical harness `c754599a42ee44b1bb3b6d41edbf783d2146a985` identifies two machine-lifecycle functions that create live `System.ServiceProcess.ServiceController` objects:

- `Stop-AgentForRepair`
- `Assert-AgentHealthy`

Both obtain controllers through `Get-Service`, use those controller instances for `Refresh` / `WaitForStatus`, and return without an explicit `Close()` / `Dispose()`. Amendment 003 could only finalize controllers that had already become unreachable; it did not establish deterministic native-handle ownership for these live objects.

Microsoft SCM deletion semantics require the service database entry to remain until every open service handle is closed and the service is stopped. The new correction therefore closes the exact controllers at their ownership boundary rather than relying on garbage collection timing.

### Authorized certification mutation

Only:

- `scripts/ci/pkg03-0316-reinstall-repair.ps1`

may change for Amendment 004. It may:

- flatten the historical certification-wrapper chain onto the immutable canonical harness blob `aa054f97309407f394bd2a87297d3d6428794711`;
- retain the canonical acceptance logic and patch only the two controller-owning functions plus the already-authorized fail-closed terminal evidence/activation helper;
- place each `Get-Service` controller in `try/finally` and call `Close()` and `Dispose()` deterministically;
- continue CIM-only progress telemetry without manually deleting service, payload or ARP state.

### Explicitly unchanged / forbidden

- no additional mutation to `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`;
- no Agent Rust change;
- no manual service deletion or cleanup from certification;
- no disabled-control force click and no generic `WM_CLOSE` success path;
- no timeout increase or acceptance predicate change;
- no weakening of root-process exit code `0`, exact SHA repair, stable registration/service/ACL identity, uninstall service/payload/ARP cleanup or zero tracked drift;
- no later-task scope.

### Proof required for Amendment 004

The exact amended branch head must pass frozen authority/parser/dependency gates and the full `PKG-03 03.16 Reinstall Repair` lifecycle. If successful, its evidence ZIP must be independently downloaded, its digest recomputed, and all three lifecycle records inspected before `03.16` can become `DONE` or PR #146 can merge. A repeat `Stopped`/payload-present/ARP-present stall is evidence against this refinement and must trigger a new causal decision rather than acceptance weakening.
