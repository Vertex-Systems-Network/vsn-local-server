#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENGINE="${VSN_CONTAINER_ENGINE:-}"
if [[ -z "$ENGINE" ]]; then
  if command -v docker >/dev/null 2>&1; then ENGINE=docker
  elif command -v podman >/dev/null 2>&1; then ENGINE=podman
  else echo 'docker or podman is required' >&2; exit 2; fi
fi
CID="$(python3 "$ROOT/scripts/release-candidate.py" id --root "$ROOT")"
IMAGE="vsn-p30-linux:${CID:0:12}"
OUT="${VSN_P30_OUTPUT_DIR:-$ROOT/dist-p30-container}"
mkdir -p "$OUT"
"$ENGINE" build -f "$ROOT/certification/linux-core.Dockerfile" -t "$IMAGE" "$ROOT"
"$ENGINE" run --rm -v "$ROOT:/src:ro" -v "$OUT:/output" "$IMAGE" bash -lc '
  set -e
  cp -a /src /work
  cd /work
  npm install --ignore-scripts --prefix apps/desktop
  npm install --ignore-scripts --prefix cloud/dashboard
  python3 scripts/p30-run-pack.py --pack linux-core --output-dir /output --run-url local://p30-linux-container --commit-sha container
'
