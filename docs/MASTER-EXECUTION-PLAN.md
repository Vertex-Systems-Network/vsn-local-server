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
`02.01` through `02.20` are accepted on PR #88's branch with genuine sequential evidence. Final accepted behavioral/state-preprojection source `dfd20329434d7df59f571d3f3b858d9856733019` passed exact-head GitHub-hosted Windows 02.20 run `32714436581`, job `97392588770`. Artifact `9515572215` has digest `sha256:7b9f15c1401aa6613c02d62f60a9064e9e63511ef5b110131783c7488002049b`; independently verified `evidence.json` digest is `sha256:3d92e4a3d86138cc4a470b7da36b3e767b3553f42d5bc04574d14c11f44db072`. Acceptance proved ConPTY host-handshake handling, authenticated start/list/write/read-wait/resize/status/stop/remove, a 3053 ms bounded idle long-poll, bounded live truncation with 374116 dropped bytes, durable 1423091-byte scrollback with a 262144-byte bounded first read, recovery checkpoint replacement/update, workspace/direct-junction/program containment, fail-closed invalid resize, a valid 41-event audit chain and complete session/recovery/scrollback cleanup. Required same-head Governance, PKG-02 sequence, 02.02, 02.08, 02.16, 02.17, 02.18 and 02.19 regressions also passed. PR-branch projected PKG-02 progress is `20/27 = 74.07%`; projected next task is `02.21` Read-only local preview fetch. Canonical `main` remains `19/27 = 70.37%`, active `02.20`, at `45d04b051c92a65ef247bccddc1375d69df3a7c1` until PR #88 is final-head re-certified and explicitly authorized for merge. Do not implement or count `02.21` before integration and a fresh canonical re-read.
