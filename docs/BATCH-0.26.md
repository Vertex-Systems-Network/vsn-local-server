# Batch 0.26 — P30 certification sprint

P0–P29 remain source-closed. This batch hardens the only remaining phase: P30 Stable Production Release with an evidence-driven certification model.

## Changes

- Evidence schema v2: blocked state, freshness, run URL, commit SHA, artifact hash and strict default no-waiver certification.
- Evidence-driven P30 calculation: `66 + 34 * valid_evidence_ratio`, capped below 100 until certified.
- Local certification driver that never converts missing prerequisites into PASS.
- Release workflow stale-ledger fix: current evidence file is evaluated, not a 0.20 historical file.
- Signing and nightly-security evidence include workflow/commit provenance.
- Protected reviewer evidence-intake workflow for external certification artifacts.

## Current result

The present Linux runner cannot supply Rust/native/external evidence, so the valid certification ledger remains 0/21. This batch improves the trustworthiness and executability of certification without fabricating Stable 1.0 evidence.
