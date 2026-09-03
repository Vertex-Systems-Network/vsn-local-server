# PKG-03 03.23 Development Preflight

Status: **PLANNING-ONLY / BLOCKED — NO IMPLEMENTATION AUTHORITY**

Canonical preflight base: `e3fb61581646a475c117cc893566286e397c2108`
Task: `03.23 — Installer hash, SBOM/provenance manifest and PKG-05 release handoff`
Linear: `ABD-98`
Branch: `pkg03/0323-provenance-preflight`

## Dependency gate

Frozen dependencies:
- `03.02` DONE
- `03.14` DONE
- `03.22` NOT DONE at this preflight checkpoint

Result: **BLOCKED**. No 03.23 implementation workflow, generated release evidence, state projection or DONE claim is authorized yet.

## Conflict-free work allowed before unlock

- Maintain task-local research/lifecycle/preflight under `.ai/features/pkg03-0323/**`.
- Inspect accepted historical 03.02/03.14 evidence contracts read-only.
- Define the future handoff schema conceptually without binding nonexistent 03.22 production evidence.
- Monitor official SBOM/provenance specification/tool changes.
- Keep Linear planning notes synchronized with the authoritative 03.22 PR.

## Forbidden before unlock

- Do not edit 03.22 planning, signing helpers, workflow or evidence.
- Do not generate or commit a fake/final SBOM from unsigned or test-signed candidates.
- Do not mark 03.23 READY/IN_PROGRESS/DONE in canonical Git state.
- Do not touch canonical tracker/master status/README projection files.
- Do not mutate product/runtime/installer/package identity/service/ACL/network/reboot surfaces.
- Do not implement PKG-04 updater/recovery or PKG-05 Linux/macOS release work.
- Do not add signing secrets, PFX files, passwords, tokens or private-key material.

## Activation checklist after 03.22 merges

Before the first implementation mutation:
1. fetch fresh `main` and verify it includes accepted 03.22 merge;
2. verify tracker shows 03.02/03.14/03.22 DONE and 03.23 READY;
3. record accepted 03.22 source SHA, run, job, artifact ID/digest and production-signed subject hashes;
4. confirm all 03.22 subjects verify with the expected publisher/timestamp contract;
5. refresh official GitHub attestation + SBOM format/tool compatibility;
6. select and freeze one exact SBOM format/version and exact generator/action versions;
7. define deterministic handoff schema and independent verifier;
8. freeze the 03.23 task plan/manifest hashes;
9. create/continue the authoritative 03.23 implementation PR from the fresh canonical base;
10. run exact-head certification before any state projection.

## Provisional PKG-05 handoff schema

The activation-time frozen schema should include only non-secret release lineage fields such as:
- schema/task/package version;
- source commit;
- upstream 03.22 evidence identity;
- package filename/type/size/SHA-256;
- expected publisher and signature/timestamp verification result references;
- SBOM format/spec/generator and SBOM SHA-256;
- provenance/attestation predicate and verification reference;
- build workflow/repository identity;
- generation timestamp in a deterministic/normalized representation where required;
- handoff manifest SHA-256.

This schema is provisional and cannot be treated as frozen acceptance authority until 03.22 is canonically accepted and activation-time reconciliation completes.

## Exact next action

Remain blocked while PR #162 / 03.22 executes. Once 03.22 becomes canonical DONE, reconcile this branch onto the new main, replace provisional assumptions with accepted signed evidence, freeze the implementation plan/manifest, then begin the provenance lane without touching 03.22-owned files.
