# VSN Dev Platform

## Current milestone — 0.38.1 / PKG-01 Linux Core

P0–P29 are **100% source-closed**. P30 remains real-world certification.

Current exact status:

- Source: **100.00%**
- PKG-01 Linux Core: **0/6 genuine PASS on the current runner**
- Certification: **0/21**
- P30: **66.00%**
- Overall exact: **98.9032%**
- Stable 1.0: **not certified**

0.38 starts the first certification chunk. `PKG-01` contains one-command bootstrap, candidate-bound reproducibility locks, real Linux compiler/frontend/deb/updater/RustSec execution, result-bundle verification, evidence import, governance verification, release gate, and a finalizer that refuses to produce `PKG01-COMPLETE` until all six controls are valid PASS.

Run on an equipped Linux runner:

```bash
python scripts/pkg01-linux-core.py all --allow-network
python scripts/pkg01-finalize.py --output-dir dist-pkg01/final
```

Existing 0.37 evidence governance remains active: result bundles are candidate-bound and can be verified, quarantined, revoked, restored, superseded, aged, rebuilt, and checkpointed.

## PKG-01 toolchain security pin

Linux Core certification requires exact Rust **1.97.1**; Rust 1.97.0 is not accepted. Offline x86_64 bootstrap validates the pinned standalone archive SHA-256 before installation.
