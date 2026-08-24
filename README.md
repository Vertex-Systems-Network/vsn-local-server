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
- PR #88 branch projection after genuine 02.20 acceptance: `20/27 = 74.07%`.
- `02.01` Local Agent lifecycle through `02.20` Interactive PTY/ConPTY session lifecycle are accepted on this PR branch with real sequential evidence. 02.20 proves authenticated PTY start/write/read-wait/resize/status/stop/remove, ConPTY terminal-host handshake handling, bounded live output, durable bounded scrollback, recovery checkpoint replacement/update, workspace/program containment, fail-closed invalid resize, audit validity and complete session/recovery/scrollback cleanup.
- Projected next task: `02.21` — Read-only local preview fetch. **No 02.21 product implementation is included in this branch.**
- Canonical `main` remains `19/27 = 70.37%`, active `02.20`, until PR #88 is final-head re-certified, explicitly authorized and merged.
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

<!-- PKG-02 02.20 accepted-state projection; final-head recertification required before integration -->
