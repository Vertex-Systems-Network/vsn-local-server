#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
command -v cargo >/dev/null || { echo 'cargo is required' >&2; exit 2; }
command -v python3 >/dev/null || { echo 'python3 is required' >&2; exit 2; }
python3 scripts/validate-schemas.py
python3 scripts/validate-batch-0.17.py
python3 scripts/release-gate.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
python3 scripts/smoke-updater-helper.py --helper target/release/vsn-updater-helper
rm -rf /tmp/vsn017-linux-dist; mkdir -p /tmp/vsn017-linux-dist
./packaging/linux/build-deb.sh 0.17.0 target/release /tmp/vsn017-linux-dist
dpkg-deb -I /tmp/vsn017-linux-dist/vsn-runtime-0.17.0-amd64.deb >/dev/null
dpkg-deb -c /tmp/vsn017-linux-dist/vsn-runtime-0.17.0-amd64.deb | grep -q './usr/local/bin/vsn-agent'
if [[ "${VSN_INSTALL_PACKAGE_SMOKE:-0}" == "1" ]]; then
  sudo dpkg -i /tmp/vsn017-linux-dist/vsn-runtime-0.17.0-amd64.deb
  /usr/local/bin/vsn --version
  sudo dpkg -r vsn-runtime
fi
echo 'VSN batch 0.17 Linux smoke PASS'
