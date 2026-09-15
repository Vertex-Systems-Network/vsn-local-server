# PKG-03 03.25 Development Preflight

Status: **PLANNING-ONLY / BLOCKED — NO FINAL-GATE AUTHORITY**

Canonical preflight base: `e3fb61581646a475c117cc893566286e397c2108`
Task: `03.25 — Final Windows installer exact-head gate, full PKG-03 regression matrix and PKG-04 handoff`
Linear: `ABD-100`
Branch: `pkg03/0325-final-gate-preflight`

## Current gate

Frozen dependency set is every task `03.02` through `03.24`. Current unresolved tail is `03.22` -> `03.23` -> `03.24` -> `03.25`.

Result: BLOCKED. No final certification execution or state projection is allowed.

## Conflict-free work allowed now

- Maintain `.ai/features/pkg03-0325/**` planning artifacts.
- Inventory accepted PKG-03 contracts read-only for future regression derivation.
- Define provisional aggregate evidence and PKG-04 handoff fields.
- Keep Linear blockers and authoritative upstream PRs synchronized.

## Forbidden now

- No final-gate workflow claiming acceptance.
- No canonical tracker/master/README/package-completion projection.
- No mutation to installer/product/signing/provenance/VM task surfaces.
- No PKG-04 updater/recovery implementation or activation.
- No weakening/removal of earlier task acceptance to obtain a green final gate.

## Activation checklist

After 03.24 is canonical DONE:
1. reconcile branch onto fresh main;
2. prove every frozen dependency 03.02–03.24 DONE and 03.25 READY;
3. bind exact final source SHA, signed package subjects, 03.23 handoff and 03.24 matrix evidence;
4. derive/freeze exact-head full-package regression coverage from the accepted contracts;
5. freeze PKG-04 non-secret handoff schema;
6. create task plan/manifest + final-gate workflow/harness/validator;
7. execute exact-head aggregate certification;
8. independently verify all evidence, signatures, provenance, cleanup and zero drift;
9. project only 03.25 DONE on accepted evidence and merge with expected SHA;
10. re-read canonical main and only then create the separate state-only PKG-03 COMPLETE / PKG-04-next projection required by the package plan.

## Provisional final evidence sections

- source/toolchain/workflow identity;
- package identity and signed SHA-256 subject list;
- install-scope and ownership invariants;
- service/ACL/data/non-mutation invariants;
- repair/uninstall/rollback/runtime/reboot/silent-deployment results;
- Authenticode publisher/timestamp verification;
- SBOM/provenance and PKG-05 handoff references;
- fresh/dirty VM matrix result;
- governance/check identities;
- secret-leak and tracked-drift verification;
- PKG-04 handoff digest.

These fields remain provisional until activation-time reconciliation.

## Exact next action

Remain blocked while 03.22, 03.23 and 03.24 complete. On unlock, reconcile to fresh main, freeze exact final acceptance authority, and run the aggregate final gate without introducing new product behavior by default.
