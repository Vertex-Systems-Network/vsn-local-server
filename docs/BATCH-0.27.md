# VSN 0.27 — P30 Evidence Aggregation Sprint

This is the P30 certification sprint for evidence aggregation and provenance hardening.

P0–P29 remain source-closed. Batch 0.27 hardens Stable 1.0 certification rather than adding product scope.

## Delivered

- Evidence v2 PASS records now require usable provenance: either an artifact SHA-256 or a workflow run URL plus evidence label.
- Evidence merge semantics are timestamp-aware: the newest record wins; exact-time ambiguity fails closed with failure/blocked states outranking PASS.
- `scripts/test-release-evidence.py` regression-tests rerun recovery, failure precedence, anonymous PASS rejection, and cross-version merge rejection.
- `scripts/p30-collect.py` discovers multiple evidence artifact ledgers, enforces one product version, merges them, validates freshness/provenance, evaluates certification, and recalculates P30.
- `.github/workflows/p30-aggregate.yml` can download evidence artifacts from explicit release, nightly-security, signing, and reviewer-approved workflow run IDs and produce a merged certification ledger.
- P30 remains evidence-driven: the 66% source-ready floor does not increase without valid certification PASS controls.

## Current certification boundary

This container still cannot provide native Rust/Windows/macOS/HA/DR/penetration evidence. Missing prerequisites are recorded as blocked/pending and never promoted to PASS.
