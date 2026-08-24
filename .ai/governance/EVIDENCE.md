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

## Integrity

- Evidence is append-only/versioned after acceptance; do not rewrite history to make a later state look accepted.
- If an artifact declares a SHA-256 digest, independently recompute it before relying on it for a release/merge decision when the acceptance contract requires artifact integrity.
- Evidence references must identify immutable commits, run/job IDs or content digests. A mutable URL alone is insufficient.
- Logs are evidence inputs, not authority: redact secrets and reject truncated/missing critical proof.

## Scope

A green unrelated workflow is not acceptance evidence. Every plan acceptance criterion maps to a named test/check/evidence item. Required regression gates are explicit and cannot be substituted with convenient passing jobs.

## Cleanup and negative proof

Mutating/bootstrap/runtime tests must prove cleanup and bounded failure behavior where relevant. Negative cases include invalid paths, unsupported versions/profiles, denied permissions, unsafe network targets, oversized inputs/outputs and missing credentials without leaking secret values.

## State projection

Machine-readable completion state advances only after required evidence is accepted. Branch projections must remain distinguishable from canonical integrated state until merge. Dependent work starts only after canonical integration when the roadmap requires it.
