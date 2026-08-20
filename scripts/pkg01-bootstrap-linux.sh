#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TOOLCHAIN="$(python3 - <<'PY'
from pathlib import Path
s=Path('rust-toolchain.toml').read_text();print(s.split('channel = "',1)[1].split('"',1)[0])
PY
)"
ALLOW_NETWORK="${VSN_PKG01_ALLOW_NETWORK:-0}"
RUST_ARCHIVE="${VSN_PKG01_RUST_ARCHIVE:-}"
RUST_ARCHIVE_SHA256="${VSN_PKG01_RUST_ARCHIVE_SHA256:-}"
OFFICIAL_RUST_TARGET="x86_64-unknown-linux-gnu"
OFFICIAL_RUST_URL="https://static.rust-lang.org/dist/2026-07-16/rust-1.97.1-x86_64-unknown-linux-gnu.tar.xz"
OFFICIAL_RUST_SHA256="88f28fa9af20594179f85d6df67078dfd6fa93e2f6da5e1e9b0ac4997988ca4f"
CARGO_AUDIT_BIN="${VSN_PKG01_CARGO_AUDIT_BIN:-}"

note(){ printf '[PKG-01] %s\n' "$*"; }
fail(){ printf '[PKG-01] ERROR: %s\n' "$*" >&2; exit 2; }

if ! command -v rustup >/dev/null 2>&1 && [[ -z "$RUST_ARCHIVE" && "$ALLOW_NETWORK" == "1" ]]; then
  note "Bootstrapping rustup from the official static.rust-lang.org installer"
  tmp_rustup="$(mktemp -d)"; trap 'rm -rf "$tmp_rustup"' EXIT
  arch="x86_64-unknown-linux-gnu"
  base="https://static.rust-lang.org/rustup/dist/${arch}/rustup-init"
  curl --proto '=https' --tlsv1.2 --fail --location --retry 3 --output "$tmp_rustup/rustup-init" "$base"
  curl --proto '=https' --tlsv1.2 --fail --location --retry 3 --output "$tmp_rustup/rustup-init.sha256" "$base.sha256"
  expected="$(awk '{print $1}' "$tmp_rustup/rustup-init.sha256" | head -1)"
  got="$(sha256sum "$tmp_rustup/rustup-init" | awk '{print $1}')"
  [[ -n "$expected" && "$got" == "$expected" ]] || fail "rustup-init SHA-256 verification failed"
  chmod +x "$tmp_rustup/rustup-init"
  "$tmp_rustup/rustup-init" -y --profile minimal --default-toolchain none
  export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
fi

if command -v rustup >/dev/null 2>&1; then
  note "Ensuring Rust ${TOOLCHAIN} via rustup"
  if ! rustup toolchain list | grep -q "^${TOOLCHAIN}"; then
    [[ "$ALLOW_NETWORK" == "1" ]] || fail "Rust ${TOOLCHAIN} missing and network bootstrap disabled"
    rustup toolchain install "$TOOLCHAIN" --profile minimal --component rustfmt --component clippy
  fi
  rustup override set "$TOOLCHAIN"
elif [[ -n "$RUST_ARCHIVE" ]]; then
  [[ -f "$RUST_ARCHIVE" ]] || fail "VSN_PKG01_RUST_ARCHIVE not found"
  [[ "$(uname -m)" == "x86_64" ]] || fail "Offline PKG-01 Rust archive policy currently supports x86_64 only"
  expected_sha="$OFFICIAL_RUST_SHA256"
  if [[ -n "$RUST_ARCHIVE_SHA256" && "$RUST_ARCHIVE_SHA256" != "$expected_sha" ]]; then
    fail "Provided Rust archive SHA-256 does not match the pinned official ${TOOLCHAIN} ${OFFICIAL_RUST_TARGET} digest"
  fi
  got="$(sha256sum "$RUST_ARCHIVE" | awk '{print $1}')"
  [[ "$got" == "$expected_sha" ]] || fail "Rust archive SHA-256 mismatch; expected pinned official ${TOOLCHAIN} digest"
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
  tar -xf "$RUST_ARCHIVE" -C "$tmp"
  installer="$(find "$tmp" -maxdepth 2 -type f -name install.sh | head -1)"
  [[ -n "$installer" ]] || fail "Rust standalone archive has no install.sh"
  prefix="${VSN_PKG01_RUST_PREFIX:-/opt/vsn-rust-${TOOLCHAIN}}"
  bash "$installer" --prefix="$prefix" --disable-ldconfig
  export PATH="$prefix/bin:$PATH"
else
  command -v rustc >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1 || fail "Need rustup, or set VSN_PKG01_RUST_ARCHIVE to ${OFFICIAL_RUST_URL}; its SHA-256 is pinned in this package"
fi

rustc_v="$(rustc --version | awk '{print $2}')"
cargo_v="$(cargo --version | awk '{print $2}')"
[[ "$rustc_v" == "$TOOLCHAIN" ]] || fail "rustc version $rustc_v != required $TOOLCHAIN"
[[ "$cargo_v" == "$TOOLCHAIN" ]] || fail "cargo version $cargo_v != required $TOOLCHAIN"

if [[ ! -f Cargo.lock ]]; then
  [[ "$ALLOW_NETWORK" == "1" ]] || fail "Cargo.lock missing and network bootstrap disabled"
  note "Generating candidate-bound Cargo.lock"
  cargo generate-lockfile
fi

if ! command -v cargo-audit >/dev/null 2>&1; then
  if [[ -n "$CARGO_AUDIT_BIN" ]]; then
    [[ -x "$CARGO_AUDIT_BIN" ]] || fail "VSN_PKG01_CARGO_AUDIT_BIN is not executable"
    install -m 0755 "$CARGO_AUDIT_BIN" /usr/local/bin/cargo-audit
  else
    [[ "$ALLOW_NETWORK" == "1" ]] || fail "cargo-audit missing and network bootstrap disabled"
    cargo install cargo-audit --locked
  fi
fi

for app in apps/desktop cloud/dashboard; do
  if [[ ! -f "$app/package-lock.json" ]]; then
    [[ "$ALLOW_NETWORK" == "1" ]] || fail "$app/package-lock.json missing and network bootstrap disabled"
    note "Generating candidate-bound lockfile for $app"
    (cd "$app" && npm install --package-lock-only --ignore-scripts --no-audit --no-fund)
  fi
  note "Installing locked dependencies for $app"
  (cd "$app" && npm ci --no-audit --no-fund)
done

note "PKG-01 bootstrap complete"
