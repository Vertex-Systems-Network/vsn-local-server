#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
[[ "$(uname -s)" == "Darwin" ]] || { echo 'macOS is required' >&2; exit 2; }
command -v cargo >/dev/null || { echo 'cargo is required' >&2; exit 2; }
python3 scripts/validate-schemas.py
python3 scripts/validate-batch-0.26.py
./target/release/vsn commands >/tmp/vsn-commands.json
./target/release/vsn diagnostics >/tmp/vsn-diagnostics.json
python3 scripts/release-gate.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
python3 scripts/smoke-updater-helper.py --helper target/release/vsn-updater-helper
rm -rf /tmp/vsn025-macos-dist; mkdir -p /tmp/vsn025-macos-dist
./packaging/macos/build-pkg.sh 0.26.0 target/release /tmp/vsn025-macos-dist
pkgutil --check-signature /tmp/vsn025-macos-dist/vsn-runtime-0.26.0-unsigned.pkg >/dev/null 2>&1 || true
if [[ "${VSN_INSTALL_PACKAGE_SMOKE:-0}" == "1" ]]; then
  sudo installer -pkg /tmp/vsn025-macos-dist/vsn-runtime-0.26.0-unsigned.pkg -target /
  /usr/local/bin/vsn --version
  sudo rm -f /usr/local/bin/vsn /usr/local/bin/vsn-agent /usr/local/bin/vsn-updater-helper /Library/LaunchAgents/dev.vsn.agent.plist
  sudo pkgutil --forget dev.vsn.runtime || true
fi
echo 'VSN batch 0.26 macOS smoke PASS'
