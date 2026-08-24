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
- Current PR-branch projected PKG-02 progress: `22/27 = 81.48%`.
- `02.01` through `02.21` are accepted and integrated on canonical `main` with real sequential evidence.
- `02.22` Advanced loopback-only local preview requests is genuinely accepted on PR #90 with exact-head GitHub-hosted Windows evidence and is projected DONE on this branch.
- Projected active task: `02.23` — Local `.test` DNS responder lifecycle and protocol behavior: plan/start/status/stop, A/AAAA loopback answers and refusal of non-`.test` names.
- Canonical `main` remains signed/verified merge commit `98702614949afda4f5aa08ad032f01fd6613b3ef` at `21/27 = 77.78%`, active `02.22`, until PR #90 is merged.
- `02.24+` remain blocked by the frozen sequential task order; **no 02.23 product implementation is included in PR #90**.
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

<!-- PKG-02 02.22 accepted on PR #90 branch; 02.23 projected active task -->
