# VSN Dev Platform — Batch 0.31.0

## P30 Runner-Pack Execution

P0–P29 remain 100% source-closed. Batch 0.31 turns the 0.30 runner-pack model into executable one-command certification entrypoints without relaxing evidence requirements.

### Added

- `scripts/p30-pack-preflight.py`: exact host/tool/environment readiness for one pack.
- `scripts/p30-run-pack.py`: executes Linux, Windows, macOS, security, live-operations or independent-review controls and records only controls actually exercised successfully.
- `scripts/p30-bootstrap-plan.py`: deterministic prerequisite/bootstrap instructions pinned to the repository Rust toolchain.
- `.github/workflows/p30-run-pack.yml`: manual single-pack CI entrypoint for Linux, Windows, macOS and nightly security packs.
- `contracts/p30-pack-run-v1.schema.json` and `contracts/p30-bootstrap-plan-v1.schema.json`.
- Windows/macOS package builders now derive the product version from `VERSION` when no explicit version is supplied.

### Evidence behavior

A pack can produce mixed results. A missing signing credential may leave signing BLOCKED while Rust, updater and installer controls PASS. PASS evidence remains candidate-bound and provenance-bearing. No source/tooling improvement itself counts toward P30 certification.

### Current exact progress

- Source completion: **100.00%**
- Certification: **0/21**
- P30: **66.00%**
- Overall exact: **98.9032%**
- Stable 1.0: **not certified**

### Candidate binding

All PASS evidence carries `candidate_id`; evidence from different source candidates is rejected during collection/merge.
