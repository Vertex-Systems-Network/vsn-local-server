# READ THIS FIRST — VSN Canonical AI Project State & Handoff

> **Purpose:** This is the canonical AI continuation file for the VSN Local Server project. Every AI, developer, reviewer, or automation that continues this project must read this file first, verify the live GitHub state, and then continue from the current ACTIVE task only.
>
> **Critical rule:** **Live GitHub state wins if this file is stale.** When a mismatch is found, verify the live state, update this file in the active working branch, append a history entry, and only then continue engineering work.

## 1. Mandatory AI startup protocol

Every new AI session must do these steps in order:

1. Read this entire file.
2. Read `docs/MASTER-EXECUTION-PLAN.md`.
3. Read `docs/MASTER-EXECUTION-STATUS.json`.
4. Read `certification/pkg01-build-foundation-v1.json` while PKG-01 is active, or the active package tracker after PKG-01.
5. Read `docs/release-candidate-current.json` and record the current candidate ID.
6. Query GitHub for current `main`, open PRs, active branch head SHA, and latest workflow runs.
7. If CI source/line numbers do not match the visible branch file, inspect the exact GitHub Actions checkout SHA / synthetic PR merge SHA before editing.
8. Continue only the current ACTIVE task. Never skip blocked gates.
9. Do the maximum safe amount of real work in the current session. Do not create cosmetic status bumps or wrapper-only revisions when no acceptance gate changed.
10. After every meaningful implementation, blocker change, PR/head change, workflow change, candidate change, package/task status change, or architectural decision, update this file and append the activity log.

## 2. Source-of-truth precedence

Use this precedence when sources disagree:

1. **Live GitHub repository / PR / CI state** — highest authority.
2. **Exact source checked out by the CI job**.
3. `VSN_AI_PROJECT_STATE.md` — canonical continuation ledger.
4. Package certification tracker, e.g. `certification/pkg01-build-foundation-v1.json`.
5. `docs/MASTER-EXECUTION-STATUS.json`.
6. `docs/MASTER-EXECUTION-PLAN.md`.
7. Historical PR comments, old CI runs, old ZIPs/packages, or chat history — lowest authority.

Never mark a task DONE because an old chat, old branch, old candidate, or older PR run says it passed.

## 3. What we are building

**VSN Local Server** is a cross-platform local development/server platform. It is intended to provide a unified local development environment and management layer covering:

- runtimes and project lifecycle;
- databases;
- domains and local HTTPS;
- desktop application;
- CLI and agent;
- updater/recovery;
- remote-management/control-plane capabilities;
- packaging/installers;
- certification/evidence;
- security and resilience;
- stable cross-platform release.

The goal is not a demo repository. The end state is an **installable, reproducible, secure, resilient, cross-platform local-server product with evidence-backed builds and release gates**.

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
- `scripts/` — validation, release, governance, certification, build helpers.
- `certification/` — package acceptance state/evidence definitions.
- `docs/` — roadmap, status, release identity, operational documentation.
- `.github/workflows/` — CI and certification workflows.

Do not commit generated junk such as `target/`, `node_modules/`, build/dist outputs, Python caches, local toolchains/assets, archives, or transfer/import chunks unless an explicit release-artifact policy requires them.

## 5. Repository / branch policy

- `main` = canonical integration/stable source.
- `pkg01/*` through `pkg08/*` = package implementation/certification branches.
- `chore/*` = hygiene/governance.
- `import/*` = temporary provenance/import work only.
- Product changes should be done on a focused branch and reviewed through a PR.
- Do not merge a package-fix PR while required acceptance gates are red.
- Keep unrelated dependency drift out of focused fixes.
- `Cargo.lock`, once committed as part of PKG-01, is immutable unless a deliberate dependency-update change is made and re-certified.

## 6. Master roadmap — 8 sequential packages

The project is divided into **8 sequential packages / 182 tracked tasks**:

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

Current master progress:

```text
Packages complete: 0 / 8
PKG-01: 6 / 22 = 27.27%
P30 genuine PASS: 0 / 21
```

## 7. PKG-01 exact 22-task state

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

```text
PKG-01  ███░░░░░░░  6/22 = 27.27%
ACTIVE: 01.07 Clippy
```

## 8. Verified completed PKG-01 work

The following work has genuine evidence and is not merely planned:

- Exact Rust/Cargo 1.97.1 runtime verified on a real Ubuntu x86_64 GitHub Actions runner.
- Cargo dependency graph resolved; prior evidence recorded 36 workspace members and 743 packages/nodes.
- Root `Cargo.lock` generated and committed.
- `cargo fetch --locked` passed.
- Workspace formatting applied and `cargo fmt --all -- --check` passed.
- Syntax errors fixed in `apps/agent/src/main.rs` and `crates/vsn-terminal/src/lib.rs`.
- Linux/Tauri CI native prerequisites added (`pkg-config`, GLib/GTK/WebKitGTK/AppIndicator/librsvg/OpenSSL/patchelf family).
- `crates/vsn-system/src/lib.rs`: Windows-only parser correctly cfg-gated; line iteration changed to `map_while(Result::ok)`.
- `crates/vsn-stream/src/lib.rs`: Clippy sort warning converted to key-based sorting.
- `crates/vsn-database/src/lib.rs`: `CapabilitySet` uses derived `Default`.
- Earlier SQL read-only helper Clippy warnings were addressed in a merged PR.
- Build Foundation workflow hardened: an existing `Cargo.lock` is verified with locked metadata instead of being regenerated. Initial generation is only for a missing lockfile.
- Repository governance CI prevents generated/transfer junk and status drift.

## 9. Current open PR — live snapshot after chat rollover

**Only open PR:** **PR #7**

- URL: `https://github.com/Vertex-Systems-Network/vsn-local-server/pull/7`
- Current live title: `PKG-01: clear post-merge remote and IPC Clippy blockers`
- State: OPEN, not draft, not merged.
- Base: `main`
- Base SHA: `e0f7fbe8925347de4202ada9f04a9f3949227f65`
- Head branch: `pkg01/clippy-after-pr6`
- Product-code head before the handoff docs commits: `f77a901898591ad5511fdd8490d88a75b9675eca`
- First canonical-handoff commit head: `aabe4cb03cab2dc631deee4a29b6404726866f37`
- This reconciliation edit creates another documentation-only head; **always query the newest PR head before engineering**.

Current PR #7 changed source scope, excluding this handoff file:

1. `crates/vsn-ipc/src/lib.rs`
   - replaces `peer_addr()?.ip().is_loopback() == false` with direct negation `!…is_loopback()`.
2. `crates/vsn-remote/src/lib.rs`
   - boxes large enum variants (`AgentPollResponseV1`, `RemoteCommandV1`) and unboxes poll response at return.
3. `crates/vsn-security/src/lib.rs`
   - adds `DeviceIdentity::verify_with_public_key(...)` wrapper over the existing `verify_signature` implementation.
4. `crates/vsn-update/src/lib.rs`
   - replaces `value.len() < 1` with `value.is_empty()`.
5. `VSN_AI_PROJECT_STATE.md`
   - this canonical continuation ledger.

**Do not merge PR #7 until 01.07 Clippy AND 01.08 tests are both green on the current head/synthetic merge.**

## 10. Current CI state

### Historical pre-handoff engineering snapshot

For source head `f77a901898591ad5511fdd8490d88a75b9675eca`, the earlier Build Foundation run was not the final source of truth after the handoff commit. It is retained only as history.

### New head after the first handoff commit

For head `aabe4cb03cab2dc631deee4a29b6404726866f37`:

- Repository Governance run `32420490028` — **SUCCESS**.
- Real Rust Runtime run `32420490004` — **SUCCESS**.
- Build Foundation run `32420490138` — **IN PROGRESS** at the last verification before this reconciliation commit.

Build Foundation `32420490138` verified at that moment:

- 01.02 Real Rust Runtime — PASS.
- 01.03 Cargo Dependency Graph — PASS.
- 01.05 Cargo Fetch Locked — PASS.
- 01.06 Rustfmt Fix Artifact — generated/uploading successfully.
- 01.06 Cargo Format Check — in progress at the snapshot.
- 01.07 Clippy — not yet authoritative on that head at the snapshot.
- 01.08 tests — cannot be considered until 01.07 passes.

Because this reconciliation commit advances the PR head again, **future AI must fetch the newest run and must not treat run `32420490138` as authoritative if a newer run exists.**

## 11. Critical CI checkout / synthetic merge rule

GitHub PR workflows may compile a synthetic merge commit rather than the visible branch head.

If a CI error references source that does not match the branch file:

1. fetch the CI job logs;
2. record the checkout SHA;
3. inspect that exact commit/synthetic merge;
4. patch only after confirming the compiled source.

Never apply a fix from stale line numbers alone.

## 12. Current active blocker definition

**Current ACTIVE task is still 01.07 Clippy.**

At the moment this reconciliation file is written, the latest documentation-head Build Foundation run had not yet reached an authoritative completed Clippy verdict. Therefore the canonical blocker is:

> **Obtain the newest current-head 01.07 Clippy result. If it fails, use that exact job log and checkout SHA as the next blocker. Do not reuse superseded database/BSON/Postgres errors from an older branch snapshot unless they reappear in the newest run.**

The PR #7 source changes currently address concrete post-merge Clippy/compiler issues in remote/security/IPC/update code:

- missing public-key verification wrapper expected by remote code;
- non-idiomatic boolean equality in IPC;
- large enum variants in remote protocol messages;
- non-idiomatic empty-string length check in updater.

These are **attempted fixes**, not DONE evidence until the full current-head Clippy gate passes.

## 13. Exact next actions — execute in this order

1. Re-fetch PR #7 and record its newest head SHA.
2. Fetch workflow runs attached to that newest head.
3. Confirm Repository Governance and real Rust runtime remain green.
4. Follow Build Foundation sequentially through 01.03, 01.05, and 01.06.
5. When 01.07 completes:
   - if PASS: mark 01.07 DONE;
   - if FAIL: fetch exact Clippy job logs and checkout SHA, then fix only the newest real blocker.
6. Require `cargo fmt --all -- --check` PASS after any source fix.
7. Require `cargo clippy --workspace --all-targets --locked -- -D warnings` PASS.
8. **Only after 01.07 PASS**, update progress to `7/22 = 31.82%` and activate 01.08.
9. Run/require `cargo test --workspace --locked` PASS.
10. **Only after 01.08 PASS**, update progress to `8/22 = 36.36%`.
11. Then continue strictly:
    - 01.09 Agent release binary;
    - 01.10 CLI release binary;
    - 01.11 updater-helper release binary;
    - 01.12–01.19 desktop/dashboard dependency/build gates;
    - 01.20 artifact SHA manifest;
    - 01.21 fresh-checkout reproducibility;
    - 01.22 final PKG-01 gate.
12. Only after 01.22 DONE may PKG-02 start.

## 14. Why the sequence matters

PKG-01 establishes a reproducible foundation. Later binaries, desktop/dashboard builds, installers, security certification, and release evidence depend on source that compiles, formats, lints, tests, and uses a stable lockfile first.

Skipping Clippy/tests would create downstream artifacts from unverified source, invalidate package percentages, and force repeated rebuilds.

## 15. Evidence / DONE rules

A task is DONE only when its acceptance condition has genuine evidence, such as:

- current relevant CI workflow/job success;
- verified artifact + checksum where required;
- required source state committed (for example `Cargo.lock`);
- candidate-bound evidence where candidate binding is part of the contract.

Not valid as DONE evidence:

- a workflow that exists but has not passed;
- a synthetic/local regression standing in for required real execution;
- an older candidate’s success;
- an older PR head’s green job after the head changed;
- chat statements, plans, estimates, or cosmetic percentage changes.

Never inflate progress. Never call a package complete until its final gate passes.

## 16. Cargo.lock policy

- Root `Cargo.lock` is a tracked release input after 01.04.
- Normal validation uses locked commands.
- Do not regenerate it on every validation run.
- If `Cargo.lock` exists, dependency graph validation uses locked metadata/fetch behavior.
- Dependency updates must be intentional, isolated, reviewed, and re-certified.
- If a focused source PR unexpectedly changes `Cargo.lock`, restore the canonical lock unless dependency update is explicitly in scope.

## 17. Candidate / release identity policy

Current snapshot release identity comes from `docs/release-candidate-current.json`.

Last recorded candidate in the handoff audit:

- Product version: `0.38.1`.
- Candidate ID: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`.

Future AI must fetch the live file because source changes can alter identity/fingerprint. Never reuse candidate-bound evidence against a different candidate unless the evidence contract explicitly permits it.

## 18. Anti-mistake guardrails

- Do not start PKG-02 while PKG-01 is incomplete.
- Do not bypass Clippy with `#[allow(...)]` simply to get green.
- Prefer correcting source semantics/architecture over hiding warnings.
- Do not delete a function as “dead code” until checking cfg-gated callers/platforms.
- Do not trust stale PR source when CI compiled a synthetic merge revision.
- Do not let focused PRs carry accidental lockfile/dependency drift.
- Do not create cosmetic package revisions that do not close a real gate.
- Do not merge merely because GitHub reports mergeable=true.
- **Do not merge PR #7 until 01.07 Clippy AND 01.08 tests are both green on the current head.**
- Do not activate 01.08 while 01.07 is failing.
- Keep `main` clean/canonical and avoid history rewrites for old naming noise.

## 19. Known historical decisions / lessons

- Initial sandbox could not download Rust due blocked outbound DNS/TCP; real runtime verification moved to GitHub Actions.
- Runtime evidence was candidate-bound and SHA-sealed to prevent false PASS imports.
- Older candidate evidence was not reused after candidate/source changed.
- Repository transfer/import chunks were removed once full source became canonical in `main`.
- Generated caches/build outputs are excluded by `.gitignore` and governance CI.
- The older “PKG-01 Linux Core 0/6” scheme was superseded by the current 22-task Reproducible Build Foundation model.
- Linux native dependencies were added because workspace Clippy reaches Tauri/GTK/WebKit-related crates.
- Build Foundation must preserve the committed lockfile instead of silently regenerating it.
- CI source mismatch incidents established the rule to inspect exact checkout/synthetic merge SHAs before patching.

## 20. Required user-facing status on every future `continue` / `next`

Every continuation response should show at minimum:

- active package;
- PKG-01 22-task statuses (full or compact while preserving every task);
- exact `DONE / required` count;
- percentage + progress bar;
- active task;
- exact current blocker/evidence;
- master 8-package status;
- what changed in the current turn.

Do not show a higher percentage unless a task genuinely moved to DONE.

## 21. Session shutdown / handoff checklist

Before ending substantial work:

1. Re-fetch active PR and branch head.
2. Re-fetch latest required CI state.
3. Update package tracker if a gate genuinely changed.
4. Update `docs/MASTER-EXECUTION-STATUS.json` if package progress changed.
5. Update this file’s current PR/CI/blocker/next-action state.
6. Append an activity entry below; do not erase older entries.
7. Ensure the next action is singular and executable.
8. If a PR remains open, explicitly state whether it is safe to merge.

## 22. Mandatory update triggers for this file

Update this file whenever any of these occur:

- open PR changes;
- active branch/head changes materially;
- CI gate status changes;
- blocker changes;
- task becomes DONE/ACTIVE/BLOCKED;
- package transition occurs;
- candidate/release identity changes;
- workflow semantics change;
- Cargo.lock/dependency policy changes;
- major architecture/repository-management decision;
- release/packaging/certification milestone.

## 23. Current continuation directive

> **READ THIS FILE FIRST → VERIFY LIVE GITHUB STATE → CONTINUE ACTIVE TASK 01.07 ONLY.**

The next useful engineering action is not more roadmap planning. It is to inspect the newest PR #7 Build Foundation run on the newest head, obtain the current 01.07 result, and fix the exact current Clippy blocker if it is red.

Only when 01.07 is green may 01.08 tests become active.

## 24. Append-only activity log

### 2026-08-21 — Asia/Karachi — Canonical AI handoff initialized

- Audited repository/open PR state after chat/context rollover.
- Confirmed PKG-01 valid progress remained 6/22 (27.27%).
- Created root `VSN_AI_PROJECT_STATE.md` on active PR #7 branch so future AI sessions have one read-first continuation ledger.
- Defined live-GitHub precedence, mandatory startup protocol, evidence rules, Cargo.lock policy, candidate policy, branch discipline, update triggers, shutdown checklist, roadmap, exact PKG-01 state, and next-action sequence.
- Initial draft captured an older blocker snapshot; immediate live verification detected that PR #7 had evolved to remote/IPC/security/update fixes.

### 2026-08-21 — Asia/Karachi — Live-state reconciliation after handoff commit

- Re-fetched PR #7 after canonical file commit.
- Confirmed live PR title is `PKG-01: clear post-merge remote and IPC Clippy blockers`.
- Confirmed source diffs are currently `vsn-ipc`, `vsn-remote`, `vsn-security`, and `vsn-update` plus this handoff file.
- Confirmed first handoff head `aabe4cb03cab2dc631deee4a29b6404726866f37` had Governance PASS, real Rust PASS, and Build Foundation run `32420490138` in progress.
- Corrected this file immediately instead of leaving stale database/BSON/Postgres blockers as the active directive.
- Reaffirmed: **01.07 remains ACTIVE, 01.08 BLOCKED, progress remains 6/22 until current-head Clippy passes.**
- This reconciliation edit advances the PR head again; future AI must query the newest head/run after this commit.

---

## One-line future-AI instruction

> **READ `VSN_AI_PROJECT_STATE.md` FIRST → VERIFY LIVE GITHUB STATE → UPDATE STALE SNAPSHOT → WORK ONLY THE ACTIVE GATE → REQUIRE REAL EVIDENCE → UPDATE TRACKERS + THIS FILE → APPEND HISTORY → NEVER FAKE PROGRESS.**
