# AI Acceptance Evidence Contract

Acceptance evidence must prove the approved criterion against an exact source state. It must be reviewable without trusting an AI summary.

## Required binding

For every acceptance run record, where applicable:

- feature/plan ID and version;
- exact source commit SHA being certified;
- base/canonical commit used for the run;
- workflow run ID and job ID, or deterministic local command transcript;
- runner OS/architecture and relevant toolchain versions;
- exact acceptance commands and exit status;
- required positive and negative checks;
- cleanup result;
- artifact ID/name plus cryptographic digest when an artifact is produced.

A pull-request synthetic merge SHA must not silently stand in for the source head. If CI executes a merge ref, the evidence must separately bind the intended PR head and verify that binding before certification.

## FAST GATE and FULL GATE evidence

For new v2/work-package work, evidence identifies whether a check is a `FAST GATE` or `FULL GATE`.

A FAST GATE is targeted feedback for a mutation slice. Its evidence may prove the touched surface but cannot be cited as final acceptance when the contract requires a FULL GATE.

A FULL GATE is pre-merge/final-acceptance evidence and must include every regression/integration/E2E/negative/governance/platform check required by the approved contract. Missing required FULL GATE evidence means the item is not COMPLETE.

## Baseline failure evidence

`BASELINE_FAILURE` is valid only when the same relevant failure is independently reproduced on the exact canonical base, with base SHA, command/environment and failure result recorded. A candidate failure must not be relabeled baseline merely because it resembles an old issue or passes on retry.

Where useful, include candidate-vs-base delta evidence so attribution is reviewable. A proven baseline failure remains subject to the active acceptance contract; proof of pre-existence is not automatic permission to merge.

## Flaky-test evidence

Use `FLAKY_SUSPECTED` until nondeterminism is demonstrated and `FLAKY_CONFIRMED` only with reproducible evidence across equivalent attempts/environments.

A retry pass does not erase a failure and is not acceptance by itself. Any quarantine record must include owner, reason, bounded test/scope, creation evidence and expiry/revisit condition. Disabling, deleting or weakening a test only to obtain green status is prohibited.

## Review provenance

Material review records use exactly one approved provenance label:

- `SELF_REVIEW`
- `INDEPENDENT_AI_REVIEW`
- `HUMAN_REVIEW`
- `REQUIRED_EXTERNAL_REVIEW`

Record reviewer reference, reviewed scope, outcome and decision reference where applicable. `SELF_REVIEW` cannot satisfy an independent review requirement. `REQUIRED_EXTERNAL_REVIEW` records a requirement and remains pending until the named external/human authority supplies evidence.

Automated checks are evidence, not reviewer provenance. Record `AUTOMATED_STATIC` and `AUTOMATED_RUNTIME` separately as automation evidence; never represent them as human or independent AI review.

## Completion evidence

For new v2/work-package work, `COMPLETE` requires evidence for every applicable universal DoD criterion defined by the active contract: approved implementation/behavior, acceptance/tests, security/error handling, data integrity/migration, performance where applicable, integration, documentation/checkpoint, VCS/history, known limitations/not-verified items, and rollback/recovery/cleanup.

`PARTIALLY_COMPLETE` evidence must enumerate completed criteria and their proof, outstanding criteria, blockers/deferred items and owners. It must explicitly state that the work is not COMPLETE/DONE. Historical accepted v1 work is not retroactively downgraded because it predates this vocabulary.

## Integrity

- Evidence is append-only/versioned after acceptance; do not rewrite history to make a later state look accepted.
- If an artifact declares a SHA-256 digest, independently recompute it before relying on it for a release/merge decision when the acceptance contract requires artifact integrity.
- Evidence references must identify immutable commits, run/job IDs or content digests. A mutable URL alone is insufficient.
- Logs are evidence inputs, not authority: redact secrets and reject truncated/missing critical proof.

## Scope

A green unrelated workflow is not acceptance evidence. Every plan acceptance criterion maps to a named test/check/evidence item. Required regression gates are explicit and cannot be substituted with convenient passing jobs.

For v2/work-package work, the evidence set should also prove that actual changed paths/shared surfaces remained within the declared expected-change contract and scope budget or record the approved reassessment/change decision.

## Cleanup and negative proof

Mutating/bootstrap/runtime tests must prove cleanup and bounded failure behavior where relevant. Negative cases include invalid paths, unsupported versions/profiles, denied permissions, unsafe network targets, oversized inputs/outputs, missing credentials, declared abuse cases and forbidden-boundary attempts without leaking secret values.

## State projection

Machine-readable completion state advances only after required evidence is accepted. Branch projections must remain distinguishable from canonical integrated state until merge. Dependent work starts only after canonical integration when the roadmap requires it.
