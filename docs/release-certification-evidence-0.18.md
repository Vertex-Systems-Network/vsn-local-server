# Stable-release certification evidence — VSN 0.18

`P30` cannot become certified from roadmap percentages alone. `scripts/release-evidence.py` maintains a bounded evidence ledger and fails `--require-certified` until every mandatory release control is `pass` or an explicitly reviewed `waived` item.

Mandatory evidence covers Rust builds on all three OS families, frontend builds, MSI/deb/pkg install-uninstall acceptance, updater apply/status/rollback on every OS, Windows signing, macOS notarization, RustSec, both fuzz targets, the Control Plane SLO load probe, HA failover, disaster-recovery restore, Vault master-key rotation and a penetration-test result.

Examples:

```bash
python scripts/release-evidence.py init --version 0.18.0 --output evidence.json
python scripts/release-evidence.py record --file evidence.json --id rust-linux --status pass --evidence ci://run/123
python scripts/release-evidence.py evaluate --file evidence.json --require-certified
```

The repository ships a 0.18 template with every item pending. It is intentionally **not** a fake green certification artifact.
