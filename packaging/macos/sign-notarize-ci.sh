#!/usr/bin/env bash
set -euo pipefail
PKG="$1"; IDENTITY="$2"; KEYCHAIN="$3"
: "${VSN_APPLE_API_KEY_ID:?VSN_APPLE_API_KEY_ID is required}"
: "${VSN_APPLE_API_ISSUER:?VSN_APPLE_API_ISSUER is required}"
: "${VSN_APPLE_API_KEY_P8_B64:?VSN_APPLE_API_KEY_P8_B64 is required}"
test -f "$PKG" || { echo "package missing: $PKG" >&2; exit 2; }
SIGNED="${PKG%.pkg}-signed.pkg"; KEYFILE="${RUNNER_TEMP:-/tmp}/AuthKey_${VSN_APPLE_API_KEY_ID}.p8"
printf '%s' "$VSN_APPLE_API_KEY_P8_B64" | base64 --decode > "$KEYFILE"
trap 'rm -f "$KEYFILE"' EXIT
productsign --keychain "$KEYCHAIN" --sign "$IDENTITY" "$PKG" "$SIGNED"
pkgutil --check-signature "$SIGNED"
xcrun notarytool submit "$SIGNED" --key "$KEYFILE" --key-id "$VSN_APPLE_API_KEY_ID" --issuer "$VSN_APPLE_API_ISSUER" --wait
xcrun stapler staple "$SIGNED"
xcrun stapler validate "$SIGNED"
printf 'signed_notarized_pkg=%s\n' "$SIGNED"
