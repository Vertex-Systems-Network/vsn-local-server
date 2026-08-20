# VSN Local Server — Master Execution Plan

Canonical repository: `Vertex-Systems-Network/vsn-local-server`

## Completion rule
A package is COMPLETE only when every acceptance subtask is genuinely verified. Source/tooling readiness, a green helper test, or an evidence wrapper does not substitute for the package acceptance gates.

## Package sequence

| Package | Scope | Subtasks | State |
|---|---|---:|---|
| PKG-01 | Reproducible Build Foundation | 22 | IN PROGRESS |
| PKG-02 | Usable Local Server Beta | 27 | NOT STARTED |
| PKG-03 | Windows Installer | 25 | NOT STARTED |
| PKG-04 | Updater & Recovery | 18 | NOT STARTED |
| PKG-05 | Linux + macOS Release | 23 | NOT STARTED |
| PKG-06 | Security Certification | 20 | NOT STARTED |
| PKG-07 | Production Resilience | 22 | NOT STARTED |
| PKG-08 | Pentest + Stable 1.0 | 25 | NOT STARTED |

## PKG-01 — Reproducible Build Foundation

1. `01.01` Rust 1.97.1 exact toolchain definition.
2. `01.02` Verify rustc/cargo 1.97.1 plus rustfmt/clippy on the candidate-bound Linux runner.
3. `01.03` Resolve Cargo dependency graph.
4. `01.04` Generate and commit root `Cargo.lock`.
5. `01.05` `cargo fetch --locked`.
6. `01.06` `cargo fmt --all -- --check`.
7. `01.07` `cargo clippy --workspace --all-targets --locked -- -D warnings`.
8. `01.08` `cargo test --workspace --locked`.
9. `01.09` Build `vsn-agent` release binary.
10. `01.10` Build `vsn` CLI release binary.
11. `01.11` Build `vsn-updater-helper` release binary.
12. `01.12` Resolve Desktop npm dependency graph.
13. `01.13` Generate/commit Desktop `package-lock.json`.
14. `01.14` Desktop `npm ci`.
15. `01.15` Desktop production build.
16. `01.16` Resolve Dashboard npm dependency graph.
17. `01.17` Generate/commit Dashboard `package-lock.json`.
18. `01.18` Dashboard `npm ci`.
19. `01.19` Dashboard production build.
20. `01.20` Version/hash build artifact manifest.
21. `01.21` Fresh-checkout reproducibility test.
22. `01.22` PKG-01 final gate; all previous tasks must be DONE.

## Execution order
Packages and subtasks are sequential. A later task may be prepared, but it is not counted DONE before its prerequisites are complete. Bugs discovered while completing a package are fixed inside that package.

## Current blocker
The root `Cargo.lock` is absent. Before dependency resolution begins, `01.02` must be re-certified against the candidate currently declared by `docs/release-candidate-current.json`.
