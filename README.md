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

## Progress dashboard

Task-weighted project progress after accepted PKG-03 `03.18`: **67/182 = 36.81%** — `████░░░░░░`.

| Module / Package | Status | Tasks | Progress | Progress bar | Start date | End date |
| --- | --- | ---: | ---: | --- | --- | --- |
| **PKG-01 — Reproducible Build Foundation** | ✅ Complete | 22/22 | **100%** | `██████████` | 2026-08-20 | 2026-08-21 |
| **PKG-02 — Usable Local Server Beta** | ✅ Complete | 27/27 | **100%** | `██████████` | 2026-08-21 | 2026-08-26 |
| **PKG-03 — Windows Installer** | 🟡 In Progress | 18/25 | **72%** | `███████░░░` | 2026-08-26 | Ongoing |
| **PKG-04 — Updater & Recovery** | ⚪ Not Started | 0/18 | **0%** | `░░░░░░░░░░` | — | — |
| **PKG-05 — Linux + macOS Release** | ⚪ Not Started | 0/23 | **0%** | `░░░░░░░░░░` | — | — |
| **PKG-06 — Security Certification** | ⚪ Not Started | 0/20 | **0%** | `░░░░░░░░░░` | — | — |
| **PKG-07 — Production Resilience** | ⚪ Not Started | 0/22 | **0%** | `░░░░░░░░░░` | — | — |
| **PKG-08 — Pentest + Stable 1.0** | ⚪ Not Started | 0/25 | **0%** | `░░░░░░░░░░` | — | — |

### PKG-03 phase / wave status

| Phase | Tasks | Status | Done | Progress | Progress bar | Start date | End date |
| --- | --- | --- | ---: | ---: | --- | --- | --- |
| **Wave 0 — Architecture / Authority** | 03.01 | ✅ Complete | 1/1 | **100%** | `██████████` | 2026-08-26 | 2026-08-26 |
| **Wave 1 — Build / Identity / Scope / Ownership** | 03.02–03.05 | ✅ Complete | 4/4 | **100%** | `██████████` | 2026-08-26 | 2026-08-27 |
| **Wave 2 — Installer Lifecycle / Payload** | 03.06–03.10 | ✅ Complete | 5/5 | **100%** | `██████████` | 2026-08-27 | 2026-08-28 |
| **Wave 3 — Service / Security / Integrity / Diagnostics** | 03.11–03.15 | ✅ Complete | 5/5 | **100%** | `██████████` | 2026-08-29 | 2026-08-29 |
| **Wave 4 — Repair / Recovery / Runtime / Reboot** | 03.16–03.20 | 🟡 In Progress | 3/5 | **60%** | `██████░░░░` | 2026-09-01 | Ongoing |
| **Wave 5 — Automation / Signing / Provenance** | 03.21–03.23 | ⚪ Pending | 0/3 | **0%** | `░░░░░░░░░░` | — | — |
| **Wave 6 — Windows VM E2E** | 03.24 | 🔒 Blocked | 0/1 | **0%** | `░░░░░░░░░░` | — | — |
| **Wave 7 — Final PKG-03 Gate** | 03.25 | 🔒 Blocked | 0/1 | **0%** | `░░░░░░░░░░` | — | — |

> Progress accounting is evidence-driven. `READY` does not count as `DONE`; dates above are backed by accepted Git history rather than forecast deadlines, and future completion dates are intentionally not guessed.

## Current package

**PKG-03 — Windows Installer**

- PKG-01 is certified COMPLETE at `22/22 = 100%`.
- PKG-02 is certified COMPLETE at `27/27 = 100%`.
- PKG-03 has a frozen denominator of exactly 25 dependency-aware acceptance tasks (`03.01`–`03.25`).
- Current genuine PKG-03 progress: `18/25 = 72.00%`.
- `03.01`–`03.18` are canonically DONE with exact-head evidence recorded in `certification/pkg03-windows-installer-v1.json`.
- Deterministic resume cursor: `03.19`; dependency-ready tasks: `03.19`, `03.22`.
- `03.11` owns the VSN Agent Windows service install/start/health/removal lifecycle; `03.12` owns installer ACL/state/config separation; `03.13` owns firewall/hosts/resolver/trust-store non-mutation; `03.14` owns installed-payload integrity/repair detection; `03.15` owns logging, deterministic exit codes, cancellation and operator diagnostics.
- At most five dependency-ready PKG-03 implementation tasks may be active concurrently. No task may advance before all frozen dependencies are canonically DONE.
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

<!-- Canonical active-package machine state: PKG-03 18/25 IN_PROGRESS; READY 03.19,03.22; deterministic cursor 03.19; query live main SHA at execution time -->