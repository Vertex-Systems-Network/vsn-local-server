# Validation report — VSN 0.20.0

Date: 2026-08-19

## Passed in this artifact environment

- Batch validator: PASS
  - JSON/config/provider files parsed: 90
  - Cargo manifests parsed: 38
  - Rust source structural scans: 40
  - YAML files parsed: 5
  - plist files parsed: 2
  - local Cargo path dependencies resolved: 74
- JSON Schema/provider validation: 86 PASS
- Node contract/provider parser: 90 PASS
- Desktop strict TypeScript source check: PASS using temporary external React/Tauri declarations
- Dashboard strict TypeScript source check: PASS using temporary external React declarations
- Bash syntax: PASS
- Python syntax/AST checks: PASS
- release-gate source/security/version scan: PASS
- Linux `.deb` builder/layout smoke with disposable fake executables: PASS
- release-runner preflight: PASS as a reporting tool; this host is **not build-ready** because `cargo` and `rustc` are absent
- release-evidence evaluator: PASS; mandatory certification remains **0/21 satisfied** in the shipped template

## 0.20 security/correctness review

- Runtime audit is read-only and complements the existing repair flow rather than silently mutating installations.
- Advanced DB model analysis is sample-bounded and deterministic; it does not guess unknown database wire protocols.
- Container registry publish validates backend/source/target, uses direct argv and the existing Docker/Podman credential context, and accepts no registry password parameter.
- Marketplace publisher governance removes suspended/retired/disallowed-channel entries from search/update resolution while preserving legacy signed indexes without a publisher table.
- Candidate AI ToolPlan validation enforces version/tool-count/parameter budgets, safe identifiers, no recursive AI call, unrestricted-shell=false and mutation confirmation before execution.
- Team Vault is a separate shared trust domain: PostgreSQL stores only nonce+ciphertext metadata and a dedicated `VSN_CONTROL_VAULT_KEY_B64` is required. List/manage/reveal permissions are separate.
- Release preflight only reports tool availability; it cannot mark release evidence as passed.

## Native/compiler limitations

This environment does not provide `cargo`, `rustc` or `pwsh`. Therefore the following are **not** claimed as passing here:

- Cargo fmt/clippy/test/build
- native Tauri build
- Windows MSI/service acceptance
- Windows ConPTY E2E
- macOS package signing/notarization
- live Bubblewrap extension execution
- live Docker/Podman registry push
- multi-node PostgreSQL Team Vault E2E

Use the 0.20 Windows/Linux/macOS smoke suites and CI/release evidence workflows on equipped targets.
