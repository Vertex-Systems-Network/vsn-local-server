# PKG-03 03.16 — Bounded Change Control 002

Status: **ACTIVE / evidence-triggered**  
Task: `03.16`  
Linear: `ABD-91`  
Scope: per-machine NSIS uninstall service-removal completion only

## Trigger evidence

Exact source head `5bc2150c74a04dc5f893737b618dd7fa84ad0eb1` failed GitHub-hosted Windows run `33352416537`, job `99368202715`, only in the genuine `nsis-per-machine` uninstall phase:

`nsis-per-machine uninstall did not reach required state.`

All authority/parser checks and all three package builds passed. The failure artifact `9744386201` (`pkg03-0316-reinstall-repair-failure`) is bound to that exact head. GitHub reports SHA-256 `f2a8971ecc3f475f6e65880618e74595a658b90a7170a37717646e7ddab0e658`, independently recomputed from the downloaded ZIP to the same value.

The artifact contains fifty identical per-machine terminal progress probes spanning approximately six minutes. Every probe records:

- the genuine NSIS content `Close` control present but disabled;
- `VSN-Agent` service state `Stopped`;
- machine payload still present;
- HKLM uninstall registration still present;
- no live `vsn-agent.exe` helper PID;
- no live `sc.exe` PID.

Current-user uninstall is the control case: it reaches the same terminal UI family, cleanup state becomes complete, the real content `Close` becomes enabled and the frozen harness completes normally. The per-machine failure is therefore a product lifecycle defect before terminal completion, not authority to force a disabled UI control or weaken acceptance.

Source inspection at the same exact head establishes the relevant product path:

1. `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh` runs the accepted per-machine `NSIS_HOOK_PREUNINSTALL` sequence: `vsn-agent.exe service stop`, then `vsn-agent.exe service uninstall`, aborting on non-zero status.
2. `apps/agent/src/main.rs` implements `service uninstall` / `service remove` as a thin synchronous wrapper around `sc.exe delete VSN-Agent`; it does not contain additional removal semantics.
3. Run-52 telemetry proves the service has reached `Stopped` and neither wrapper nor `sc.exe` remains running, while service registration/payload teardown never advances for the remainder of the timeout.

This evidence narrows the defect to the per-machine NSIS service-removal boundary. `CC-0316-001` does not authorize this mutation, so this separate control is required.

## Authorized mutation

Exactly one product-input path may change under this control:

- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

The allowed behavior change is limited to making the per-machine `NSIS_HOOK_PREUNINSTALL` service-removal step explicit and completion-aware inside NSIS:

- preserve the accepted service stop operation and fail closed on a real stop failure;
- remove `VSN-Agent` through the Windows Service Control Manager without depending on the installed Agent executable as a second removal wrapper;
- treat only the already-absent service as an idempotent success;
- fail closed on any other service-delete failure;
- verify that `VSN-Agent` is no longer queryable before permitting installer payload/ARP cleanup to continue;
- keep the change bounded to the `perMachine` uninstall hook.

The implementation may use Windows-native `sc.exe` commands already relied on by the product, but must not add a new runtime, helper executable, PowerShell dependency, acceptance shim, or signing secret.

## Explicitly forbidden

- no change to service name, account, start mode, binary path, display name, description or security identity;
- no Agent Rust payload mutation, including `apps/agent/src/main.rs`;
- no current-user install/uninstall behavior change;
- no post-install service-registration behavior beyond `CC-0316-001`;
- no Tauri configuration/template replacement;
- no ACL/security-state mutation;
- no manual payload or HKLM ARP deletion added to the certification harness;
- no force-click of disabled installer controls, fake completion signal, skipped cleanup predicate, timeout inflation or weakened process-exit/exit-code requirement;
- no running-process/Restart Manager, dirty-data, rollback, reboot, unattended deployment, signing, updater, PKG-04 or PKG-05 scope;
- production signing material remains external and unavailable signing must continue to fail closed.

## Required proof after mutation

The exact changed head must rerun the 03.16 authority/governance gates and `PKG-03 03.16 Reinstall Repair`. Acceptance remains unchanged: all three installer lifecycles must pass; missing/tampered payloads must be restored to exact accepted SHA-256 bytes; service/registration/ACL identity must remain stable; uninstall must prove service, payload and registration cleanup; root processes must exit with the required successful status; tracked drift must remain zero.

A successful Windows run is not sufficient by itself. Its exact-head evidence artifact and reported digest must be independently downloaded, recomputed and inspected before `03.16` may be marked DONE or PR #146 may be merged.

`CC-0316-001` remains authoritative for its post-install idempotence mutation. This control extends authority only for the proven per-machine uninstall service-removal defect above and authorizes no dependent canonical task.