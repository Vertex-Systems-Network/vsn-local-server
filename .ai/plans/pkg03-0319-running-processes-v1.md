# PKG-03 03.19 — Running Desktop, CLI and Agent handling with Restart Manager/service coordination plan v1

Status: frozen task plan
Task: `03.19`
Linear: `ABD-94`
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Certify safe Windows installer behavior while exact installed Desktop, CLI and Agent resources are actively in use, including Windows Installer Restart Manager evidence and accepted service coordination, without hangs, silent force termination, corruption or partial package state.

## Acceptance

Exact-head Windows evidence must:
1. build/hash all three Windows installer formats;
2. install exact candidates and capture installed payload hashes/identity;
3. start exact installed Desktop and deterministic long-running CLI processes; run `VSN-Agent` for machine formats;
4. bind each PID/image/hash/service state before installer invocation;
5. invoke format-specific reinstall/uninstall while resources remain active;
6. prove the installer itself—not the harness—coordinates shutdown/service quiescence or deterministically blocks before destructive mutation;
7. forbid indefinite hang and silent force-kill behavior;
8. for coordinated completion, prove exact package/service integrity afterward and no duplicate identity;
9. for deterministic block, prove installed state remains byte/identity coherent and retry succeeds after separately recorded operator cleanup;
10. for MSI, bind verbose logs and Restart Manager-related observations/properties;
11. for NSIS, bind visible UI/process/action evidence and exact exit behavior;
12. finish with coherent installed or cleanly uninstalled state and zero tracked repository drift.

## Boundaries

- Reboot/no-restart semantics are 03.20.
- Unattended deployment is 03.21.
- No signing/updater/PKG-04/PKG-05 work.
- Initial v1 is certification-first; product/installer mutation requires bounded change control after exact evidence.

## Governance sequence

Planning head -> five governance gates -> task-owned certification implementation -> exact implementation-head governance + `PKG-03 03.19 Running Processes` -> independent evidence verification -> canonical DONE projection only after accepted evidence.

## Evidence artifact

`pkg03-0319-running-processes`
