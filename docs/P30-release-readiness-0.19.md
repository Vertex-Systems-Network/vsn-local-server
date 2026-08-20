# P30 stable-release readiness — VSN 0.19

P30 roadmap maturity is **55%**, but roadmap percentage is not a certification signal.

Stable release requires the **21-control** evidence ledger enforced by `scripts/release-evidence.py`:

1. Rust build/test/clippy on Windows, Linux and macOS.
2. Desktop and dashboard production builds.
3. MSI, deb and pkg install/uninstall acceptance.
4. Updater apply/status/rollback E2E on all three OS families.
5. Windows Authenticode and macOS notarization evidence.
6. RustSec audit and both fuzz targets.
7. Control Plane load/SLO evidence.
8. HA failover and disaster-recovery restore drills.
9. Vault master-key rotation acceptance.
10. Penetration-test evidence.

`docs/release-evidence-0.19.json` begins with every item `pending`. `python scripts/release-evidence.py evaluate --file ... --require-certified` exits non-zero until every required item is `pass` or an explicitly reviewed `waived` control.

This makes the stable-release exit condition objective: source scaffolding, roadmap percentages, or a locally passing static validator cannot by themselves certify P30.
