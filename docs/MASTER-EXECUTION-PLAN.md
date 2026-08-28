# VSN Local Server — Master Execution Plan

Canonical repository: `Vertex-Systems-Network/vsn-local-server`

## Completion rule
A package is COMPLETE only when every acceptance subtask is genuinely verified. Source/tooling readiness, a green helper test, or an evidence wrapper does not substitute for the package acceptance gates.

## Package sequence

| Package | Scope | Subtasks | State |
|---|---|---:|---|
| PKG-01 | Reproducible Build Foundation | 22 | COMPLETE |
| PKG-02 | Usable Local Server Beta | 27 | COMPLETE |
| PKG-03 | Windows Installer | 25 | IN PROGRESS |
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

## PKG-03 — Windows Installer

PKG-03 is governed by the frozen dependency-aware 25-task contract in `.ai/plans/pkg03-windows-installer-v1.md`, with machine state in `certification/pkg03-windows-installer-v1.json`. Task IDs and order are fixed at `03.01`–`03.25`; no dependent task may advance before all declared prerequisites are canonically DONE.

- Maximum concurrent dependency-ready implementation tasks: **5**.
- `active_task` is a deterministic resume cursor, not a claim that only one task may be READY.
- `03.01` freezes installer architecture, supported Windows package formats, identity source and ownership boundaries.
- Wave 1 tasks `03.02`–`03.05` cover deterministic build/artifacts, identity/upgrade metadata, install-scope/elevation and exact payload/resource ownership respectively.
- Updater/recovery remains PKG-04; Linux/macOS release remains PKG-05; deep security certification remains PKG-06; resilience remains PKG-07; pentest/stable-1.0 remains PKG-08.

## Execution order
Packages are activated sequentially, while dependency-ready subtasks inside the active package may execute in the frozen DAG up to that package's concurrency ceiling. A later package may be researched/prepared, but its product implementation is not counted DONE before its package prerequisite is COMPLETE. Bugs discovered while completing a package are fixed inside the minimum task scope that proves the defect.

## Current blocker

Canonical product acceptance is currently **PKG-03 — Windows Installer** at `10/25 = 40.00%`. Tasks `03.01`–`03.10` are canonically DONE with exact evidence recorded in `certification/pkg03-windows-installer-v1.json`.

The deterministic resume cursor is `03.11`. Dependency-ready tasks are `03.11`, `03.12`, `03.13`, `03.14`, and `03.15`; dependent Wave 4+ tasks remain blocked by the frozen DAG until their prerequisites are canonically DONE.

Product implementation is temporarily paused by the user-approved `ENG-GOV-V3` governance amendment. That governance pause does **not** change PKG-03 task status, denominator, dependency order or accepted evidence. Live WIP/pause details are checkpointed in `.ai/current-work.json`, which is non-authoritative and must be refreshed against live repository/CI state before mutation.

The live canonical `main` SHA is intentionally not hardcoded here. `docs/MASTER-EXECUTION-STATUS.json` plus the unique active tracker selected by `package_id == active_package` are the acceptance authority.

### Historical PKG-03 activation record — superseded live projection

The following paragraphs preserve the earlier activation snapshot as historical evidence. They are **not** the current progress/cursor authority.

PKG-02 is COMPLETE at `27/27 = 100%`. PKG-03 planning/freeze PR #106 was accepted and merged into canonical `main` as `4606579e07ae57785d1bc1dc12073ea1d036ab4d`, freezing exactly 25 Windows Installer tasks and the max-five DAG/resume contract.

`03.01 — Activate PKG-03 execution authority and freeze Windows installer architecture, format, identity and ownership contract` is DONE with genuine GitHub-hosted Windows architecture evidence on source `a988e2ea2786a6d5184946f2ef62a3674f9cddcb` from run `32965973057`, job `98168417117`, artifact `9605689209` (`sha256:834d4a949e35419c115923bff8df3c8c9f1aa340853445d0f69de7e94259600b`).

`03.02 — Deterministic GitHub-hosted Windows bundle build and artifact manifest` is accepted on exact source `b295d694277ae365de6c478a97148f918395469b` from run `32985006668`, job `98229676273`, artifact `9612956973` (`sha256:8861185d6ace102350583652de868d38d2247b82ecdb8680a25c961486fc8537`). Independently recomputed evidence.json is `sha256:8fa6411e7af14158ef5c14d0f8d94c3bb1c811c597552670b739a9f88682a689`; artifact manifest is `sha256:e25653ee66755891c5fc8c1ac99f916975fd1c81d616676a697400ce75e357c3`. The accepted bundles are NSIS `sha256:8cb36a8a0fdd1b11cd243c42f2fea44a1a8b4f1f587b3a872f73f79a0a7c2b96` and MSI `sha256:1b8e641bbcafff46b2f98171907f2e32df5cc851b45e466a0ee04b5d2d6cf414`, with strict zero tracked drift and no installer execution, privileged mutation or signing.

At that historical snapshot, PKG-03 was projected at `2/25 = 8.00%`; `03.03`, `03.04`, and `03.05` were dependency-ready and the deterministic cursor was `03.03`. That snapshot is superseded by the current projection above.

<!-- Canonical active-package machine state: PKG-03 10/25 IN_PROGRESS; READY 03.11,03.12,03.13,03.14,03.15; deterministic cursor 03.11; query live main SHA at execution time -->
