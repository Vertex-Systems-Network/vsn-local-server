# PKG-03 03.24 Development Preflight

Status: **PLANNING-ONLY / BLOCKED — NO IMPLEMENTATION AUTHORITY**

Canonical preflight base: `e3fb61581646a475c117cc893566286e397c2108`
Task: `03.24 — Fresh and dirty Windows VM install/repair/uninstall acceptance matrix`
Linear: `ABD-99`
Branch: `pkg03/0324-vm-matrix-preflight`

## Current gate

Required DONE set: `03.16`, `03.17`, `03.18`, `03.19`, `03.20`, `03.21`, `03.22`, `03.23`.

Current unresolved chain: `03.22` -> `03.23` -> `03.24`.

Result: 03.24 remains BLOCKED. This branch owns planning only.

## Conflict-free work allowed now

- Maintain `.ai/features/pkg03-0324/**` planning artifacts.
- Define provisional matrix dimensions and evidence fields.
- Inspect prior accepted task contracts read-only.
- Evaluate VM/test infrastructure capabilities without creating product changes.

## Forbidden now

- No implementation workflow/harness claiming acceptance.
- No canonical tracker/master/README projection.
- No installer/product/signing/provenance mutation.
- No PKG-04 updater/recovery or PKG-05 release implementation.
- No acceptance claim based on unsigned, test-signed or non-03.23-bound artifacts.

## Activation checklist

After 03.23 is canonical DONE:
1. reconcile branch onto fresh main;
2. prove all frozen dependencies DONE and 03.24 READY;
3. bind exact 03.23 handoff manifest and accepted package hashes;
4. verify production signatures and SBOM/provenance before VM execution;
5. freeze Windows image/build identities and clean/dirty seed mechanics;
6. freeze matrix rows, expected transitions, timeouts and cleanup assertions;
7. create task plan/manifest + exact-head task workflow/validator;
8. execute certification-first with no product change initially;
9. independently verify aggregate evidence and zero unauthorized drift;
10. project DONE only on the exact accepted head, then guarded merge.

## Provisional matrix dimensions

Rows should be derived from already accepted behavior rather than creating new requirements. Dimensions include:
- installer: current-user NSIS / per-machine NSIS / MSI;
- starting state: fresh / accepted installed / repair-needed tamper / preserved-user-data dirty state / supported running-resource or reboot-related state;
- action: install / reinstall-repair / uninstall;
- interaction: interactive where owned by prior tasks and strict silent where owned by 03.21;
- expected privilege/scope, service, registration, shortcut, install-root, user-data and cleanup outcomes.

The final matrix is not frozen until activation-time reconciliation.

## Exact next action

Remain blocked while 03.22 and 03.23 complete. On unlock, reconcile to fresh main and replace provisional rows with an exact candidate-bound matrix before executing any VM acceptance workload.
