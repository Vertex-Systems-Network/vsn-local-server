# PKG-03 03.24 Lifecycle Review — Windows VM acceptance matrix

Status: **PLANNING-ONLY / BLOCKED**
Canonical preflight base: `e3fb61581646a475c117cc893566286e397c2108`
Task: `03.24`
Linear: `ABD-99`
Lane: `e2e`

## Authority

03.24 may not activate until every frozen dependency is canonically DONE: `03.16`, `03.17`, `03.18`, `03.19`, `03.20`, `03.21`, `03.22`, `03.23`.

This preflight does not change the tracker, cursor, ready set, package counter or master state.

## Lifecycle

1. **Blocked planning** — maintain only task-local research/lifecycle/preflight.
2. **Unlock** — after 03.23 merges, re-read fresh main and prove all dependencies DONE plus 03.24 READY.
3. **Candidate binding** — consume the exact 03.23 handoff manifest, signed package hashes, SBOM/provenance and source/evidence lineage.
4. **Infrastructure freeze** — define the exact Windows image(s), snapshot/seed method, test isolation, reboot-persistence path where required, and timeout/diagnostic policy.
5. **Matrix freeze** — enumerate every fresh/dirty row with expected state transitions and previously accepted task ownership.
6. **Certification-first execution** — run the matrix without product mutation initially.
7. **Failure classification** — distinguish product, harness, infrastructure and governance/evidence failures. Product change requires explicit minimal change-control and invalidates stale evidence.
8. **Independent evidence verification** — parse all row results, verify package hashes/signatures/provenance, prove cleanup/non-mutation and bind the aggregate matrix to the exact source head.
9. **Same-PR projection** — only genuine exact-head PASS may mark 03.24 DONE and unlock 03.25.
10. **Guarded merge** — final governance + task-specific gates on exact final head, then `expected_head_sha` merge and fresh main re-read.

## Ownership boundary

After activation 03.24 may own task-local `.ai/features/pkg03-0324/**`, task plan/manifest, task-specific workflow/harness/validator and evidence/docs. It must not edit 03.22 signing helpers, 03.23 provenance generators, accepted installer/product behavior or downstream PKG-04/05 implementation unless a classified defect creates explicit change control.

## Stop conditions

Stop if an accepted package hash/signature/provenance does not match, the VM state is not demonstrably fresh/seeded, a reboot-dependent case cannot be proven by the chosen infrastructure, evidence is from a stale source SHA, or a failure would require widening task scope.
