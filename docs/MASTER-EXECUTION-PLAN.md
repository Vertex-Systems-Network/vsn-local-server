# VSN Local Server — Master Execution Plan

Canonical repository: `Vertex-Systems-Network/vsn-local-server`

## Completion rule
A package is COMPLETE only when every acceptance subtask is genuinely verified. Source/tooling readiness, a green helper test, or an evidence wrapper does not substitute for the package acceptance gates.

## Package sequence

| Package | Scope | Subtasks | State |
|---|---|---:|---|
| PKG-01 | Reproducible Build Foundation | 22 | COMPLETE |
| PKG-02 | Usable Local Server Beta | 27 | IN PROGRESS |
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

## PKG-02 — Usable Local Server Beta

PKG-02 is the first end-user local-server acceptance package. The execution boundary remains `vsn-agent`; CLI and Desktop are authenticated controllers. Installer/updater packaging, remote production operations, security certification and production-resilience certification remain in later packages and are not counted here.

1. `02.01` Local Agent startup, machine identity, health/status and clean shutdown acceptance.
2. `02.02` Authenticated local IPC envelope, replay-window, nonce, frame-bound and response-binding acceptance.
3. `02.03` CLI core operator path: status, machine, security, diagnostics, config and audit verification.
4. `02.04` Desktop authenticated Agent bridge, online/offline posture and Overview refresh/error-state acceptance.
5. `02.05` Workspace roots: add/list/remove persistence, canonical paths and workspace-containment enforcement.
6. `02.06` Project detection and dependency analysis for registered workspace projects.
7. `02.07` Project template catalog and deterministic bootstrap-plan acceptance.
8. `02.08` Project bootstrap execution inside an allowed workspace with bounded/idempotent failure behavior.
9. `02.09` Runtime inventory, registry and audit acceptance across provider-reported runtimes.
10. `02.10` Trusted runtime catalog verification, signature/trust failure handling and archive path-safety acceptance.
11. `02.11` Runtime install plus per-project runtime activation acceptance using a verified catalog artifact.
12. `02.12` Runtime uninstall and repair/audit recovery acceptance without damaging unrelated runtimes.
13. `02.13` VSN-managed OS service lifecycle: status/start/stop/restart with namespace and permission boundaries.
14. `02.14` Local diagnostics: process snapshot/metrics, port list/check, TCP health check and bounded log tail.
15. `02.15` Docker/Podman local container baseline: backend discovery plus bounded read/lifecycle operations with unavailable-daemon handling.
16. `02.16` Workspace text-file operations: list/read/write/mkdir/move/delete with root-protection and path-containment checks.
17. `02.17` Resumable binary workspace transfer: chunk/offset enforcement, status/abort, finalize and SHA-256 digest verification.
18. `02.18` Bounded direct terminal execution inside an allowed workspace, including timeout/output limits and invalid-command handling.
19. `02.19` Persistent pipe terminal sessions: start/write/read-wait/status/stop/list/remove with bounded output.
20. `02.20` Interactive PTY/ConPTY session lifecycle: start/write/read-wait/resize/status/stop/remove plus bounded scrollback/recovery behavior.
21. `02.21` Read-only local preview fetch against loopback development servers with bounded response handling.
22. `02.22` Advanced local preview requests: allowed HTTP methods, bounded request/response bodies and filtered headers; loopback-only mutation boundary.
23. `02.23` Local `.test` DNS responder lifecycle and protocol behavior: plan/start/status/stop, A/AAAA loopback answers and refusal of non-`.test` names.
24. `02.24` Local domain/HTTPS planning and privileged network boundary: domain plan, hosts apply/remove/reload behavior and fail-closed elevation requirements.
25. `02.25` SQLite Database Studio end-to-end: inspect, browse, safe query, indexes/relations/stats and structured insert/update/delete.
26. `02.26` External/native database beta adapters: client detection plus PostgreSQL/MySQL/MariaDB/MongoDB/Redis declared-capability handling, with loopback/TLS and unsupported-capability fail-closed rules.
27. `02.27` Fresh-state local beta final gate: CLI + Desktop end-to-end smoke over all accepted local capabilities, zero unintended file/lock drift, and evidence that tasks `02.01`–`02.26` are DONE.

## PKG-02 scope guardrails

- The beta must prove usable local workflows, not merely schema/conformance existence.
- Every mutating operation remains behind Agent authentication, authorization and workspace/VSN-managed-resource boundaries.
- Unknown runtimes/databases/providers must fail closed rather than being guessed.
- Windows installer/signing is PKG-03; updater apply/rollback is PKG-04; Linux/macOS release packaging is PKG-05; deep security certification is PKG-06; production resilience is PKG-07; pentest/stable-1.0 certification is PKG-08.
- Remote Control Plane production acceptance is not required to count PKG-02 local beta tasks.

## Execution order
Packages and subtasks are sequential. A later task may be prepared, but it is not counted DONE before its prerequisites are complete. Bugs discovered while completing a package are fixed inside that package.

## Current blocker

The live canonical `main` HEAD is deliberately not embedded here because a documentation merge would immediately invalidate such a value. Query GitHub at execution time and bind any implementation/certification branch to the observed live SHA. The canonical machine state is PKG-02 `26/27 = 96.30%`: `02.01` through `02.26` are accepted and integrated, and the active task is `02.27 — Fresh-state local beta final gate: CLI + Desktop end-to-end smoke over all accepted local capabilities, zero unintended file/lock drift, and evidence that tasks 02.01–02.26 are DONE.`

`02.26` was integrated by PR #100 merge commit `d30baa7045c6a9b3649ef79f592ca905e40fb615` after final exact-head GitHub-hosted Windows acceptance and independent artifact verification. Final source `dfdba4427d38fe5433ec409d2ce52e569a1af7f2` passed run `32907910932`, job `97995887246`; artifact `9585772443` digest `sha256:f6a9e15e73e44c48e2ab05ef6d8a8457d4254527cf3559ffdd36f0a4edc3d71f` and independently verified `evidence.json` digest `sha256:f44d64e60b8577ea55222712ba80b1a46241e8bb86ced7fb5947b4a8530b31f3` matched. The frozen 17-gate same-head regression set also passed.

Before any `02.27` product mutation or certification claim, re-read live canonical state, reconcile stale preparation PR #61 against current `main`, instantiate/freeze the 02.27 AI lifecycle and exact acceptance evidence contract, and pass planning governance. Historical preparation branches may inform research but are not implementation baselines or acceptance authority.
