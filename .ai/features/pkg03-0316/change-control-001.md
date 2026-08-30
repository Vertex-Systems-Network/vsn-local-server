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
