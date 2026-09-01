# PKG-03 03.18 Lifecycle Review — Failure rollback and interrupted recovery

Reviewed: 2026-08-30
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.18`
Linear: `ABD-93`

## Lifecycle matrix

| Format | Forced-failure proof | Interrupted-install proof | Recovery proof |
| --- | --- | --- | --- |
| NSIS current-user | deterministic install-root conflict causes genuine setup failure; no partial owned registration | terminate active generated setup after positive start observation | rerun exact setup; require one coherent complete install, then ordinary cleanup |
| NSIS per-machine | same under elevated machine scope; no partial `VSN-Agent`/security mutation | terminate installer-owned setup process only | rerun exact setup; require service/security invariants and one coherent install |
| MSI/WiX per-machine | force genuine Windows Installer failure with rollback enabled and `/L*V` log | interrupt client transaction after positive start evidence and wait for Windows Installer quiescence | rerun exact MSI; require valid complete install and deterministic cleanup |

## Required post-failure absence

After a forced failure, unless it was present in the preflight fixture, the following must be absent:
- accepted package-owned executables;
- product ARP/ProductCode registration;
- owned shortcuts/application registration;
- `VSN-Agent` service registration;
- machine IPC security state created only by the failed attempt.

Pre-existing failure-injection sentinels must remain byte-identical.

## Interrupted-state handling

Interruption is not accepted merely because a process was killed. Evidence must show:
1. installer transaction was active;
2. exact installer-owned processes targeted by termination;
3. bounded post-termination residue inventory;
4. subsequent exact package rerun resolves that residue into one valid installed state or a clean fail-closed state;
5. no duplicate registration/service/shortcut identity results;
6. final ordinary uninstall returns the runner to the expected product-absent state.

## Nonclaims

03.18 does not certify live-running VSN process coordination, Restart Manager, reboot-required behavior, unattended deployment, signing, updater recovery, or cross-platform behavior.
