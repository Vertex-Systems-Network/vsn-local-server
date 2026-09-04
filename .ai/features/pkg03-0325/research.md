# PKG-03 03.25 Research — Final Windows installer exact-head gate and PKG-04 handoff

Status: **PLANNING-ONLY / BLOCKED**
Reviewed: 2026-09-04
Canonical preflight base: `e3fb61581646a475c117cc893566286e397c2108`
Task: `03.25`
Linear: `ABD-100`

## Frozen dependency contract

03.25 depends on every implementation/certification task `03.02` through `03.24`. It is the final PKG-03 acceptance gate, not a shortcut around any prior task.

Current unresolved chain includes 03.22 -> 03.23 -> 03.24 -> 03.25. Therefore no final-gate implementation or completion projection is authorized.

## Preflight finding

03.25 should be an **exact-head aggregate certification** over the final accepted Windows installer candidate. It must not introduce new installer behavior by default. Its role is to prove that the complete PKG-03 contract still holds together on one source head and to produce the governed handoff required for PKG-04 activation.

The final gate must consume rather than recreate authority from prior tasks:
- deterministic build/identity/scope/ownership contracts;
- NSIS/MSI install lifecycles;
- Desktop/CLI/Agent placement and service behavior;
- ACL/data-separation/non-mutation boundaries;
- integrity/diagnostics/repair/uninstall/rollback/runtime/reboot semantics;
- strict silent deployment;
- production Authenticode acceptance;
- hashes/SBOM/provenance handoff;
- fresh/dirty VM matrix acceptance.

## Candidate-bound acceptance principle

All final regressions must run against the same exact source head and the exact production-signed package subjects bound by the accepted 03.24 matrix / 03.23 provenance chain. A green historical task run on another SHA is supporting lineage, not a substitute for the final exact-head regression subset.

The final gate must preserve **subject-byte identity** as well as source identity:
- exact filename/role/size/SHA-256 must match the accepted 03.23 handoff and 03.24 matrix evidence;
- accepted Authenticode publisher/timestamp verification must remain valid for those exact bytes;
- a deterministic rebuild may be used as a reproducibility comparison where the frozen contract requires it, but rebuilt bytes cannot silently replace the already accepted production-signed subjects;
- if any required final test needs different package bytes, the candidate lineage is broken and the failure must return to the smallest owning task/change-control boundary before final acceptance can continue.

## Upstream evidence continuity

03.25 must consume the durable evidence identities established upstream rather than only textual PASS claims. Activation-time final authority should bind, at minimum:
- 03.22 trusted production-signing source/run/job/artifact identity, signed subject SHA-256 values and production verification evidence;
- 03.23 handoff manifest digest, SBOM digest/schema/generator identity, provenance predicate, independently verified attestation subject binding, and retained attestation-bundle SHA-256 where that contract is accepted;
- 03.24 aggregate matrix evidence, VM/image/seed identities and any accepted real-reboot persistence proof from infrastructure that preserves the same machine across the boot boundary;
- exact final source head and immutable tool/action versions used by the final gate.

Missing, stale, expired-without-governed-reproduction, differently hashed or unverifiable upstream evidence is a stop condition, not permission to substitute a newer convenient artifact.

## PKG-04 handoff boundary

03.25 may prepare a non-secret handoff describing the accepted Windows release boundary needed by the later Updater & Recovery package, including:
- final source SHA and package version/identity;
- exact signed package hashes and provenance/SBOM references;
- install roots and per-user/per-machine ownership boundaries;
- service identity/lifecycle and mutable-state/user-data boundaries;
- accepted rollback/repair/uninstall semantics;
- known no-restart/reboot behavior relevant to updater design;
- final evidence/run/artifact identities;
- explicit statement that PKG-04 is not activated until PKG-03 package completion projection is canonical.

It must not implement update feeds, differential updates, self-update, recovery orchestration or updater policy.

## Evidence expectation

The future final gate should produce one aggregate machine-readable evidence record bound to:
- exact source SHA;
- all final package subject filenames/roles/sizes/SHA-256 values and production signature verification identities;
- accepted 03.23 handoff/SBOM/provenance/attestation-bundle digests;
- accepted 03.24 matrix evidence digest plus VM/image/seed/reboot-proof identities where applicable;
- final regression matrix results;
- exact required governance/task-specific workflow run and job identities;
- final gate workflow/toolchain/action identities;
- zero unauthorized tracked drift;
- no leaked signing, OIDC or other reusable secret material;
- deterministic PKG-04 handoff digest.

Independent verification must reconstruct these bindings from machine-readable evidence rather than trusting human-readable summaries alone.

## Change-required decision

`change_required = false` for the frozen PKG-03 boundary at this checkpoint. Any final-gate failure must be classified and fixed in the smallest owning task boundary rather than weakening final acceptance.
