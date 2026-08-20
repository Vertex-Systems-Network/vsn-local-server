# Source closure — 0.26

**P0–P29 are source-closed. P30 is not source-closed by declaration because it is an evidence/certification phase.**

Newly closed in 0.26:

- **P23 Cloud Workspaces:** complete source lifecycle and provider-copy/clone semantics across AWS/Azure/GCP, including Azure cross-location artifact-copy paths and explicit status.
- **P25 Extensions:** complete signed/dependency/provider lifecycle plus fail-closed Linux, Windows and macOS executable sandbox backends.

`python scripts/source-readiness.py --run-gate` validates that every P0–P29 row is `done` at 100% and runs the source release gate.

Native execution/certification is intentionally not reclassified as source closure; it remains P30 evidence.

Batch 0.26 does not reopen source phases; it makes P30 certification evidence objective and executable.
