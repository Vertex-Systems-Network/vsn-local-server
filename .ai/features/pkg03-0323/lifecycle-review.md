# PKG-03 03.23 Lifecycle Review — Provenance and release handoff

Status: **PLANNING-ONLY / BLOCKED**

Canonical preflight base: `e3fb61581646a475c117cc893566286e397c2108`
Task: `03.23`
Linear: `ABD-98`
Lane: `provenance`
Depends on: `03.02`, `03.14`, `03.22`

## Authority

The frozen PKG-03 package plan remains the only task-order authority. This preflight branch may prepare task-local planning knowledge, but 03.23 must remain Backlog/BLOCKED until `03.22` is canonically DONE on `main`.

No task status, package counter, cursor, ready-set, `.ai/state.json`, master execution status or PKG-05 implementation state may change from this preflight.

## Lifecycle

1. **Blocked preflight** — research and handoff constraints may be prepared on task-owned `.ai/features/pkg03-0323/**` paths only.
2. **Dependency unlock** — re-read fresh canonical main after 03.22 merges accepted production-signing evidence. Verify 03.02/03.14/03.22 are DONE and 03.23 is actually READY.
3. **Evidence reconciliation** — bind the exact accepted 03.22 source/run/job/artifact and signed subject hashes. Reject test/self-signed or stale-head evidence.
4. **Fresh standards/tool delta** — re-check current GitHub attestation behavior plus selected SBOM format/generator support. Material drift requires plan change-control before implementation.
5. **Freeze task authority** — create/finalize 03.23 task plan, manifest, workflow/validator contract and exact hashes from the unlocked canonical base.
6. **Implementation/certification** — generate deterministic subject hashes, SBOM/provenance, attest/verify where supported, and create the PKG-05 release-handoff manifest without changing accepted installer bytes.
7. **Independent verification** — parse generated SBOM/manifests, verify every subject digest against accepted signed artifacts, verify provenance/attestation identity, scan evidence for secret material, and prove zero unauthorized tracked drift.
8. **Same-PR state projection** — only after genuine exact-head evidence passes may the PR project 03.23 DONE and unlock 03.24.
9. **Guarded merge** — final task-specific + repository governance checks must be green on the exact final head; merge with `expected_head_sha`, then immediately re-read main.

## Ownership boundary

03.23 may own, after activation:
- `.ai/features/pkg03-0323/**`;
- a task-specific `.ai/plans/pkg03-0323-*` plan;
- a task-specific `.ai/manifests/pkg03-0323-*` manifest;
- `.github/workflows/pkg03-0323-*`;
- `scripts/ci/pkg03-0323-*`;
- task-specific provenance/SBOM/handoff documentation and generated evidence staging paths;
- canonical projection files only after accepted evidence and only in the same task PR.

03.23 must not mutate:
- `scripts/ci/pkg03-0322-*` or `.github/workflows/pkg03-0322-*`;
- production signing credential handling or signer identity;
- accepted installer/product payload bytes;
- package identity/version/upgrade code;
- service, ACL, firewall/hosts/resolver/trust-store, runtime or reboot behavior;
- updater/recovery implementation (PKG-04);
- Linux/macOS implementation (PKG-05).

## Acceptance model

03.23 acceptance must fail closed unless all of the following are true on one exact source head:
- canonical dependencies are DONE;
- accepted 03.22 production-signed subjects are identified and digest-bound;
- SHA-256 manifest is deterministic and independently reproduced;
- SBOM is schema-valid using the frozen selected format/version;
- SBOM describes the locked build/dependency graph and is digest-bound to the accepted release handoff;
- provenance/attestation is generated and independently verified using the frozen exact tool/action versions where the platform supports it;
- no subject bytes change during SBOM/provenance generation;
- evidence contains no secret/private-key/token material;
- PKG-05 handoff identifies exactly the accepted Windows subjects, hashes, SBOM/provenance digests and source/evidence lineage;
- zero unauthorized repository drift is proven.

## Stop conditions

Stop rather than improvise if:
- 03.22 is not canonically DONE;
- accepted signed artifacts are unavailable/expired without a governed reproduction path;
- signed subject hashes differ from 03.22 evidence;
- the selected SBOM/provenance tooling cannot verify deterministically;
- a standards/tooling change requires a material scope or trust-boundary change;
- implementation would need to mutate installer/product/shared signing surfaces.
