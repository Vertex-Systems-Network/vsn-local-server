# PKG-03 03.16 — Bounded Change Control 002

Status: **ACTIVE / evidence-triggered**  
Task: `03.16`  
Linear: `ABD-91`  
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`  
Parent control: `.ai/features/pkg03-0316/change-control-001.md`  
Authorized product path: `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

## Purpose

This addendum authorizes only the smallest product correction required by the exact corrected A009 WiX evidence. All frozen 03.16 acceptance rules and every guardrail in change-control-001 remain unchanged.

## A009 corrected diagnostic evidence

Exact head `0e46b5ef443dc56fd37a97e881de92d955bd6ad7` ran `PKG-03 03.16 Reinstall Repair` as run `33434777496`, job `99628434407`. Frozen authority/parser/dependency validation and all three exact-head package builds succeeded. The lifecycle again failed only at `wix-per-machine initial-install did not reach required state.`

Failure artifact `9774753924` was independently downloaded and recomputed to GitHub's exact SHA-256 `77276b7cdaf1cf8827edaaf65e4f6f5d29bbceba2b801940e93583be9cc99712`. Unlike the first A009 trigger, this artifact contains `wix-per-machine-initial-install.log`, proving the corrected diagnostic injection executed.

The verbose MSI log establishes both of these facts:

1. The MSI is genuinely per-machine (`ALLUSERS=1`, elevated per-machine transaction), and its authored default directory is under `ProgramFiles64Folder`.
2. Before costing, client-side `AppSearch` sets `INSTALLDIR` to `C:\Users\runneradmin\AppData\Local\VSN Dev Platform`; that value is then forwarded to the elevated server transaction. The MSI therefore copies its payload to the prior current-user path instead of Program Files.

The same log shows `Pkg0311InstallService` executes, then `Pkg0311StartService` returns process exit `1`, producing Windows Installer error `1722`, `InstallFinalize` return value `3`, rollback, and final MSI result `1603`. The Agent CLI collapses failed native `sc.exe` statuses into process exit `1`, so this evidence does not identify a native SCM start code. No WiX service-start transport change is authorized by this addendum.

## Deterministic cross-installer cause

Tauri CLI/bundler `2.11.4` upstream templates explain the `INSTALLDIR` mutation exactly:

- the NSIS template writes its installed `$INSTDIR` to the unnamed/default value of `HKCU\Software\<Manufacturer>\<ProductName>` for current-user installs;
- normal NSIS uninstall deletes that vendor/product key only when the user elects to delete application data;
- the frozen 03.16 safety automation explicitly leaves `Delete the application data` **off** during genuine current-user uninstall;
- the WiX template declares a per-machine package and a Program Files directory, but its `INSTALLDIR` property first performs an HKCU `RegistrySearch` for that NSIS default value so NSIS-to-MSI migrations can reuse the previous location;
- therefore the stale current-user NSIS install-location pointer survives the successful current-user uninstall and deterministically overrides the later per-machine MSI's Program Files default.

The run's UI evidence records `ensure-safety-checkbox-off` for control `Delete the application data` during `nsis-current-user` uninstall, followed by the real `Uninstall` action and successful terminal close. Integrity evidence records the complete current-user and per-machine NSIS reinstall/repair phases with exact `MATCH`, `MISSING -> MATCH`, and `HASH_MISMATCH -> MATCH` restoration before WiX begins.

This is installer metadata contamination, not application-data cleanup. Leaving the unnamed install-location pointer after the payload has been genuinely removed creates a false predecessor location for the next installer technology.

## Amendment 010 — clear stale current-user NSIS install-location pointer

Status: **ACTIVE / proof required**

Only `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh` may change.

Authorized correction:

- add a `NSIS_HOOK_POSTUNINSTALL` branch compiled only for `INSTALLMODE == currentUser`;
- after Tauri's normal uninstall body has completed, delete only the unnamed/default value of `HKCU\Software\${MANUFACTURER}\${PRODUCTNAME}`;
- preserve all named values such as `Installer Language`;
- preserve application data and keep the frozen `Delete the application data` checkbox off;
- do not delete the vendor/product key wholesale;
- do not modify per-machine NSIS service behavior retained from A008;
- do not modify the WiX template/fragment, Agent Rust, Tauri configuration, package/service identity, service account/start mode/binPath, ACL/security/network behavior, certification timeout, UI automation, completion predicates, repair assertions, or any 03.17+ scope.

The expected causal effect is narrow: after successful current-user NSIS uninstall, the later per-machine MSI `AppSearch` must no longer inherit the obsolete LocalAppData path and must resolve its authored `INSTALLDIR` under Program Files. If the WiX service still fails after the install root is corrected, that failure must be treated as a newly isolated boundary and diagnosed from fresh exact-head evidence before any service-start product mutation is authorized.

## Proof required

The exact A010 head must pass all required governance, frozen authority/parser/dependency validation, all three exact-head package builds, and the complete GitHub-hosted `PKG-03 03.16 Reinstall Repair` workflow without acceptance weakening.

A green workflow is candidate evidence only. Before `03.16` can become `DONE` or PR #146 can merge, the success artifact must be independently downloaded, its ZIP SHA-256 recomputed against GitHub's reported digest, `evidence.json` and `evidence.json.sha256` verified, and every current-user NSIS, per-machine NSIS, MSI/WiX, repair, identity, service safety, uninstall cleanup, process-exit, MSI `/fa` log, exact-source, and zero-drift invariant inspected.
