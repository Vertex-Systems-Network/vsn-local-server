# P30 Stable 1.0 release readiness — 0.20

P30 is not certified by roadmap percentage. Stable release requires the mandatory 21-control machine-readable release-evidence controls to be genuinely satisfied on external runners/environments.

## Fast runner triage

Before a release runner is used, execute:

```bash
python scripts/release-preflight.py --json
python scripts/release-preflight.py --strict
```

Preflight checks host-relevant Rust/Node/package/signing tools and optional container/sandbox/security backends. It reports missing capability only; **it never updates certification evidence**.

## Evidence

`docs/release-evidence-0.20.json` retains the mandatory certification ledger. Run:

```bash
python scripts/release-evidence.py evaluate --file docs/release-evidence-0.20.json --require-certified
```

Stable release remains blocked until all mandatory build, installer, updater, signing/notarization, RustSec/fuzz/load, HA/DR, Vault rotation and penetration-test controls are supported by real evidence.

0.20 adds more implementation maturity but does not waive any external P30 control.
