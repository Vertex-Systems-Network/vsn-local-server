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

Status: **EVIDENCE-TESTED / REFINED BY AMENDMENTS 005–006**  
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

Status: **EVIDENCE-TESTED / INSUFFICIENT**  
Additional scope: certification-harness ServiceController lifetime only; no additional product mutation

### Causal refinement

Inspection of the immutable canonical harness `c754599a42ee44b1bb3b6d41edbf783d2146a985` identifies two machine-lifecycle functions that create live `System.ServiceProcess.ServiceController` objects:

- `Stop-AgentForRepair`
- `Assert-AgentHealthy`

Both obtain controllers through `Get-Service`, use those controller instances for `Refresh` / `WaitForStatus`, and return without an explicit `Close()` / `Dispose()`. Amendment 003 could only finalize controllers that had already become unreachable; it did not establish deterministic native-handle ownership for these live objects.

Microsoft SCM deletion semantics require the service database entry to remain until every open service handle is closed and the service is stopped. The correction therefore closes the exact controllers at their ownership boundary rather than relying on garbage collection timing.

### Amendment-004 result

Exact source head `ee8a579476b5e52f22e3f36ee601a6c58bc7be23` executed Amendment 004 in GitHub-hosted Windows run `33371568429`, job `99423675789`. Frozen authority, parser, locked Node/Cargo graphs and all three exact-head package builds passed. The run failed only in genuine `nsis-per-machine` uninstall with `nsis-per-machine uninstall did not reach required state.`

Failure artifact `9750927176` (`pkg03-0316-reinstall-repair-failure`) reports GitHub SHA-256 `8c5a88d368b062f6e948257f2dfa870ce3e199c368f890406e9fbe3c773a7638`; the downloaded ZIP was independently recomputed to the exact same digest. After the uninstall action, the real NSIS terminal page appeared with content `Close` disabled. Hundreds of subsequent probes continued to report `VSN-Agent=Stopped`, machine payload and HKLM registration present, and no live `vsn-agent.exe` or `sc.exe` process. Runner cleanup ultimately terminated the still-live NSIS root process `Un`.

Deterministic `ServiceController.Close()/Dispose()` therefore does not resolve the pending uninstall. Amendment 004 remains useful deterministic resource hygiene but is not accepted as the causal remediation.

---

## Amendment 005 — allow SCM marked deletion to converge across uninstaller exit

Status: **EVIDENCE-TESTED / INSUFFICIENT AS SOLE REMEDIATION**  
Additional scope: per-machine NSIS pre-uninstall service-delete completion boundary only

### Trigger and causal decision

The Amendment-004 artifact and source audit establish a more precise failure mechanism:

1. `sc.exe delete VSN-Agent` succeeds, but Windows SCM deletion is a mark-for-deletion operation; Microsoft documents that the database record is removed only after the service is stopped and the last open service handle closes.
2. Amendment 002 then polls `sc.exe query VSN-Agent` inside `NSIS_HOOK_PREUNINSTALL` and requires `1060` before allowing the uninstall section to continue.
3. That polling is bounded to 40 × 250 ms. If the record is still queryable, the hook executes `Abort` from the NSIS uninstall Section.
4. NSIS Section semantics make that state fail-closed by stopping script execution and leaving only Cancel enabled. This exactly matches the artifact: the terminal page appears almost immediately, content `Close` remains disabled, payload and ARP cleanup never execute, and the root uninstaller remains alive until the external acceptance timeout.
5. Waiting for final SCM record disappearance *inside the same uninstaller section* is therefore the wrong completion boundary. It can prevent the process exit after which outstanding service handles are naturally released.

### Authorized correction

The product mutation remains restricted to the same singleton path:

- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

For per-machine uninstall only:

- preserve the accepted `vsn-agent.exe service stop` operation and fail closed on a real stop failure;
- issue `sc.exe delete VSN-Agent` directly;
- accept delete exit `0` and already-absent exit `1060` only;
- fail closed on every other delete exit;
- do **not** poll for non-queryability from inside the NSIS uninstall Section;
- permit normal payload/ARP cleanup and process exit after a successful delete request;
- retain the frozen external/post-process acceptance requirement that, after the root process exits successfully, the service, owned payload and uninstall registration are all absent.

This does not weaken cleanup acceptance: it relocates the final service-removal observation to the already-existing post-process boundary where SCM marked-deletion semantics can converge.

### Amendment-005 result

Exact source head `f00371b550da9b71044bb1281116609f1f061283` executed Amendment 005 in GitHub-hosted Windows run `33376576131`, job `99439326448`. Frozen authority/parser checks, all five required governance gates and all three exact-head package builds passed. The run again failed only at genuine `nsis-per-machine` uninstall with `nsis-per-machine uninstall did not reach required state.` Runner cleanup terminated the still-live NSIS root process `Un`, proving that removing the internal delete-query wait was not sufficient by itself. Failure artifact `9752714070` is bound to that exact head and GitHub reports SHA-256 `76b1e249874ba5a961c742e4cc72d24317c3256744d767ba4b9f2fe062f4dac4`.

### Explicitly unchanged / forbidden

- no service identity/account/start-mode/binPath change;
- no Agent Rust mutation;
- no manual service, payload or ARP cleanup from certification;
- no acceptance timeout increase;
- no force-click or Cancel-as-success behavior;
- no relaxation of exit code `0`, exact repair restoration, identity stability, ACL invariants, service/payload/registration cleanup or zero tracked drift;
- no running-process coordination, rollback/recovery, reboot, unattended deployment, signing or updater scope.

---

## Amendment 006 — idempotent already-stopped service pre-uninstall

Status: **ACTIVE / evidence-triggered**  
Additional scope: per-machine NSIS pre-uninstall service-stop idempotence only

### Deterministic causal evidence

The frozen 03.16 lifecycle intentionally quiesces the machine Agent immediately before destructive uninstall by calling `Stop-AgentForRepair`. That function waits for `VSN-Agent` to reach `Stopped` before the uninstaller is started. The product `NSIS_HOOK_PREUNINSTALL` then invokes `vsn-agent.exe service stop` a second time and currently accepts only exit code `0`.

The Agent service command is a thin wrapper around `sc.exe stop VSN-Agent`. Windows SCM defines `ERROR_SERVICE_NOT_ACTIVE (1062)` when the service has not been started or its current state is `SERVICE_STOPPED`. Therefore the certification precondition deterministically makes the second stop request eligible to return native `1062`. The current hook treats that idempotent already-stopped state as a real failure and executes `Abort` before Tauri can remove the main payload, sidecars, uninstall registration or shortcuts. That exactly matches the repeated evidence family: `VSN-Agent=Stopped`, payload/ARP still present, no long-lived Agent/sc helper, disabled terminal completion and a still-live NSIS `Un` root process.

This correction does not broaden 03.16 into live-running process coordination; it makes the existing stop operation idempotent for the already-quiescent state required by the frozen test itself.

### Authorized correction

Only the already-authorized product path may change:

- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

Within `NSIS_HOOK_PREUNINSTALL` for `perMachine` only:

- keep the accepted `vsn-agent.exe service stop` command;
- accept exit `0` as normal stop success;
- accept exit `1062` only as the idempotent already-stopped success case;
- fail closed on every other stop exit code;
- preserve Amendment 005's direct `sc.exe delete VSN-Agent` behavior with delete success limited to `0` or already-absent `1060`;
- preserve all post-process service/payload/ARP cleanup, root-exit, exact-repair, identity, ACL and zero-drift acceptance.

### Explicit prohibitions

- no Agent Rust mutation;
- no service start/stop identity or configuration change;
- no forced process termination or Restart Manager behavior;
- no manual payload/ARP deletion from the harness;
- no timeout or completion-predicate change;
- no treatment of arbitrary nonzero service-stop exits as success;
- no 03.19 live-running coordination, rollback/recovery, reboot, unattended deployment, signing or updater scope.

### Proof required for Amendment 006

The exact amended head must rerun frozen authority/parser/dependency validation, all required governance, all three exact-head package builds and the complete genuine 03.16 reinstall/repair/uninstall lifecycle. A green workflow remains only candidate evidence until its success ZIP is independently downloaded, its SHA-256 recomputed against GitHub's artifact digest, and all lifecycle records are inspected. Only then may `03.16` be projected `DONE` or PR #146 be merged.
