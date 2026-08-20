#!/usr/bin/env bash
set -euo pipefail
ROOT_REPO="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${1:-$(cat "$ROOT_REPO/VERSION")}"; SOURCE_DIR="${2:-target/release}"; OUT="${3:-dist}"
for f in vsn-agent vsn vsn-updater-helper; do test -x "$SOURCE_DIR/$f" || { echo "missing $SOURCE_DIR/$f" >&2; exit 2; }; done
ROOT="$(mktemp -d)"; trap 'rm -rf "$ROOT"' EXIT
mkdir -p "$ROOT/usr/local/bin" "$ROOT/Library/LaunchAgents" "$OUT"
install -m 0755 "$SOURCE_DIR/vsn-agent" "$ROOT/usr/local/bin/vsn-agent"
install -m 0755 "$SOURCE_DIR/vsn" "$ROOT/usr/local/bin/vsn"
install -m 0755 "$SOURCE_DIR/vsn-updater-helper" "$ROOT/usr/local/bin/vsn-updater-helper"
install -m 0644 "$(dirname "$0")/dev.vsn.agent.plist" "$ROOT/Library/LaunchAgents/dev.vsn.agent.plist"
pkgbuild --root "$ROOT" --identifier dev.vsn.runtime --version "$VERSION" "$OUT/vsn-runtime-$VERSION-unsigned.pkg"
shasum -a 256 "$OUT/vsn-runtime-$VERSION-unsigned.pkg" > "$OUT/vsn-runtime-$VERSION-unsigned.pkg.sha256"
