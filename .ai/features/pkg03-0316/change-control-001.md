# PKG-03 03.16 — Bounded Change Control 001

Status: **ACTIVE / evidence-triggered**  
Task: `03.16`  
Linear: `ABD-91`  
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`  
Authorized product path: `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

## Guardrails

This control exists only for defects exposed by the frozen 03.16 reinstall/repair certification. It does not authorize acceptance weakening or broader installer redesign.

Always forbidden:

- Agent Rust or payload mutation;
- Tauri package identity, service identity/account/start mode/binPath changes;
- ACL/security/network mutation;
- manual service, payload or ARP cleanup from certification;
- timeout inflation, skipped phases, fake completion, disabled-control force-click or Cancel-as-success;
- weakening root-process exit code `0`, exact SHA restoration, stable identity/ACL, service/payload/registration cleanup or zero tracked drift;
- 03.17+ cleanup/rollback/running-process/reboot/unattended/signing/updater scope.

## Amendment 001 — same-version service registration idempotence

Status: **ACTIVE / retained**

Trigger head `08967f1fa4c5da68cbf0d1f5498bd118222ea051`, run `33281884610`, job `99178308962` failed at the first healthy same-version per-machine maintenance pass. The accepted postinstall hook unconditionally recreated `VSN-Agent`; `sc create` is non-idempotent when the service already exists.

Authorized correction: query `VSN-Agent`; reuse an existing registration; only execute the already accepted install helper when missing; always retain the accepted start operation and fail closed on real install/start errors.

## Amendment 002 — direct SCM service removal

Status: **EVIDENCE-TESTED / REFINED BY A005–A007**

Trigger head `5bc2150c74a04dc5f893737b618dd7fa84ad0eb1`, run `33352416537`, job `99368202715`, artifact `9744386201`, independently verified SHA-256 `f2a8971ecc3f475f6e65880618e74595a658b90a7170a37717646e7ddab0e658` failed only in genuine per-machine uninstall. Telemetry repeatedly showed `VSN-Agent=Stopped`, payload and HKLM ARP still present, disabled genuine content `Close`, and no live Agent/sc helper.

Authorized correction: retain the service stop, delete through native `sc.exe delete VSN-Agent`, keep idempotent native states narrowly enumerated, and fail closed on all other SCM errors.

## Amendment 003 — certification finalizer drain

Status: **EVIDENCE-TESTED / INSUFFICIENT**

Head `4222ad2092c4412e03bff8ef8b15d592383c00e4`, run `33369117565`, job `99415984298`, artifact `9750009133`, independently verified SHA-256 `72c6fab2eb21c89440d460f46bf575b03fdc6069d59bcb98319a79fe139c695e` still failed per-machine uninstall. After the finalizer drain, 292 CIM-only probes remained `VSN-Agent=Stopped` with payload/ARP present and no helper blocker. GC timing is not accepted remediation.

## Amendment 004 — deterministic certification ServiceController ownership

Status: **EVIDENCE-TESTED / INSUFFICIENT AS CAUSAL REMEDIATION**

The immutable canonical harness `c754599a42ee44b1bb3b6d41edbf783d2146a985` / blob `aa054f97309407f394bd2a87297d3d6428794711` created `ServiceController` objects in `Stop-AgentForRepair` and `Assert-AgentHealthy` without deterministic release. The certification wrapper therefore closes/disposes those owned controllers in `finally` and keeps CIM-only progress telemetry.

Head `ee8a579476b5e52f22e3f36ee601a6c58bc7be23`, run `33371568429`, job `99423675789`, artifact `9750927176`, independently verified SHA-256 `8c5a88d368b062f6e948257f2dfa870ce3e199c368f890406e9fbe3c773a7638` still failed genuine per-machine uninstall. The deterministic release remains valid resource hygiene but did not resolve the lifecycle.

## Amendment 005 — allow marked deletion to converge across uninstaller exit

Status: **EVIDENCE-TESTED / INSUFFICIENT AS SOLE REMEDIATION**

Microsoft SCM deletion is mark-for-deletion until the service is stopped and outstanding service handles close. The prior product hook waited inside the NSIS uninstall Section for `sc query` to return 1060, then `Abort`ed after a bounded retry budget. That completion boundary could prevent normal NSIS payload/ARP cleanup and process exit.

Authorized correction: after service stop, issue direct `sc.exe delete`; accept only successful delete or specifically documented idempotent SCM states; do not require non-queryability inside the same NSIS section; leave final service/payload/ARP absence to the unchanged post-process acceptance boundary.

Head `f00371b550da9b71044bb1281116609f1f061283`, run `33376576131`, job `99439326448`, artifact `9752714070`, independently verified SHA-256 `76b1e249874ba5a961c742e4cc72d24317c3256744d767ba4b9f2fe062f4dac4` still failed. All builds and repair/reinstall phases passed; runner cleanup terminated a still-live `Un` process. Fresh artifact telemetry contained 338 progress probes with `VSN-Agent=Stopped`, payload/ARP present and no Agent/sc process.

## Amendment 006 — already-stopped service idempotence

Status: **EVIDENCE-TESTED / INSUFFICIENT AS SOLE REMEDIATION**

The frozen 03.16 lifecycle deliberately calls `Stop-AgentForRepair` before destructive machine uninstall. Product `NSIS_HOOK_PREUNINSTALL` then invokes `vsn-agent.exe service stop` again. Native `ERROR_SERVICE_NOT_ACTIVE (1062)` is the idempotent result for an already-stopped service, so treating every nonzero stop result as fatal was incorrect for the frozen precondition.

Authorized correction: accept service-stop exit `0` or exactly `1062`; fail closed on every other stop result. No live-running process coordination is introduced.

Exact head `e852d54aaeebfb9bee30fb87c9db293a9274b1e2` executed A006 in GitHub-hosted Windows run `33406302044`, job `99534626588`. All five required governance gates, frozen authority/parser checks and all three exact-head package builds passed. Genuine per-machine uninstall still failed to reach required cleanup. Failure artifact `9764176057` is bound to that head and was independently downloaded and recomputed to GitHub's exact SHA-256 `04706d62e767b73ba85536b80ab7509481e288a9790ed677fef1ec8532878607`. Its per-machine uninstall evidence contains 315 repeated progress probes after the real Uninstall action; every probe reports `VSN-Agent=Stopped`, machine payload present, HKLM ARP present, and no `vsn-agent.exe` or `sc.exe` process. A006 therefore rules out the already-stopped stop result as the sole blocker.

## Amendment 007 — already-marked-for-delete idempotence

Status: **ACTIVE / evidence-triggered**  
Additional scope: per-machine NSIS native service-delete result classification only

### Causal decision

After A006, the preuninstall hook has only one remaining product-side fail-closed boundary before Tauri-owned payload/ARP cleanup: native `sc.exe delete VSN-Agent` accepts `0` or already-absent `1060`, while every other return executes `Abort`.

Windows SCM defines `ERROR_SERVICE_MARKED_FOR_DELETE (1072)` for a service record on which deletion has already been requested. This is not an arbitrary failure: it is the native pending-deletion state for the exact same requested outcome. Because final database removal can wait for outstanding handles to close, a repeated delete request can legitimately observe 1072 while the service is already stopped and pending removal.

The A006 artifact is consistent with this boundary: the service is `Stopped`, no delete helper remains alive, the NSIS progress page transitions immediately to a failed disabled-Close state, and Tauri payload/ARP cleanup never starts. No broader product mutation is justified by that evidence.

### Authorized correction

Only `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh` may change.

For `NSIS_HOOK_PREUNINSTALL` / `perMachine` delete only:

- retain `sc.exe delete VSN-Agent`;
- accept exit `0` as a successful delete request;
- accept `1060` only as already absent;
- accept `1072` only as already marked for deletion / pending the same delete outcome;
- fail closed on every other native delete result;
- retain service-stop success set exactly `{0,1062}`;
- retain normal NSIS/Tauri payload and ARP cleanup;
- retain the frozen post-process requirement that service, payload and registration are all absent after root-process exit code `0`.

No timeout, acceptance predicate, harness cleanup behavior, identity, ACL, Agent, Tauri configuration or later-task scope changes are authorized.

### Proof required for Amendment 007

The exact A007 head must pass frozen authority/parser/dependency validation, all required governance and the complete GitHub-hosted `PKG-03 03.16 Reinstall Repair` workflow. A green run is candidate evidence only. Before `03.16` can become `DONE` or PR #146 can merge, its success ZIP must be independently downloaded, its artifact SHA-256 recomputed against GitHub's digest, `evidence.json` and its declared SHA verified, and every lifecycle/repair/identity/cleanup/MSI-log/zero-drift invariant inspected.
