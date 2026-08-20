# Validation report — VSN 0.21.0

Date: 2026-08-19

## Source/offline gates passed

- JSON/config/provider parsing: 96 files
- JSON Schema/provider validation: 92 validations
- Node contract/provider parser: 96 checks
- Cargo manifests parsed: 38
- Product Cargo packages: 36 at `0.21.0` plus one cargo-fuzz harness
- Rust structural scans: 40
- Local Cargo path dependencies resolved: 74
- YAML: 5
- plist: 2
- WiX XML: 1
- Desktop TypeScript strict source check: PASS with temporary declarations for unavailable React/Tauri packages
- Dashboard TypeScript strict source check: PASS with temporary declarations for unavailable React packages
- Bash syntax: PASS
- Release gate: PASS
- Version synchronization: PASS
- private-key/AWS-key-like source scan: PASS
- symlink/generated-cache scan: PASS
- Linux `.deb` disposable package-layout and SHA smoke: PASS
- P0–P30 percentage arithmetic / source-closure assertions: PASS

## 0.21 source closures

P1, P2, P7, P11, P20, P22 and P24 are marked 100% because their defined source/product contracts have no known open source-scope gap after this batch. This does not substitute for P30 external certification.

P3 and P6 were deepened but remain below 100 because provider/platform-specific service/project extensibility edges remain.

## Native/external limitations

This artifact environment does not provide `cargo`, `rustc` or `pwsh`. Therefore Cargo format/clippy/test/build, native Tauri, Windows Agent/ConPTY/MSI acceptance and macOS package/notarization are **not** claimed as passed here. Those remain release-certification evidence under P30.

Node/npm/tsc and `dpkg-deb` are available. Full project npm dependencies were not installed in the source artifact; source-level strict TypeScript used temporary external-module declarations and no `node_modules` or `dist` directory is retained.
