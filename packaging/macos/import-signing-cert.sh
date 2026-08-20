#!/usr/bin/env bash
set -euo pipefail
: "${VSN_MACOS_P12_B64:?VSN_MACOS_P12_B64 is required}"
: "${VSN_MACOS_P12_PASSWORD:?VSN_MACOS_P12_PASSWORD is required}"
KEYCHAIN="${1:-$RUNNER_TEMP/vsn-signing.keychain-db}"; PASS="$(python3 - <<'PY'
import secrets;print(secrets.token_urlsafe(32))
PY
)"; P12="${RUNNER_TEMP:-/tmp}/vsn-signing.p12"
printf '%s' "$VSN_MACOS_P12_B64" | base64 --decode > "$P12"
security create-keychain -p "$PASS" "$KEYCHAIN"
security set-keychain-settings -lut 21600 "$KEYCHAIN"
security unlock-keychain -p "$PASS" "$KEYCHAIN"
security import "$P12" -k "$KEYCHAIN" -P "$VSN_MACOS_P12_PASSWORD" -T /usr/bin/productsign -T /usr/bin/security
security set-key-partition-list -S apple-tool:,apple: -s -k "$PASS" "$KEYCHAIN" >/dev/null
security list-keychains -d user -s "$KEYCHAIN" $(security list-keychains -d user | tr -d '"')
rm -f "$P12"
printf 'keychain=%s\n' "$KEYCHAIN"
printf 'keychain_password=%s\n' "$PASS"
