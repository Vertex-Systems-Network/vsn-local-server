# VSN Local Server

VSN Local Server is a cross-platform local development/server platform intended to provide project, runtime, database, HTTPS/domain, desktop/CLI, updater, remote-management and release tooling from one system.

> Current state: active development. VSN 1.0 is **not** certified or stable yet.

## Canonical execution model

Development to 1.0 is governed by eight sequential packages:

1. **PKG-01 — Reproducible Build Foundation** — 22 subtasks — **COMPLETE**
2. **PKG-02 — Usable Local Server Beta** — 27 subtasks — **IN PROGRESS**
3. **PKG-03 — Windows Installer** — 25 subtasks
4. **PKG-04 — Updater & Recovery** — 18 subtasks
5. **PKG-05 — Linux + macOS Release** — 23 subtasks
6. **PKG-06 — Security Certification** — 20 subtasks
7. **PKG-07 — Production Resilience** — 22 subtasks
8. **PKG-08 — Pentest + Stable 1.0** — 25 subtasks

See `docs/MASTER-EXECUTION-PLAN.md` and `docs/MASTER-EXECUTION-STATUS.json` for the authoritative plan and machine-readable progress.

## Current package

**PKG-02 — Usable Local Server Beta**

- PKG-01 is certified COMPLETE at `22/22 = 100%`.
- PKG-02 has a fixed, frozen denominator of 27 sequential acceptance tasks.
- Current genuine canonical PKG-02 progress: `22/27 = 81.48%`.
- `02.01` through `02.22` are accepted and integrated on canonical `main` with real sequential evidence.
- `02.22` Advanced loopback-only local preview requests was integrated by signed/verified PR #90 merge commit `d9c5aa245efb0d20957b4eb840e29a4f95a520d2` after exact-head GitHub-hosted Windows acceptance.
- PR #91 then integrated the audited AI-native planning/governance layer without changing PKG-02 machine state or implementing the next task.
- Active task: `02.23` — Local `.test` DNS responder lifecycle and protocol behavior: plan/start/status/stop, A/AAAA loopback answers and refusal of non-`.test` names.
- Canonical `main` is signed/verified merge commit `4c0a9ef5bf90d17f8d62a09bdf5ca78c58ae4738` at `22/27 = 81.48%`, active `02.23`.
- `02.24+` remain blocked by the frozen sequential task order.
- Stale stacked preparation PR #57 must not be merged as-is or treated as current 02.23 acceptance evidence.
- Installer/updater/release/security/resilience/pentest certification remain later packages and do not count toward PKG-02.

## Repository layout

- `apps/` — agent, CLI, desktop and updater helper
- `crates/` — shared Rust workspace crates
- `cloud/` — cloud/control-plane components
- `contracts/` — schemas/contracts
- `packaging/` — installer/package definitions
- `fuzz/` — fuzzing targets/corpora
- `scripts/` — validation, evidence and release tooling
- `certification/` — machine-verifiable certification definitions/evidence
- `docs/` — execution plan, architecture, release and governance documentation

## Repository hygiene

Generated dependencies/build output, temporary package-transfer chunks, local toolchains and archives must not be committed to `main`. See `docs/REPOSITORY-MANAGEMENT.md` and `.gitignore`.

## Architecture boundary

`vsn-agent` is the machine execution and mutation boundary. CLI/Desktop/Web clients are authenticated controllers and do not directly own runtime/database/process privileges. Unsupported providers/capabilities must fail closed rather than being guessed.

## Legacy certification tooling

Older `PKG-01 Linux Core` / P30 six-control scripts are retained only as legacy certification/governance tooling. Their status must **not** be confused with the current eight-package execution model.

## Toolchain pin

The certified build foundation uses exact Rust **1.97.1** with `rustfmt` and `clippy`; JavaScript build gates use the committed npm lockfiles and pinned Node/npm evidence declared by PKG-01.

<!-- Canonical PKG-02 state: 22/27, 02.23 active -->