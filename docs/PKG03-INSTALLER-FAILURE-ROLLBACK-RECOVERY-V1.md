# PKG-03 03.18 — Installer Failure Rollback & Interrupted Recovery Contract v1

Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.18`
Linear: `ABD-93`

## Contract

A failed or interrupted installation is acceptable only when the resulting machine can be shown, from exact evidence, to be either:
- cleanly rolled back to the pre-install product-absent state; or
- deterministically recoverable by rerunning the exact candidate into one coherent complete install with no duplicate identity.

A nonzero exit code alone is not rollback proof.

## Failure invariants

Forced failure must not leave new package-owned executables, ARP/ProductCode registration, owned shortcuts, `VSN-Agent` registration, widened ACLs, or protected network/trust mutation. Pre-existing sentinels used to force failure must be byte-identical afterward.

## Interruption invariants

The harness must prove the installer had started before terminating installer-owned processes. It must inventory residue before recovery, then rerun the exact package without manually deleting product residue. Recovery succeeds only when the final installed state satisfies accepted package/service/integrity contracts with one identity.

## MSI rule

Windows Installer rollback must be enabled. Verbose logs must demonstrate the failing/recovery transaction and be evidence-bound.

## NSIS rule

No transactional guarantee is assumed. The exact generated installer must prove clean failure/recovery behavior on the runner; otherwise 03.18 fails closed for change control.

## Nonclaims

No live-running product coordination, Restart Manager, reboot, silent deployment, signing, updater recovery, or later-package behavior is claimed.
