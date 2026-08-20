#!/usr/bin/env bash
set -euo pipefail
PKG="$1"; IDENTITY="$2"; PROFILE="${3:-}"
test -f "$PKG" || { echo "package not found: $PKG" >&2; exit 2; }
SIGNED="${PKG%.pkg}-signed.pkg"
productsign --sign "$IDENTITY" "$PKG" "$SIGNED"
pkgutil --check-signature "$SIGNED"
if [ -n "$PROFILE" ]; then
  xcrun notarytool submit "$SIGNED" --keychain-profile "$PROFILE" --wait
  xcrun stapler staple "$SIGNED"
fi
printf 'signed_pkg=%s\n' "$SIGNED"
