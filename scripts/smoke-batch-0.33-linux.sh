#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
test "$(cat VERSION)" = "0.33.0"
python scripts/validate-schemas.py
python scripts/validate-batch-0.33.py
python scripts/test-release-evidence.py
python scripts/test-release-candidate.py
python scripts/test-p30-scoreboard.py
python scripts/test-p30-fragments.py
python scripts/test-p30-pack.py
python scripts/source-readiness.py
python scripts/release-gate.py
python scripts/release-candidate.py verify --root . --file docs/release-candidate-current.json
python scripts/p30-pack-preflight.py --pack linux-core >/tmp/vsn-p30-linux-preflight.json || test $? -eq 3
