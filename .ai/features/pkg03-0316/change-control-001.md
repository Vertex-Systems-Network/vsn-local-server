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

Status: **EVIDENCE-TESTED / REFINED BY A005–A008**

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

Status: **EVIDENCE-TESTED / NATIVE RESULT WAS NOT EXPOSED**

The frozen 03.16 lifecycle deliberately calls `Stop-AgentForRepair` before destructive machine uninstall. Product `NSIS_HOOK_PREUNINSTALL` then invokes `vsn-agent.exe service stop` again. Native `ERROR_SERVICE_NOT_ACTIVE (1062)` is the idempotent SCM result for an already-stopped service, so treating that native state as fatal would be incorrect for the frozen precondition.

A006 attempted to accept service-stop exit `0` or exactly `1062` at the NSIS hook boundary and fail closed on every other result.

Exact head `e852d54aaeebfb9bee30fb87c9db293a9274b1e2` executed A006 in GitHub-hosted Windows run `33406302044`, job `99534626588`. All five required governance gates, frozen authority/parser checks and all three exact-head package builds passed. Genuine per-machine uninstall still failed to reach required cleanup. Failure artifact `9764176057` was independently downloaded and recomputed to GitHub's exact SHA-256 `04706d62e767b73ba85536b80ab7509481e288a9790ed677fef1ec8532878607`. Its per-machine uninstall evidence contains 315 repeated progress probes after the real Uninstall action; every probe reports `VSN-Agent=Stopped`, machine payload present, HKLM ARP present, and no `vsn-agent.exe` or `sc.exe` process.

A008 source audit subsequently proved that `vsn-agent.exe service stop` collapses any failed native `sc.exe` status into the process-level generic `ExitCode::FAILURE` (`1`). Therefore A006 did **not** actually expose or test native `1062` at the NSIS boundary; its earlier inference is superseded by A008.

## Amendment 007 — already-marked-for-delete idempotence

Status: **EVIDENCE-TESTED / INSUFFICIENT; PREMISE SUPERSEDED BY A008**  
Additional scope: per-machine NSIS native service-delete result classification only

### Causal decision tested by A007

After A006, the working hypothesis was that the preuninstall hook had only one remaining product-side fail-closed boundary before Tauri-owned payload/ARP cleanup: native `sc.exe delete VSN-Agent` accepted `0` or already-absent `1060`, while every other return executed `Abort`.

Windows SCM defines `ERROR_SERVICE_MARKED_FOR_DELETE (1072)` for a service record on which deletion has already been requested. A007 therefore retained direct `sc.exe delete`, additionally accepted exactly `1072`, and kept all other delete results fail closed. Final database removal remained required only at the unchanged post-process acceptance boundary.

### A007 result

Exact head `049fc9673338ed27ce644c04ea0e4d832bc8f5b6` executed A007 in GitHub-hosted Windows run `33409818394`, job `99546311766`. Frozen authority/parser validation and all three exact-head package builds succeeded, but genuine `nsis-per-machine` uninstall again failed with `nsis-per-machine uninstall did not reach required state.` Failure artifact `9765516734` was independently downloaded and recomputed to GitHub's exact SHA-256 `2629ec66d5e4209237427412fc0ead16907b2d8c9f1a53cfbd50f2e772351600`.

The artifact records the real Uninstall activation at `15:59:58.603Z`; by `15:59:59.407Z` the content `Close` control was already disabled. It then records 313 consecutive progress probes through the timeout, all with `VSN-Agent=Stopped`, machine payload present, HKLM registration present, and no `vsn-agent.exe` or `sc.exe` helper process. Runner cleanup terminated the still-live `Un` process. A007 therefore did not resolve the product boundary, and A008 source audit invalidates the assumption that the stop result had already been natively classified before delete.

## Amendment 008 — preserve native SCM stop result at the NSIS boundary

Status: **ACTIVE / RETAINED; EFFECTIVE FOR NSIS**  
Additional scope: per-machine NSIS service-stop transport only

### Causal decision

The A006/A007 evidence consistently enters failed uninstall progress almost immediately after the real Uninstall action, with the service already `Stopped`, payload/ARP untouched and no helper process left alive. Exact source inspection supplies the missing deterministic link:

1. `NSIS_HOOK_PREUNINSTALL` invoked `"$INSTDIR\bin\vsn-agent.exe" service stop` and attempted to classify `{0,1062}`.
2. `service_command` returns `ExitCode::SUCCESS` only when `windows_service_host::manage` returns `Ok`; every `Err` is converted to generic `ExitCode::FAILURE`.
3. Windows `manage(... "stop" ...)` calls its `sc(&["stop", SERVICE_NAME])` helper.
4. That helper turns every non-successful `sc.exe` process status into `Err(...)` rather than propagating the native SCM code.
5. Consequently an already-stopped native `ERROR_SERVICE_NOT_ACTIVE (1062)` is observed by NSIS as process exit `1`, which immediately takes the fail-closed `Abort` branch before delete and Tauri cleanup.

### Authorized correction

Only `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh` may change.

For `NSIS_HOOK_PREUNINSTALL` / `perMachine` stop only:

- use direct `"$SYSDIR\sc.exe" stop VSN-Agent` so the NSIS hook receives the native SCM process result;
- accept exit `0` and exactly `1062`;
- fail closed on every other stop result;
- retain direct delete result set exactly `{0,1060,1072}`;
- retain `SetAutoClose true`, normal NSIS/Tauri payload and ARP cleanup, and the frozen post-process requirement that service, payload and registration are absent only after root-process exit code `0`.

No Agent Rust, payload, Tauri configuration, package/service identity, ACL/security/network, certification timeout, cleanup shim, acceptance predicate or 03.17+ scope change is authorized.

### A008 result

Exact head `db80a67555d614dfdaaff87a74a50ffd1ca150de` executed GitHub-hosted run `33429865150`. Failure artifact `9772949341` was independently downloaded and recomputed to GitHub's exact SHA-256 `727ecab6981eda25e2e0255603aed2c14abc3683e1e2b512fc6c32f052e0773c`.

The current-user NSIS lifecycle and the complete per-machine NSIS initial install, healthy same-version reinstall, missing-file exact repair, tamper exact repair, second healthy pass and genuine uninstall all crossed their prior boundary. The run then reached WiX initial install. A008 is therefore retained as effective for the NSIS blocker; full 03.16 remained blocked downstream in WiX.

## Amendment 009 — verbose WiX initial-install diagnostic overlay

Status: **DIAGNOSTIC / EVIDENCE HARVESTED**  
Product mutation: **none**

The first A009 trigger at head `479f9eca51b6154278a2f7e525640ca522867c96`, run `33432803244`, artifact `9773738639`, independently verified SHA-256 `59df5492c3a5ce2f4d24c11300daef5379e632cde4d214b8d2a03d96d30997b1`, was invalid as product evidence because the diagnostic quote anchor matched the canonical WiX start block zero times before lifecycle execution. The artifact contained packages only and no native MSI log.

The corrected diagnostic head `0e46b5ef443dc56fd37a97e881de92d955bd6ad7` executed run `33434777496`, job `99628434407`. Frozen authority/parser/dependency validation and all three exact-head package builds passed. The lifecycle again failed only at `wix-per-machine initial-install did not reach required state.` Failure artifact `9774753924` was independently downloaded and recomputed to GitHub's exact SHA-256 `77276b7cdaf1cf8827edaaf65e4f6f5d29bbceba2b801940e93583be9cc99712`. This artifact contains `wix-per-machine-initial-install.log`, proving the corrected `/l*v` injection executed.

### A009 causal evidence

The native MSI log establishes:

- the package is genuinely per-machine (`ALLUSERS=1`) and its authored default directory is under `ProgramFiles64Folder`;
- before costing, client-side `AppSearch` changes `INSTALLDIR` to `C:\Users\runneradmin\AppData\Local\VSN Dev Platform`;
- that LocalAppData value is forwarded to the elevated server transaction, so the per-machine MSI copies its payload to the previous current-user path instead of Program Files;
- `Pkg0311InstallService` executes;
- `Pkg0311StartService` returns process exit `1`, Windows Installer emits error `1722`, `InstallFinalize` returns value `3`, rollback runs, and MSI ends `1603`.

The Agent CLI collapses failed native `sc.exe` statuses into generic process exit `1`, so A009 does **not** identify a native SCM start code. No WiX service-start transport change is authorized from this evidence alone.

### Deterministic Tauri 2.11.4 cross-installer cause

Exact upstream Tauri 2.11.4 templates bind the install-root failure:

1. The NSIS template writes the installed `$INSTDIR` to the unnamed/default value of `HKCU\Software\<Manufacturer>\<ProductName>` for a current-user installation.
2. Its normal uninstall removes that vendor/product location key only when the operator elects to delete application data.
3. Frozen 03.16 UI safety explicitly leaves `Delete the application data` **off** during genuine current-user uninstall, while still requiring payload and ARP removal.
4. The WiX template is per-machine and authors Program Files as the default `INSTALLDIR`, but its `INSTALLDIR` property first performs an HKCU `RegistrySearch` for the NSIS unnamed/default install-location value to support installer migration.
5. Therefore the successfully removed current-user installation leaves a stale installer-location pointer which deterministically overrides the later per-machine MSI's Program Files default.

This is installer metadata contamination, not application-data cleanup.

## Amendment 010 — clear stale current-user NSIS install-location pointer

Status: **ACTIVE / PROOF REQUIRED**  
Authorized product path remains exactly: `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

### Authorized correction

Within the already-authorized hook file only:

- add `NSIS_HOOK_POSTUNINSTALL` compiled only for `INSTALLMODE == currentUser`;
- after Tauri's normal uninstall body completes, delete only the unnamed/default value of `HKCU\Software\${MANUFACTURER}\${PRODUCTNAME}`;
- preserve every named value, including `Installer Language`;
- preserve all application data and keep the frozen `Delete the application data` safety checkbox off;
- do not delete the vendor/product key wholesale;
- retain A008 per-machine NSIS service behavior unchanged;
- do not modify WiX template/fragment, Agent Rust, Tauri configuration, package/service identity, service account/start mode/binPath, ACL/security/network behavior, certification timeout, UI automation, completion predicates, repair assertions, or any 03.17+ scope.

Expected causal effect: after successful current-user NSIS uninstall, a later per-machine MSI must no longer inherit the obsolete LocalAppData path from the NSIS migration registry value and must resolve its authored Program Files target. If WiX service start still fails after the install root is corrected, that is a newly isolated boundary requiring fresh exact-head evidence before any additional product mutation.

### A010 first trigger — authority-only invalid run

Head `98ec4c808e79513bec0bf30a2c0099ae0366f958`, run `33438484287`, job `99640625468` did **not** build or execute A010 product behavior. Frozen authority validation stopped the run because an additional documentation file `.ai/features/pkg03-0316/change-control-002.md` had been introduced outside the validator's explicitly frozen path set. The exact error was:

`03.16 branch changed unauthorized paths: ['.ai/features/pkg03-0316/change-control-002.md']`

This is governance evidence only, not a product failure. The manifest, validator and authority are not widened. A009/A010 history is instead recorded in this existing authorized CC-0316-001 artifact, and the temporary extra addendum is removed from the branch.

### Proof required for Amendment 010

The exact governance-compatible A010 head must pass frozen authority/parser/dependency validation, all required governance and the complete GitHub-hosted `PKG-03 03.16 Reinstall Repair` workflow. Acceptance remains unchanged.

A green run is candidate evidence only. Before `03.16` can become `DONE` or PR #146 can merge, its success ZIP must be independently downloaded, its artifact SHA-256 recomputed against GitHub's digest, `evidence.json` and its declared SHA verified, and every current-user NSIS, per-machine NSIS, MSI/WiX, repair, identity, service safety, uninstall cleanup, root-process exit, MSI `/fa` log, exact-source and zero-drift invariant independently inspected.
