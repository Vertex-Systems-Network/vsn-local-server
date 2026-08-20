#!/usr/bin/env bash
set -euo pipefail
VERSION="${1:-0.38.1}"; SOURCE_DIR="${2:-target/release}"; OUT="${3:-dist}"
for f in vsn-agent vsn vsn-updater-helper; do test -x "$SOURCE_DIR/$f" || { echo "missing $SOURCE_DIR/$f" >&2; exit 2; }; done
ROOT="$(mktemp -d)"; trap 'rm -rf "$ROOT"' EXIT
mkdir -p "$ROOT/DEBIAN" "$ROOT/usr/local/bin" "$ROOT/usr/lib/systemd/user" "$OUT"
cat > "$ROOT/DEBIAN/control" <<CTRL
Package: vsn-runtime
Version: $VERSION
Section: devel
Priority: optional
Architecture: amd64
Maintainer: VSN
Description: VSN local-first development runtime, CLI and secure Agent
CTRL
install -m 0755 "$SOURCE_DIR/vsn-agent" "$ROOT/usr/local/bin/vsn-agent"
install -m 0755 "$SOURCE_DIR/vsn" "$ROOT/usr/local/bin/vsn"
install -m 0755 "$SOURCE_DIR/vsn-updater-helper" "$ROOT/usr/local/bin/vsn-updater-helper"
install -m 0644 "$(dirname "$0")/vsn-agent.service" "$ROOT/usr/lib/systemd/user/vsn-agent.service"
dpkg-deb --build --root-owner-group "$ROOT" "$OUT/vsn-runtime-$VERSION-amd64.deb"
sha256sum "$OUT/vsn-runtime-$VERSION-amd64.deb" > "$OUT/vsn-runtime-$VERSION-amd64.deb.sha256"
