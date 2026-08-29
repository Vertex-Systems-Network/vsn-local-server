# PKG-03 03.16 Lifecycle Review — Reinstall and repair

Reviewed: 2026-08-30
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.16`
Linear: `ABD-91`

## Lifecycle matrix

| Format | Healthy idempotent pass | Missing-file repair | Tampered-file repair | Quiescence / service rule |
| --- | --- | --- | --- | --- |
| NSIS current-user | rerun exact same setup; require success and stable identity/hashes | Desktop + CLI + Agent allowed | Desktop + CLI + Agent allowed | no machine Agent service may be created |
| NSIS per-machine | rerun exact same elevated setup; require success and stable identity/hashes | Desktop + CLI | Desktop + CLI | stop `VSN-Agent` before destructive probe/repair; verify same service identity/health after repair |
| MSI/WiX per-machine | native Windows Installer repair of exact installed ProductCode; require success and stable identity/hashes | Desktop + CLI | Desktop + CLI | stop `VSN-Agent` before repair; verify same service identity/health after repair |

## Required phase order per lifecycle

1. clean install exact candidate and capture package/product/install-root identity;
2. capture expected SHA-256 for accepted owned executables;
3. healthy idempotent reinstall/repair;
4. verify exact hashes and registration cardinality are unchanged;
5. create one bounded `MISSING` probe and certify exact restoration;
6. return to healthy state;
7. create one bounded `HASH_MISMATCH` probe and certify exact restoration;
8. run a second healthy reinstall/repair and verify no duplicate state;
9. verify applicable service identity/health and accepted ACL/state-location invariants;
10. perform normal certification cleanup and prove zero tracked repository drift.

## Integrity boundary

03.14 remains the authority for pre-repair classification. 03.16 must record the exact `MATCH`/`MISSING`/`HASH_MISMATCH` state immediately before and after each repair operation and bind expected/restored hashes into evidence.

A repair pass is accepted only when the damaged owned file is restored to the exact candidate SHA-256. “Process returned success” without exact byte restoration is a failure.

## Service / running-process boundary

03.16 does not certify repair while Desktop, CLI, or Agent are actively using files. Per-machine destructive probes occur only after `VSN-Agent` is intentionally stopped. After repair, the accepted `VSN-Agent` SCM name, display name, account, automatic-start configuration, executable path, and bounded health behavior must remain compatible with 03.11.

Any Restart Manager or live-running process coordination claim remains owned by 03.19.

## State and ACL boundary

Repair must not:
- broaden SYSTEM/Administrators/LocalService ACL floors established by 03.12;
- relocate accepted machine security state;
- create machine-wide security state from a current-user NSIS lifecycle;
- create duplicate shortcuts, ARP entries, services, or install roots.

This is not the comprehensive dirty-user-data preservation matrix; 03.17 retains that scope.

## Format-specific behavior

### MSI/WiX

Use documented Windows Installer repair semantics against the exact installed ProductCode. Force-reinstall semantics may be used for damaged-file restoration because checksum repair is not assumed for files lacking MSI checksum metadata. Preserve `/L*V` logs and exact process exit codes as evidence.

### NSIS

Use the exact generated candidate setup executable. No custom repair executable, template fork, plugin, or new product hook is authorized. Same-version rerun must demonstrate real restoration on the runner. If it does not, the task fails closed for change control instead of fabricating repair evidence.

## Nonclaims

03.16 does not certify:
- repair with running product processes or Restart Manager coordination;
- interrupted-install rollback/recovery;
- dirty-user-data uninstall preservation;
- reboot-required/no-restart semantics;
- unattended/silent deployment;
- signing, updater, PKG-04 recovery, or cross-platform release behavior.
