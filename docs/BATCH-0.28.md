# Batch 0.29 — Exact P30 Certification Scoreboard

0.28 does not inflate Stable 1.0 readiness with additional source work. P0–P29 remain source-closed. The batch fixes the misleading whole-number progress display and turns P30 into an exact, per-control scoreboard.

## Exact progress model

- Source completion: 100.00%
- P30 source-ready floor: 66.00%
- Certification evidence: 0/21 = 0.00%
- P30 exact completion: 66.00%
- Overall exact completion: 98.9032%
- Rounded headline: 99%

A valid PASS contributes 34/21 = 1.6190 P30 points and 34/(21×31) = 0.0522 overall percentage-points. Stable 1.0 is certified only at 21/21 valid controls, regardless of rounded headline.

## New tooling

- `scripts/p30-scoreboard.py`: exact scoreboard, current-runner overlay, milestone simulation.
- `scripts/p30-fastest-path.py`: shortest grouped execution sequence and predicted exact score after each runner stage.
- `contracts/p30-scoreboard-v1.schema.json`: machine-readable scoreboard contract.
- Roadmap status now carries exact overall/P30/certification values in addition to the compatibility integer headline.

## Current runner

Local certification detects 13 blocked Linux/cross-platform controls and 8 Windows/macOS-specific pending controls. No blocked or pending control is counted as PASS.
