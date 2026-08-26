# VSN Local Server

VSN Local Server is a cross-platform local development/server platform intended to provide project, runtime, database, HTTPS/domain, desktop/CLI, updater, remote-management and release tooling from one system.

> Current state: active development. VSN 1.0 is **not** certified or stable yet.

## Canonical execution model

Development to 1.0 is governed by eight sequential packages:

1. **PKG-01 — Reproducible Build Foundation** — 22 subtasks — **COMPLETE**
2. **PKG-02 — Usable Local Server Beta** — 27 subtasks — **COMPLETE**
3. **PKG-03 — Windows Installer** — 25 subtasks — **IN PROGRESS**
4. **PKG-04 — Updater & Recovery** — 18 subtasks
5. **PKG-05 — Linux + macOS Release** — 23 subtasks
6. **PKG-06 — Security Certification** — 20 subtasks
7. **PKG-07 — Production Resilience** — 22 subtasks
8. **PKG-08 — Pentest + Stable 1.0** — 25 subtasks

See `docs/MASTER-EXECUTION-PLAN.md` and `docs/MASTER-EXECUTION-STATUS.json` for the authoritative plan and machine-readable progress.

## Current package

**PKG-03 — Windows Installer**

- PKG-01 is certified COMPLETE at `22/22 = 100%`.
- PKG-02 is certified COMPLETE at `27/27 = 100%`.
- PKG-03 has a frozen denominator of exactly 25 dependency-aware acceptance tasks (`03.01`–`03.25`).
- Current genuine PKG-03 progress: `2/25 = 8.00%`.
- `03.01` — installer architecture/format/identity-source/ownership authority — is DONE with GitHub-hosted Windows evidence.
- `03.02` — deterministic GitHub-hosted Windows NSIS + MSI bundle build and artifact manifest — is DONE with exact-head evidence, strict zero tracked drift and independently verified installer hashes.
- Dependency-ready Wave 1 tasks are now `03.03`, `03.04`, and `03.05`; the deterministic resume cursor is `03.03`.
- At most five dependency-ready PKG-03 implementation tasks may be active concurrently. No task may advance before all frozen dependencies are canonically DONE.
- `03.03` owns publisher/upgrade metadata; `03.04` install-scope/elevation; `03.05` exact payload/resource ownership. `03.06` remains blocked until all of `03.02`–`03.05` are DONE.
- PKG-04 updater/recovery, PKG-05 Linux/macOS release, PKG-06 security certification, PKG-07 production resilience and PKG-08 pentest/stable-1.0 remain later packages and are not counted toward PKG-03.
- The live canonical `main` HEAD is intentionally **not hardcoded in this document**. Query GitHub at execution time; `docs/MASTER-EXECUTION-STATUS.json` and the active package tracker are the progress authority.
- Old preparation/superseded PRs are historical input only and must not be treated as acceptance authority.

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

<!-- Canonical PKG-03 machine state: 2/25 IN_PROGRESS; 03.03-03.05 READY; deterministic cursor 03.03; query live main SHA at execution time; CI refresh marker -->
