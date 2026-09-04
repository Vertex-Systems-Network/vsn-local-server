# PKG-03 03.25 Lifecycle Review — Final package gate

Status: **PLANNING-ONLY / BLOCKED**
Canonical preflight base: `e3fb61581646a475c117cc893566286e397c2108`
Task: `03.25`
Linear: `ABD-100`
Lane: `final`

## Authority

03.25 may activate only after every frozen dependency `03.02` through `03.24` is canonically DONE. This preflight does not change PKG-03 progress, cursor, ready set or package-completion state.

## Lifecycle

1. **Blocked planning** — task-local research/lifecycle/preflight only.
2. **Unlock** — after 03.24 merges, re-read fresh main and prove the complete frozen dependency set DONE plus 03.25 READY.
3. **Candidate freeze** — bind exact final source head, 03.23 provenance/hashes and 03.24 accepted VM evidence.
4. **Regression freeze** — derive the full-package exact-head regression set from all accepted PKG-03 contracts; do not silently drop high-risk gates.
5. **Handoff freeze** — define non-secret PKG-04 input contract without activating or implementing PKG-04.
6. **Final certification** — run aggregate exact-head build/lifecycle/security/non-mutation/signing/provenance/VM regressions and collect evidence.
7. **Independent verification** — prove package hashes/signatures/provenance, expected state transitions, cleanup, no secret leakage and zero unauthorized tracked drift.
8. **03.25 state projection** — only exact-head PASS may mark 03.25 DONE. PKG-03 itself is still not COMPLETE until the separate package completion projection required by the frozen plan is merged.
9. **Guarded merge** — task PR final head must pass repository + AI planning + engineering + operational governance, PKG-03 sequence and task-specific final gate; merge with expected SHA.
10. **Post-merge package completion projection** — re-read main, verify all 25 tasks canonically DONE, then create the separate state-only PKG-03 COMPLETE / PKG-04-next projection. No product mutation in that projection.

## Ownership boundary

After activation, 03.25 may own task-local planning, final-gate workflow/harness/validator/evidence and PKG-04 handoff documentation. It must not change product/installer/signing/provenance behavior merely to make the final gate green; classified defects return to the smallest owning boundary with fresh evidence.

## Stop conditions

Stop if any dependency is not DONE, any final subject/hash/provenance differs from accepted lineage, a required high-risk regression cannot execute, evidence is stale or incomplete, a fix would weaken an earlier acceptance contract, or PKG-04 implementation would be needed before PKG-03 completion.
