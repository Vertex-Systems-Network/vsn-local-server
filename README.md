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
- Current genuine PKG-02 progress: `25/27 = 92.59%`.
- `02.01` through `02.25` have genuine sequential acceptance evidence recorded in the canonical tracker.
- `02.23` Local `.test` DNS responder was integrated by signed/verified PR #94 merge commit `4e33fcd9244d07d7e5062a96d239e73d68b11b0e` after final-head acceptance and independent artifact verification.
- `02.24` Local domain/HTTPS privileged-boundary work was integrated by PR #96 merge commit `bd714fa946124f6c1eee31e557524bcb173230e1` after final-head GitHub-hosted Windows acceptance and independent artifact verification.
- `02.25` SQLite Database Studio acceptance passed on exact source `d1f5e2f38e9c20032cdfc4ccb0e53a71db46c4f6` in GitHub-hosted Windows run `32863289161`, job `97852297671`. Artifact `9569661040` digest `sha256:c40f83f4afc2cfa857b31de7319c57085857dfae90c118b07e885cad6a1449ba` and `evidence.json` digest `sha256:1c04b490f841090418327499a9efbfc49ef3850a5f3c1e749ef049d40d39e38c` were independently recomputed and matched; PR #98 merged as `84ae3e224e9c4ec7ae71eef692b8fa8159fe741a`.
- Active task: `02.26` — External/native database beta adapters.
- The live canonical `main` HEAD is intentionally **not hardcoded in this document**. Query GitHub at execution time; `docs/MASTER-EXECUTION-STATUS.json` and the active package tracker are the progress authority.
- `02.27` remains blocked by the frozen sequential task order.
- Old preparation PRs are historical input only and must not be treated as acceptance authority.
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

<!-- Canonical PKG-02 machine state: 25/27, 02.26 active; query live main SHA at execution time -->
