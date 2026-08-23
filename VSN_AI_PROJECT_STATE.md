# READ THIS FIRST — VSN Canonical AI Project State & Handoff

> **Purpose:** canonical AI continuation ledger for `Vertex-Systems-Network/vsn-local-server`.
>
> **Critical rule:** live GitHub repository / PR / CI state wins over this file. If this file and live GitHub disagree, reconcile it on the active working branch before product implementation. Never change the roadmap or acceptance criteria silently.

## 1. Mandatory startup protocol

1. Read this file.
2. Read `docs/MASTER-EXECUTION-PLAN.md`.
3. Read `docs/MASTER-EXECUTION-STATUS.json`.
4. Read `certification/pkg02-usable-local-beta-v1.json` while PKG-02 is active.
5. Read `docs/release-candidate-current.json` and record the candidate ID.
6. Query live GitHub for `main`, active-task PRs/branches, review blockers, and exact-head workflow runs.
7. If CI source differs from the visible branch, inspect the exact checkout/synthetic merge SHA before editing.
8. Work only on the current active task. Do not count future-task preparation.
9. Mark DONE only with real acceptance evidence on the required environment.
10. After a meaningful task/PR/head/candidate/blocker/state change, reconcile this ledger and machine-readable state as appropriate.

## 2. Source-of-truth precedence

1. Live GitHub repository / PR / CI state.
2. Exact source checked out by the relevant CI job.
3. This continuation ledger.
4. Active package certification tracker.
5. `docs/MASTER-EXECUTION-STATUS.json`.
6. `docs/MASTER-EXECUTION-PLAN.md`.
7. Historical PR comments, old branches/runs/artifacts, and chat history.

## 3. Current canonical repository state

Repository: `Vertex-Systems-Network/vsn-local-server`

Default branch: `main`

Canonical `main` HEAD:

`2364a5cf7460b6ee8bb0be31f8405181401335cb`

That commit merged PR #84 (`PKG-02: certify 02.16 contained workspace text files`). The merge is signed/verified and has parents `abbd6e8a7f59cafaf8695f4455738b90f41a81f0` and accepted PR head `791fb1b2e940e696ecd5bd2d17d4b944527df608`.

Canonical machine-readable state on `main`:

- active package: `PKG-02 — Usable Local Server Beta`
- PKG-02 progress: `16/27 = 59.26%`
- `02.01` through `02.16`: `DONE`
- active task: `02.17`
- `02.18+`: blocked by sequence
- package complete: `false`

Current release candidate ID:

`c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`

Product version: `0.38.1`

## 4. Active task — frozen canonical acceptance

`02.17 — Resumable binary workspace transfer: chunk/offset enforcement, status/abort, finalize and SHA-256 digest verification.`

This wording comes from the frozen 27-task master execution plan and is the authority. Do not broaden the denominator or silently import requirements from stale preparation branches.

Package guardrails still apply:

- mutations remain behind authenticated `vsn-agent` / Core authorization;
- workspace containment must remain fail-closed;
- unsupported/invalid input must fail closed rather than be guessed;
- installer/updater/release/security/resilience/pentest work remains in later packages;
- do not begin/count `02.18` before `02.17` has genuine acceptance evidence and its accepted state is integrated.

## 5. 02.16 acceptance/integration record

PR #84 was merged only after exact-head certification and required regressions were green on `791fb1b2e940e696ecd5bd2d17d4b944527df608`.

Final 02.16 evidence on that head:

- workflow run `32655268446`, job `97232865171` — SUCCESS
- artifact `9497409818` (`pkg02-0216-workspace-text-files-github-hosted`)
- artifact digest `sha256:c4adca3428a186fda121fb993d6b22e22637aaf14c6942b3f50befe863877efa`
- `evidence.json` digest `sha256:34069b969ba72e38bf853253b21135de7772fb4108b066903a06bcb9c5e2f6ce`
- GitHub-hosted Windows/X64, IPC `127.0.0.1:39731`
- all frozen 02.16 checks true, audit valid with 20 events, cleanup fully true

Required latest-head gates also succeeded: Repository Governance, PKG-02 Acceptance Sequence, 02.01, 02.02, 02.08, 02.11–02.13, 02.14, and 02.15.

## 6. Current 02.17 working branch

Fresh branch created directly from canonical `main` `2364a5cf7460b6ee8bb0be31f8405181401335cb`:

`pkg02/0217-resumable-binary-transfer-main-sync`

This branch is the only valid implementation baseline for the current session unless live GitHub changes again.

## 7. Relevant open PRs / stale preparation

PR #50 — `PKG-02: prepare 02.17 resumable binary workspace transfer`

- state: OPEN, DRAFT, mergeable
- head: `pkg02/0217-resumable-binary-transfer` @ `6bb471bfcecd1df8b64f4053b72b4fc29e0b1602`
- base: stale `pkg02/0216-workspace-text-files` @ `d99c2932cf21b608d7a7488d258a19a9cb442239`
- created as stacked preparation when canonical task was much earlier
- **must not be merged or used as the implementation baseline**

Its body is useful only as an audit lead. It claims prepared checks for offset/status, abort/restart, bounded multi-chunk upload, finalize/digest equality, chunked download reconstruction, containment/unsafe transfer-id rejection, crash recovery, audit validity, and isolated runner state. Those checks must be independently reconciled against current source and the frozen canonical 02.17 wording before being adopted.

PR #50 also reports a possible recovery defect: `binary_upload_status()` runs `recover_binary_replace()` but `abort_binary_upload()` may not, potentially leaving a destination missing after a crash window. Treat that as an unverified lead until current `main` source and tests confirm it.

PR #51 and later preparation PRs are future-task material and remain out of scope.

## 8. Current blocker / reconciliation result

The post-merge contradiction was that this ledger still described 02.16/PR #84 as active while machine-readable `main` correctly advanced to 02.17. This branch reconciles that contradiction before product changes.

No 02.17 product code has been changed by this reconciliation commit.

The next engineering blocker is therefore not a state contradiction; it is to audit current `main` 02.17 implementation and compare it with stale PR #50 without trusting that branch.

## 9. Exact next actions

1. Inspect current-main binary transfer APIs, Core/Agent/CLI wiring, limits, containment and recovery logic.
2. Inspect PR #50 changed files/patch only as historical preparation and compare each change against current `main`.
3. Identify the smallest real gap relative to canonical 02.17 acceptance.
4. Add or adapt focused tests on the fresh branch that prove the gap before/with the fix.
5. Implement only the required 02.17 fix(es); do not include 02.18 work.
6. Run required format/Clippy/tests and build the current Agent/CLI path.
7. Add/repair an exact-head GitHub-hosted Windows 02.17 certification gate if current `main` lacks a valid one.
8. Do not mark 02.17 DONE or advance to 17/27 until exact-head acceptance evidence succeeds and required regressions remain green.
9. After successful acceptance, update machine-readable state to 17/27 with 02.18 active, update this ledger, commit focused changes, and prepare an acceptance PR. Do not merge it without explicit authorization.

## 10. Activity log

### 2026-08-23 — 02.16 integration and 02.17 activation reconciliation

- Verified PR #84 exact head `791fb1b2e940e696ecd5bd2d17d4b944527df608` unchanged, mergeable, with no unresolved review threads.
- Re-verified frozen required 02.16 gates green.
- Merged PR #84 with expected-head protection.
- Merge commit: `2364a5cf7460b6ee8bb0be31f8405181401335cb`.
- Re-read canonical `main` machine-readable state: PKG-02 `16/27 = 59.26%`, active `02.17`.
- Verified release candidate remains `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`.
- Verified stale draft PR #50 exists for 02.17 but is based on an obsolete preparation branch and is not authoritative.
- Created fresh current-main branch `pkg02/0217-resumable-binary-transfer-main-sync`.
- Reconciled this ledger before any 02.17 product implementation.
