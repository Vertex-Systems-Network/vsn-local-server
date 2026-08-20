# P30 Stable Release Certification

P30 is evidence-driven. Source closure alone cannot produce Stable 1.0.

## Formula

Until certified: `P30 = 66 + round(34 * valid_controls / 21)` and maximum is 99. When every required control is current and valid, P30 becomes 100.

## Evidence safety

Evidence v2 records product version, status, platform, timestamp/freshness, run URL, commit SHA and optional artifact SHA-256. `blocked` is distinct from `fail` and `pending`. A waiver does not certify Stable 1.0 unless the evaluator is explicitly invoked with `--allow-waivers`.

## Commands

```bash
python scripts/certify-local.py
python scripts/release-evidence.py evaluate --file docs/release-evidence-current.json
python scripts/p30-progress.py --evidence docs/release-evidence-current.json --write
```

External evidence can be accepted only through the protected `production-certification` workflow or merged from successful CI/security/signing evidence artifacts.
