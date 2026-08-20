# PKG-01 — Linux Core

Completion rule: **6/6 valid PASS or package remains open**.

Run on an equipped Linux x86_64 runner:

```bash
python scripts/pkg01-linux-core.py status
python scripts/pkg01-linux-core.py prepare --allow-network
python scripts/pkg01-linux-core.py execute
python scripts/pkg01-finalize.py --output-dir dist-pkg01/final
```

Offline Rust bootstrap is supported by setting `VSN_PKG01_RUST_ARCHIVE` to the official standalone Rust archive and optionally `VSN_PKG01_RUST_ARCHIVE_SHA256`. An offline `cargo-audit` binary can be supplied through `VSN_PKG01_CARGO_AUDIT_BIN`.

`prepare` will not generate lockfiles unless network bootstrap is explicitly enabled. `execute` will not certify Rust without `Cargo.lock`, and will not certify either frontend without a committed/generated `package-lock.json` and installed locked dependencies.

## Rust 1.97.1 certification pin

PKG-01 no longer accepts Rust 1.97.0. The certification toolchain is pinned to **Rust 1.97.1**. The x86_64 Linux standalone distribution is bound to the dated official Rust distribution URL and SHA-256 recorded in `certification/pkg01-linux-core-v1.json`; offline bootstrap rejects any archive whose digest differs.

This is a certification-policy correction, not a certification PASS. PKG-01 remains incomplete until all six Linux controls produce genuine valid PASS evidence.
