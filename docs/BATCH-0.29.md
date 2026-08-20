# Batch 0.29 — Candidate-Bound P30 Certification

VSN 0.29 keeps P0–P29 source-closed and hardens P30 evidence so certification from different source candidates cannot be mixed.

## Release candidate identity

`release-candidate.py` fingerprints release inputs (code, contracts, workflows, packaging, providers, native helpers and root build/toolchain files). Generated evidence/status documents are excluded so evidence updates do not mutate the candidate.

Every evidence v3 ledger contains the exact `candidate_id`. Merge fails closed when product versions match but candidate IDs differ.

## Reproducible CI

Rust is pinned to 1.97.0 through `rust-toolchain.toml`; release workflows use the candidate fingerprint and read the product version from `VERSION` instead of repeating package-version literals.

## Certification status

Bundled authoritative evidence remains 0/21. P30 therefore remains 66.00% and exact overall remains 98.9032%. No certification percentage is awarded for automation alone.
