# Batch 0.30 — Incremental P30 Certification

VSN 0.30 keeps **P0–P29 source-closed at 100%** and changes P30 evidence execution from all-or-nothing CI aggregation to **incremental, candidate-bound evidence fragments**.

## Key changes

- Successful Rust/updater matrix entries emit per-OS P30 evidence fragments.
- Frontend, MSI, deb, pkg, RustSec, nightly fuzz, and signing jobs emit independent fragments.
- Evidence merge jobs run with `always()` and merge whatever successful fragments exist.
- One failed OS/job no longer discards valid PASS evidence from independent successful jobs.
- `certification/p30-runner-packs.json` maps all 21 controls into Linux, Windows, macOS, security, operations, and independent-review packs.
- `scripts/p30-runner-plan.py` reports current-host prerequisites and remaining controls.
- `scripts/p30-fragment.py` creates candidate-bound provenance-bearing PASS fragments.

P30 remains evidence-driven; CI orchestration improvements do not count as certification PASS by themselves.

## Compatibility notes

This continues the **Candidate-Bound P30 Certification** model: every ledger contains a `candidate_id`, and evidence from **different source candidates** is rejected. At 0/21 evidence the exact overall score remains **98.9032%**.
