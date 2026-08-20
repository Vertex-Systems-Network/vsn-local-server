# VSN Local Server

VSN Local Server is a cross-platform local development/server platform intended to provide project, runtime, database, HTTPS/domain, desktop/CLI, updater, remote-management and release tooling from one system.

> Current state: active development. VSN 1.0 is **not** certified or stable yet.

## Canonical execution model

Development to 1.0 is governed by eight sequential packages:

1. **PKG-01 — Reproducible Build Foundation** — 22 subtasks
2. **PKG-02 — Usable Local Server Beta** — 27 subtasks
3. **PKG-03 — Windows Installer** — 25 subtasks
4. **PKG-04 — Updater & Recovery** — 18 subtasks
5. **PKG-05 — Linux + macOS Release** — 23 subtasks
6. **PKG-06 — Security Certification** — 20 subtasks
7. **PKG-07 — Production Resilience** — 22 subtasks
8. **PKG-08 — Pentest + Stable 1.0** — 25 subtasks

See `docs/MASTER-EXECUTION-PLAN.md` and `docs/MASTER-EXECUTION-STATUS.json` for the authoritative plan and machine-readable progress.

## Current package

**PKG-01 — Reproducible Build Foundation**

- `01.01` Rust 1.97.1 exact toolchain definition: **DONE**
- `01.02` candidate-bound Rust runtime verification: **IN PROGRESS**
- `01.03–01.22`: blocked by the sequential acceptance chain
- Root `Cargo.lock`: currently absent

A previous real Ubuntu GitHub Actions run proved Rust/Cargo 1.97.1 plus rustfmt/clippy on candidate `24ab1344…`. The currently declared main candidate is `c579788d…`, so strict candidate-bound policy requires a fresh 01.02 run before that task is counted DONE.

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

## Legacy certification tooling

Older `PKG-01 Linux Core` / P30 six-control scripts are retained only as legacy certification/governance tooling. Their status must **not** be confused with the current 22-task PKG-01 completion state.

## Toolchain pin

The current build foundation requires exact Rust **1.97.1** with `rustfmt` and `clippy`.
