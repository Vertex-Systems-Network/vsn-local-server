#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
test "$(cat VERSION)" = "0.33.0"
python3 scripts/validate-batch-0.33.py
python3 scripts/test-p30-pack.py
python3 scripts/source-readiness.py
python3 scripts/release-candidate.py verify --root . --file docs/release-candidate-current.json
python3 scripts/p30-pack-preflight.py --pack macos-core >/tmp/vsn-p30-macos-preflight.json || test $? -eq 3
