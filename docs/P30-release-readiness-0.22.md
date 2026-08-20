# P30 stable-release readiness — VSN 0.22

Roadmap maturity and source closure do not certify Stable 1.0. P30 remains at **66%** until mandatory external/native evidence is genuinely satisfied.

0.22 moves service/project/control/IAM/audit/production-hardening source contracts to closed status and adds Control Plane DR automation, but it does not waive any release evidence requirement.

Required evidence still includes multi-OS Rust builds, Desktop/Dashboard production builds, MSI/deb/pkg install-uninstall acceptance, updater apply/rollback on all target OSes, Windows signing, macOS notarization, RustSec/fuzz gates, Control Plane SLO/load evidence, HA failover, DR restore rehearsal, Vault key rotation and independent penetration testing.

Use `scripts/release-evidence.py evaluate --file <ledger> --require-certified` for the objective certification gate and `scripts/source-readiness.py --run-gate` for the separate source-completeness gate.
