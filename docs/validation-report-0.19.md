# Validation report — VSN 0.19.0

Validated offline in the artifact workspace on 2026-08-19.

## Passed source/static checks

- batch validator: PASS
  - JSON/config/provider files parsed: 84
  - Cargo manifests parsed: 38
  - Rust source structural scans: 40
  - YAML files parsed: 5
  - plist files parsed: 2
  - local Cargo path dependencies resolved: 74
- JSON Schema/provider manifest validations: 80 PASS
- Node contract/provider parser: 84 PASS
- product Cargo packages: 36 at 0.19.0; one cargo-fuzz harness remains 0.0.0 by design
- Desktop strict TypeScript source check: PASS with temporary external React/Tauri declarations
- Dashboard strict TypeScript source check: PASS with temporary external React declarations
- Bash syntax: PASS
- WiX XML: PASS
- Python AST parse: PASS
- secret-like/private-key/AWS-key pattern scan: PASS
- generated-directory/symlink scan: PASS
- disposable Linux `.deb` package-layout smoke with fake executables: PASS
- release evidence evaluator: PASS, correctly reports 0/21 externally certified controls for the template
- roadmap percentage arithmetic: PASS, overall 85%

## Important native limitation

This environment still has no usable `cargo`, `rustc` or `pwsh`. Therefore Cargo fmt/clippy/test/build, native Tauri build, Windows service/MSI acceptance, macOS package/notarization, real Bubblewrap execution, privileged resolver mutation, real container-engine mutation and multi-OS release certification are **not** claimed as passed here.

The repository contains CI/smoke definitions for equipped runners. External certification evidence must be merged into the release evidence ledger before P30 can be certified.
