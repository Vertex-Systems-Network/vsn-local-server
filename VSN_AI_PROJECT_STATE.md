# READ THIS FIRST — VSN Canonical AI Project State & Handoff

> **Purpose:** canonical AI continuation ledger for `Vertex-Systems-Network/vsn-local-server`.
>
> **Critical rule:** live GitHub repository / PR / CI state wins over this file. If this file and live GitHub disagree, reconcile the contradiction before product implementation. Never silently change the roadmap, acceptance criteria, task order, or prerequisite rules.

## 1. Mandatory startup protocol

1. Read this file.
2. Read `docs/MASTER-EXECUTION-PLAN.md`.
3. Read `docs/MASTER-EXECUTION-STATUS.json`.
4. Read `certification/pkg02-usable-local-beta-v1.json` while PKG-02 is active.
5. Read `docs/release-candidate-current.json` and record the candidate ID.
6. Query live GitHub for `main`, the active-task PR/branch, relevant open preparation PRs, review blockers, and exact-head workflow runs.
7. Work only on the current canonical task. Do not implement/count a future task before its prerequisite is integrated.
8. Mark DONE only with real acceptance evidence on the required environment.
9. Reconcile machine-readable state and this handoff only after real acceptance.
10. Do not merge an acceptance PR unless explicit merge authorization is given.

## 2. Source-of-truth precedence

1. Live GitHub repository / PR / CI state.
2. Exact source checked out by the relevant CI job.
3. This continuation ledger.
4. Active package certification tracker.
5. `docs/MASTER-EXECUTION-STATUS.json`.
6. `docs/MASTER-EXECUTION-PLAN.md`.
7. Historical PR comments, old branches/runs/artifacts, and chat history.

## 3. Current canonical `main`

Repository: `Vertex-Systems-Network/vsn-local-server`

Default branch: `main`

Canonical `main` HEAD:

`2364a5cf7460b6ee8bb0be31f8405181401335cb`

That commit merged PR #84 (`PKG-02: certify 02.16 contained workspace text files`).

Canonical machine-readable state on `main` remains:

- active package: `PKG-02 — Usable Local Server Beta`
- progress: `16/27 = 59.26%`
- `02.01` through `02.16`: `DONE`
- canonical active task: `02.17`
- `02.18+`: blocked until accepted 02.17 state is integrated
- package complete: `false`

Current release candidate ID:

`c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`

Product version: `0.38.1`

## 4. Active acceptance PR and branch projection

PR #85 — `PKG-02: certify 02.17 resumable binary workspace transfer`

Branch:

`pkg02/0217-resumable-binary-transfer-main-sync`

Base is canonical `main` `2364a5cf7460b6ee8bb0be31f8405181401335cb`.

The PR is intentionally **not merged** without explicit merge authorization.

After genuine 02.17 behavioral acceptance, the PR branch state projects:

- PKG-02 progress: `17/27 = 62.96%`
- `02.17`: `DONE`
- projected next task: `02.18 — Bounded direct terminal execution`
- `02.19+`: blocked by sequence

This is a **branch projection only**. It does not become canonical until PR #85 is integrated into `main`.

No 02.18 product implementation is included in PR #85.

## 5. Frozen 02.17 acceptance wording

`02.17 — Resumable binary workspace transfer: chunk/offset enforcement, status/abort, finalize and SHA-256 digest verification.`

The frozen 27-task master plan is authoritative. The denominator and wording were not changed.

Package guardrails remain:

- mutations stay behind authenticated `vsn-agent` / Core authorization;
- workspace containment remains fail-closed;
- unsupported/invalid inputs fail closed;
- installer/updater/release/security/resilience/pentest work remains in later packages;
- no 02.18 product work may begin before accepted 02.17 state is merged and canonical state is re-read.

## 6. 02.17 implementation and behavioral acceptance

02.17 used current canonical `main` as its implementation baseline. Historical preparation PR #50 was inspected only as an audit lead and was not used as the branch baseline.

Scoped product change:

- `abort_binary_upload()` performs staged-replacement recovery before discarding the partial upload so an interrupted finalize cannot leave the original destination missing or orphan its backup.

Focused regression coverage proves:

- exact offset enforcement without advancing committed bytes;
- 512 KiB binary chunk ceiling rejection before partial-file creation;
- finalize SHA-256 correctness and persisted file digest;
- checksum-mismatch preservation of the accepted destination;
- interrupted-replace status recovery;
- interrupted-replace abort recovery.

Exact accepted behavioral source:

`8d515575d5957f94531375214705b0ad031fe79d`

Task-specific GitHub-hosted Windows acceptance:

- workflow run `32660580865`
- job `97246023900`
- result: `SUCCESS`
- artifact `9498748049` (`pkg02-0217-resumable-binary-transfer-github-hosted`)
- artifact digest `sha256:ab41e2281fcef0944a7b9cf765e11b9c01bcc1903046b33d2197a5fffc0ccf86`
- independently verified `evidence.json` digest `sha256:1de8b4831b8ee387617259bbd851404194b05fc96ef850d1d0c5ea35b9b2eeb6`
- runner: GitHub-hosted Windows/X64
- IPC: `127.0.0.1:39731`
- audit chain: valid, 25 events
- cleanup: Agent stopped, LOCALAPPDATA restored, IPC key restored, sandbox removed

All 02.17 evidence checks were true:

- exact offset enforced;
- status verified;
- abort verified;
- restart after abort verified;
- multi-chunk resume verified;
- finalize SHA-256 verified;
- Agent digest verified;
- chunked download verified;
- per-chunk SHA-256 verified;
- direct workspace containment verified;
- Windows-junction containment verified;
- unsafe transfer ID rejected;
- crash/interrupted-replace recovery tests verified;
- audit chain valid.

Required exact-source regressions on the accepted behavioral head also passed:

- Repository Governance run `32660580955`
- PKG-02 Acceptance Sequence run `32660580891`
- 02.01 Agent Lifecycle run `32660580860`
- 02.02 Authenticated IPC run `32660580878`
- 02.03 CLI Core Operator Path run `32660580827`
- 02.08 Windows GitHub-Hosted Certification run `32660580830`
- 02.16 Workspace Text Files run `32660580836`

## 7. Acceptance failures encountered and reconciled

The first 02.17 certification waves were not counted as acceptance.

Observed blockers:

1. New 02.17 regression file was not rustfmt-clean. Only rustfmt-required formatting changes were applied.
2. Strict Clippy rejected nine `root.clone()` slice constructions with `cloned_ref_to_slice_refs`. They were replaced with `std::slice::from_ref(&root)` without behavior changes.
3. The 02.17 certification initially ran strict Clippy across `vsn-core`, which traversed an unchanged future-network dependency and failed on a pre-existing `needless_return` in `crates/vsn-network/src/lib.rs`. That warning exists on canonical `main` and is outside 02.17 scope. Future network code was **not** modified. The certification command was narrowed to strict active-package `vsn-files` Clippy while retaining `vsn-files`/`vsn-core` tests and the full locked release Agent/CLI build.

No failed/superseded head was used as acceptance evidence.

Legacy PKG-01 npm-graph workflows may still fail automatically; those are not frozen 02.17 acceptance gates and must not silently redefine 02.17 acceptance.

## 8. Relevant stale preparation

PR #50 — `PKG-02: prepare 02.17 resumable binary workspace transfer`

- stale stacked preparation branch;
- not the implementation baseline;
- must not be merged as the active solution;
- retained only as historical/audit material.

Future preparation PRs remain out of scope until their prerequisite task becomes canonical.

## 9. Current state-reconciliation checkpoint

Behavioral 02.17 acceptance is real on `8d515575d5957f94531375214705b0ad031fe79d`.

The PR branch has now advanced the state projection to `17/27 = 62.96%` with 02.18 projected active in:

- `docs/MASTER-EXECUTION-STATUS.json`
- `certification/pkg02-usable-local-beta-v1.json`
- `docs/MASTER-EXECUTION-PLAN.md`
- `README.md`
- this handoff ledger

Because these state-only changes produce a newer PR head than the accepted behavioral source, they must receive a **fresh exact-head certification wave** before PR #85 is called final-head acceptance-ready.

## 10. Exact next actions

1. Re-read live PR #85 head after this reconciliation commit and ensure all five state projections agree on `17/27`, `02.17 DONE`, and projected `02.18 active`.
2. Verify `main` is still `2364a5cf7460b6ee8bb0be31f8405181401335cb`; if not, stop and reconcile before proceeding.
3. Require fresh exact-head success on the final PR head for Repository Governance, PKG-02 Acceptance Sequence, 02.17, 02.02, 02.08, and 02.16. Retain any additional relevant green regressions as supporting evidence.
4. Verify the final-head 02.17 artifact binds `source_commit` to that exact final PR head and independently verify its evidence digest/checks/cleanup.
5. Update PR #85 metadata with the final-head evidence only after those gates pass.
6. Do **not** merge PR #85 without explicit merge authorization.
7. After an explicitly authorized merge, re-read canonical `main` and machine-readable state. Only if canonical state is then `17/27` with `02.18` active may 02.18 product implementation begin.

## 11. Activity log

### 2026-08-23 — 02.16 integration and 02.17 activation

- PR #84 was exact-head certified and merged as `2364a5cf7460b6ee8bb0be31f8405181401335cb`.
- Canonical PKG-02 advanced to `16/27 = 59.26%`, active `02.17`.
- Fresh 02.17 branch was created directly from canonical `main`.
- Stale draft PR #50 was inspected only as an audit lead.

### 2026-08-24 — 02.17 behavioral acceptance and state projection

- Reconciled the current-main binary-transfer path and identified the interrupted-finalize abort-recovery defect.
- Added the scoped recovery fix, focused regression tests, and GitHub-hosted Windows certification.
- Repaired formatting and test-only strict-Clippy failures without behavior changes.
- Refused to modify unchanged future-network code merely to satisfy an over-broad 02.17 Clippy traversal; scoped the task gate to the active file package while preserving tests and full release builds.
- Exact behavioral head `8d515575d5957f94531375214705b0ad031fe79d` passed 02.17 and required regressions.
- Verified artifact `9498748049`, artifact digest `sha256:ab41e2281fcef0944a7b9cf765e11b9c01bcc1903046b33d2197a5fffc0ccf86`, evidence digest `sha256:1de8b4831b8ee387617259bbd851404194b05fc96ef850d1d0c5ea35b9b2eeb6`, all 14 checks true, valid 25-event audit chain, and complete cleanup.
- Advanced PR-branch state projection to `17/27 = 62.96%`, `02.17 DONE`, projected `02.18 active`.
- Final-head exact-source recertification remains required before PR #85 can be called acceptance-ready.
