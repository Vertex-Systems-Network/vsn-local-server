# READ THIS FIRST — VSN Canonical AI Project State & Handoff

> **Purpose:** This is the canonical AI continuation file for the VSN Local Server project. Every AI, developer, reviewer, or automation that continues this project must read this file first, verify the live GitHub state, and then continue from the current ACTIVE task only.
>
> **Critical rule:** Live GitHub state wins if this file is stale. When a mismatch is found, verify the live state, correct this file in the same working branch, and append a history entry before continuing.

## 1. Mandatory AI startup protocol

Every new AI session must do these steps in order:

1. Read this entire file.
2. Read `docs/MASTER-EXECUTION-PLAN.md`.
3. Read `docs/MASTER-EXECUTION-STATUS.json`.
4. Read `certification/pkg01-build-foundation-v1.json` while PKG-01 is active, or the active package tracker after PKG-01.
5. Read `docs/release-candidate-current.json` and record the current candidate ID.
6. Query GitHub for current `main`, open PRs, active branch head SHA, and latest workflow runs.
7. If a PR CI error references source lines that do not match the branch head, inspect the exact GitHub Actions checkout SHA / synthetic PR merge SHA before editing.
8. Continue only the current ACTIVE task. Do not skip blocked gates.
9. Do the maximum safe amount of real work in the current session. Do not create cosmetic status bumps or wrapper-only revisions when no acceptance gate changed.
10. After every meaningful implementation, blocker change, PR/head change, workflow change, candidate change, package/task status change, or architectural decision, update this file and append the activity log.

## 2. Source-of-truth precedence

Use this precedence when sources disagree:

1. **Live GitHub repository/PR/CI state** — highest authority.
2. **Current checked-out source at the exact CI checkout SHA**.
3. `VSN_AI_PROJECT_STATE.md` — canonical handoff/continuation ledger.
4. Package certification tracker, e.g. `certification/pkg01-build-foundation-v1.json`.
5. `docs/MASTER-EXECUTION-STATUS.json`.
6. `docs/MASTER-EXECUTION-PLAN.md`.
7. Historical PR comments, old CI runs, old package ZIPs, or chat history — lowest authority.

Never mark a task DONE because an old chat, old branch, or old candidate says it passed.

## 3. What VSN Local Server is

VSN Local Server is a cross-platform local development/server platform. It is intended to provide a unified local development environment and management layer covering runtimes, projects, databases, HTTPS/domains, desktop management, CLI/agent tooling, updater/recovery, remote management, packaging, certification, and production release workflows.

The goal is not a demo repository. The end state is an installable, reproducible, secure, resilient, cross-platform local-server product with evidence-backed builds and release gates.

## 4. Repository architecture map

Important source areas:

- `apps/agent` — background/local agent.
- `apps/cli` — command-line interface.
- `apps/desktop` — desktop application/UI.
- updater helper under `apps/` — update/recovery support.
- `crates/` — shared Rust workspace crates and platform services.
- `cloud/` — control-plane/dashboard-related source.
- `contracts/` — schemas/contracts.
- `packaging/` — release/install packaging.
- `fuzz/` — fuzz targets.
- `scripts/` — validation, release, governance, certification, and build helpers.
- `certification/` — package acceptance state/evidence definitions.
- `docs/` — roadmap, status, release identity, and operational documentation.
- `.github/workflows/` — CI and certification workflows.

Do not commit generated junk such as `target/`, `node_modules/`, build/dist outputs, Python caches, local toolchains/assets, archives, or transfer/import chunks unless an explicit release artifact policy requires them.

## 5. Branch and repository-management policy

- `main` is the canonical integration/stable source branch.
- `pkg01/*` through `pkg08/*` are package implementation/certification branches.
- `chore/*` is for hygiene/governance only.
- `import/*` is temporary provenance/import work only.
- Product changes should be done on a focused branch and reviewed through a PR.
- Do not merge a package-fix PR while its required acceptance gates are red.
- Keep unrelated dependency drift out of focused fixes.
- `Cargo.lock`, once committed as part of PKG-01, is immutable unless a deliberate dependency-update change is made and re-certified.

## 6. Master roadmap — 8 sequential packages

The project is divided into 8 sequential packages, 182 tracked tasks total:

| Package | Name | Tasks | Current status |
|---|---|---:|---|
| PKG-01 | Reproducible Build Foundation | 22 | IN PROGRESS |
| PKG-02 | Usable Local Server Beta | 27 | NOT STARTED |
| PKG-03 | Windows Installer | 25 | NOT STARTED |
| PKG-04 | Updater & Recovery | 18 | NOT STARTED |
| PKG-05 | Linux + macOS Release | 23 | NOT STARTED |
| PKG-06 | Security Certification | 20 | NOT STARTED |
| PKG-07 | Production Resilience | 22 | NOT STARTED |
| PKG-08 | Pentest + Stable 1.0 | 25 | NOT STARTED |

**Sequence rule:** do not start the next package until the previous package is genuinely DONE.

Current package completion:

```text
Packages complete: 0 / 8
PKG-01: 6 / 22 = 27.27%
P30 genuine PASS: 0 / 21
```

## 7. PKG-01 exact task state

| ID | Acceptance task | Status |
|---|---|---|
| 01.01 | Rust 1.97.1 exact toolchain definition | DONE |
| 01.02 | Real Rust runtime components verification | DONE |
| 01.03 | Resolve Cargo dependency graph | DONE |
| 01.04 | Generate/commit root `Cargo.lock` | DONE |
| 01.05 | `cargo fetch --locked` | DONE |
| 01.06 | `cargo fmt --all -- --check` | DONE |
| **01.07** | `cargo clippy --workspace --all-targets --locked -- -D warnings` | **IN PROGRESS — ACTIVE** |
| 01.08 | `cargo test --workspace --locked` | BLOCKED by 01.07 |
| 01.09 | Build `vsn-agent` release binary | BLOCKED |
| 01.10 | Build `vsn` CLI release binary | BLOCKED |
| 01.11 | Build updater-helper release binary | BLOCKED |
| 01.12 | Desktop npm dependency resolution | BLOCKED |
| 01.13 | Desktop `package-lock.json` | BLOCKED |
| 01.14 | Desktop `npm ci` | BLOCKED |
| 01.15 | Desktop production build | BLOCKED |
| 01.16 | Dashboard npm dependency resolution | BLOCKED |
| 01.17 | Dashboard `package-lock.json` | BLOCKED |
| 01.18 | Dashboard `npm ci` | BLOCKED |
| 01.19 | Dashboard production build | BLOCKED |
| 01.20 | Build artifact SHA manifest | BLOCKED |
| 01.21 | Fresh-checkout reproducibility | BLOCKED |
| 01.22 | PKG-01 final gate | BLOCKED |

Progress bar:

```text
PKG-01  ███░░░░░░░  6/22 = 27.27%
ACTIVE: 01.07 Clippy
```

## 8. Verified completed PKG-01 work

The following work has genuine evidence and is not merely planned:

- Exact Rust/Cargo 1.97.1 runtime was verified on a real Ubuntu x86_64 GitHub Actions runner.
- Cargo dependency graph was resolved; prior evidence recorded 36 workspace members and 743 packages/nodes.
- Root `Cargo.lock` was generated and committed.
- Locked dependency fetch passed.
- Workspace-wide Rust formatting was applied and `cargo fmt --all -- --check` passed.
- Syntax errors were fixed in `apps/agent/src/main.rs` and `crates/vsn-terminal/src/lib.rs`.
- Linux/Tauri CI prerequisites were added (`pkg-config`, GLib/GTK/WebKitGTK/AppIndicator/librsvg/OpenSSL/patchelf family).
- `crates/vsn-system/src/lib.rs`: Windows-only netstat parser was correctly cfg-gated and log-line iteration changed to `map_while(Result::ok)`.
- `crates/vsn-stream/src/lib.rs`: Clippy sort warning was converted to key-based sorting.
- `crates/vsn-database/src/lib.rs`: `CapabilitySet` uses derived `Default` instead of a redundant manual implementation.
- Prior SQL read-only helper Clippy warnings were addressed in an earlier merged PR.
- Build Foundation workflow was hardened so an existing committed `Cargo.lock` is verified with locked metadata instead of being regenerated. Lock generation is only for a missing lockfile.
- Repository governance CI exists to prevent generated/transfer junk and status drift.

## 9. Current open PR snapshot

**Only open PR at this snapshot:** PR #7

- Title: `PKG-01: clear terminal and native-database Clippy blockers`
- Base: `main`
- Base SHA before this handoff commit: `e0f7fbe8925347de4202ada9f04a9f3949227f65`
- Head branch: `pkg01/clippy-after-pr6`
- Head SHA before this handoff commit: `f77a901898591ad5511fdd8490d88a75b9675eca`
- State: OPEN, not draft
- Mergeability may appear available, but **do not merge while 01.07/01.08 are not both green**.

This handoff file itself is being committed to the same PR branch, so the PR head will move after the snapshot above. Future AI must query the newest head and newest workflow runs before using old run IDs.

PR #7 intended code scope before this documentation commit included:

- `Cargo.toml` — removes direct workspace `bson = "3"` dependency.
- `crates/vsn-database-native/Cargo.toml` — removes `bson.workspace = true`.
- `crates/vsn-database-native/src/lib.rs` — attempts to normalize Mongo BSON usage through `mongodb::bson` and addresses Postgres connection/TLS API.
- `crates/vsn-terminal/src/lib.rs` — intended descending sort uses `sort_by_key(Reverse(...))`.
- `certification/pkg01-build-foundation-v1.json` — current tracker/status.

## 10. Latest authoritative CI snapshot before this handoff commit

For pre-handoff PR #7 head `f77a901898591ad5511fdd8490d88a75b9675eca`:

- Build Foundation run: `32417415196` — FAILURE.
- Repository Governance run: `32417415438` — SUCCESS.
- Real Rust Runtime run: `32417415157` — SUCCESS.

Build Foundation run state:

- 01.02 Real Rust Runtime — PASS.
- 01.03 Cargo Dependency Graph — PASS.
- 01.05 Cargo Fetch Locked — PASS.
- 01.06 Cargo Format Check — PASS.
- Rustfmt fix artifact job — PASS.
- **01.07 Cargo Clippy — FAIL**, job `96581998141`.
- 01.08 Cargo Tests — SKIPPED because 01.07 failed.

Therefore **current valid completion remains 6/22**. Do not mark 01.07 or 01.08 DONE until a newer current-head run proves them green.

## 11. Critical CI source-revision rule

GitHub PR workflows may compile a synthetic merge commit rather than the visible branch head.

For the pre-handoff PR #7 failing Clippy run, the checkout SHA was:

`da79d48911bbadf59d0c46e65f3c13c10d4555b4`

which represented a synthetic merge equivalent to:

`Merge f77a901898591ad5511fdd8490d88a75b9675eca into e0f7fbe8925347de4202ada9f04a9f3949227f65`

**Never patch from stale line numbers.** If CI logs show code that does not match the PR branch file, inspect the exact CI checkout/synthetic merge SHA before changing source.

## 12. Current active blockers from PR #7 Clippy

The latest known 01.07 Clippy failure is in the native database/Mongo/Postgres path plus a terminal sort discrepancy.

### 12.1 Postgres TLS/API blocker

File: `crates/vsn-database-native/src/lib.rs`

Known error:

- unresolved import `postgres::SslMode`
- Rust error E0432: no `SslMode` in the `postgres` crate root.

**Required action:** inspect the actual `postgres` crate version/API used by this lockfile (known prior build used `postgres 0.19.x`) and implement the correct configuration/TLS connection API. Do not guess the enum path and do not suppress the compiler error.

### 12.2 BSON version/type split

Latest known CI compiled two BSON versions:

- `bson 2.15.0`, used by `mongodb 3.8.0`.
- `bson 3.1.0`, still present through another dependency path.

Known resulting mismatches include:

- Mongo `.find(filter)` expects `mongodb::bson::Document` / BSON 2.15 type, but receives BSON 3.x `Document`.
- Insert/update/delete helpers mix `Bson`/`Document` types from different major versions.
- `update_many` update/filter types do not match the Mongo driver’s expected types.

**Required action in order:**

1. Inspect reverse dependency graph equivalent to `cargo tree -i bson@3.1.0`.
2. Inspect `cargo tree -i bson@2.15.0`.
3. Identify the remaining BSON 3.x source/dependency.
4. Standardize `vsn-database-native` Mongo helper types on the Mongo driver’s BSON type, normally `mongodb::bson`, only after confirming the exact locked graph.
5. Do not add aliases or `#[allow]` attributes merely to hide the mismatch.

### 12.3 Terminal sort discrepancy

Clippy previously reported code equivalent to:

`out.sort_by(|a, b| b.started_at_unix_ms.cmp(&a.started_at_unix_ms));`

and recommends key-based descending sort with `std::cmp::Reverse`.

PR #7 patch claims the key-based fix already exists. Therefore this is a textbook case for the synthetic-merge rule: inspect the current CI checkout SHA/source before reapplying anything.

## 13. Exact next actions — do these in order

1. Query current PR #7 info after this handoff commit and record the newest head SHA.
2. Query workflow runs for the newest head. Do not rely on run `32417415196` if a newer run exists.
3. If the newest 01.07 run still fails, fetch its Clippy job logs and exact checkout SHA.
4. Inspect that exact revision’s `crates/vsn-database-native/src/lib.rs` and `crates/vsn-terminal/src/lib.rs`.
5. Resolve the Postgres `SslMode`/TLS API using the actual locked `postgres` crate API.
6. Inspect BSON reverse dependencies for both 2.15.x and 3.1.x.
7. Remove/normalize the remaining BSON 3.x path causing native database type mismatches.
8. Reconcile the terminal sort source against the exact CI checkout SHA; only patch if the current compiled source still contains the rejected `sort_by` form.
9. Run/require `cargo fmt --all -- --check` PASS.
10. Run/require `cargo clippy --workspace --all-targets --locked -- -D warnings` PASS.
11. **Only after full Clippy PASS**, mark 01.07 DONE and update progress to `7/22 = 31.82%`.
12. Then run `cargo test --workspace --locked`.
13. **Only after full tests PASS**, mark 01.08 DONE and update progress to `8/22 = 36.36%`.
14. Then continue sequentially to 01.09 Agent release binary, 01.10 CLI, 01.11 updater-helper, then desktop/dashboard build gates, artifact manifest, reproducibility, and final PKG-01 gate.

## 14. Why this order must not be changed

PKG-01 is establishing a reproducible foundation. Later binaries, desktop/dashboard builds, packaging, installers, security certification, and release evidence depend on the source compiling, linting, testing, and using a stable lockfile first.

Skipping Clippy/tests would allow downstream artifacts to be created from source that has not passed the agreed quality gates. That would make later package percentages meaningless and force repeated rebuilds.

## 15. Evidence and DONE rules

A task can be marked DONE only when its acceptance condition has genuine evidence.

Examples:

- CI workflow/job success on the current relevant source/candidate.
- Verified artifact + checksum where required.
- Source file committed where the task is a source-state requirement, e.g. `Cargo.lock`.
- Candidate-bound evidence when candidate binding is part of the contract.

Not valid as DONE evidence:

- a planned workflow that has not passed;
- a local synthetic regression standing in for a real runtime when real runtime is required;
- an older candidate’s success;
- a green earlier job followed by a newer head that has not been revalidated;
- chat statements or percentage estimates.

Never inflate progress. Never call a package complete until its final gate passes.

## 16. Cargo.lock policy

- Root `Cargo.lock` is a tracked release input after 01.04.
- Normal validation must use locked commands.
- Do not run a workflow that regenerates the lockfile on every validation run.
- If `Cargo.lock` exists, dependency graph validation should use locked metadata/fetch behavior.
- A dependency update must be intentional, isolated, reviewed, and then re-certified.
- If a focused source PR unexpectedly changes `Cargo.lock`, restore the canonical lock unless the dependency update is part of that PR’s explicit purpose.

## 17. Candidate/release identity policy

Current snapshot release identity comes from `docs/release-candidate-current.json`:

- Product version: `0.38.1`.
- Snapshot candidate ID recorded before this handoff: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`.

Future AI must fetch the live file because any source change may alter release identity/fingerprint. Never reuse candidate-bound evidence against a different candidate unless the evidence contract explicitly allows it.

## 18. Anti-mistake guardrails

- Do not start PKG-02 while PKG-01 is incomplete.
- Do not bypass Clippy with `#[allow(...)]` unless the lint is intentionally accepted for a documented architectural reason.
- Prefer fixing source semantics/architecture over hiding warnings.
- Do not delete a function as “dead code” until checking cfg-gated callers and other platforms.
- Do not trust PR head source when CI clearly compiled a synthetic merge revision; inspect the CI checkout SHA.
- Do not let focused PRs carry accidental dependency-lock drift.
- Do not create dozens of cosmetic package revisions that do not close a real acceptance task.
- Do not merge PR #7 merely because GitHub says it is mergeable.
- **Do not merge PR #7 until 01.07 Clippy AND 01.08 tests are both green on the current head/synthetic merge.**
- Do not mark 01.08 as active while 01.07 is failing.
- Do not rewrite repository history to clean old merge messages; maintain current state forward.
- Keep `main` clean and canonical.

## 19. Known historical decisions and lessons

- Initial sandbox could not download Rust because outbound DNS/TCP was blocked; real runtime verification moved to GitHub Actions.
- Runtime evidence was made candidate-bound and SHA-sealed to prevent false PASS imports.
- Earlier candidate evidence could not be reused after source/candidate changed; this is why current-head revalidation matters.
- Repository transfer/import chunks were removed after the full source became canonical in `main`.
- Generated caches/build outputs are intentionally excluded by `.gitignore` and governance CI.
- The original older “PKG-01 Linux Core 0/6” model was superseded by the current 22-task PKG-01 Reproducible Build Foundation model.
- CI native Linux dependencies were added because workspace Clippy reaches Tauri/GTK/WebKit-related crates.
- Build Foundation workflow must preserve the committed lockfile instead of silently regenerating it.

## 20. Required status shown to the user on every future `continue` / `next`

Every continuation response should include, at minimum:

- active package;
- complete PKG-01 22-task status or a compact table that still shows every task;
- exact `DONE / required` count;
- percentage and progress bar;
- current active task;
- exact blocker or latest evidence;
- master 8-package status;
- what changed in the current turn.

Do not show a higher percentage unless a task genuinely moved to DONE.

## 21. Session shutdown/update checklist

Before ending any substantial AI work session:

1. Re-fetch the active PR and branch head.
2. Re-fetch latest required CI status.
3. Update package tracker if a gate genuinely changed.
4. Update `docs/MASTER-EXECUTION-STATUS.json` if package progress changed.
5. Update this `VSN_AI_PROJECT_STATE.md` snapshot/current blockers/next action.
6. Append an entry to the activity log below; do not erase historical entries.
7. Make sure the next action is singular and executable, not vague.
8. If a PR is still open, state explicitly whether it is safe to merge.

## 22. Update triggers for this file

Update this file whenever any of these occur:

- open PR changes;
- active branch/head SHA changes materially;
- CI gate status changes;
- current blocker changes;
- package/task becomes DONE/ACTIVE/BLOCKED;
- candidate/release identity changes;
- workflow semantics change;
- `Cargo.lock` policy/dependency graph changes;
- significant architecture or repository-management decision is made;
- package transition occurs;
- release/packaging/certification milestone occurs.

## 23. Current exact continuation directive

**Read this file first, verify live GitHub state, and continue from ACTIVE task 01.07 only.**

At the time of this snapshot, the next useful engineering work is not more planning. It is to obtain the newest PR #7 Clippy failure on the newest head, inspect its exact checkout revision, resolve the native database Postgres/BSON type/API blockers and any real terminal-sort discrepancy, and rerun the full locked Clippy gate.

Only when 01.07 is green may 01.08 tests become active.

## 24. Append-only activity log

### 2026-08-21 — Asia/Karachi — Canonical AI handoff initialized

- Audited repository and open PRs after chat/context rollover.
- Confirmed only PR #7 was open before this documentation commit.
- Confirmed current valid PKG-01 progress remained 6/22 (27.27%).
- Confirmed pre-handoff PR #7 Build Foundation run `32417415196` failed at 01.07 Clippy and skipped 01.08 tests.
- Recorded pre-handoff PR #7 head `f77a901898591ad5511fdd8490d88a75b9675eca` and synthetic CI merge SHA `da79d48911bbadf59d0c46e65f3c13c10d4555b4`.
- Recorded current known Postgres `SslMode`, BSON 2.x/3.x type split, and terminal sort discrepancy blockers.
- Added this root canonical handoff file to the active PR branch so it can merge into `main` with the project state.
- **After this commit, CI must be re-queried using the new PR head; older run IDs are historical snapshots only.**

---

## One-line future-AI instruction

> **READ `VSN_AI_PROJECT_STATE.md` FIRST → VERIFY LIVE GITHUB STATE → UPDATE STALE SNAPSHOT IF NEEDED → WORK ONLY THE ACTIVE GATE → REQUIRE REAL EVIDENCE → UPDATE TRACKERS + THIS FILE → APPEND HISTORY → NEVER FAKE PROGRESS.**
