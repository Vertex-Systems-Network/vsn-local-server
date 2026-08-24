# READ THIS FIRST — VSN Canonical AI Project State & Handoff

> **Purpose:** continuation ledger for `Vertex-Systems-Network/vsn-local-server`.
>
> **Critical rule:** live GitHub repository / PR / CI state wins over this file. If this file and live GitHub disagree, stop product implementation and reconcile first. Never silently change the roadmap, acceptance criteria, denominator, task order, or prerequisite rules.

## 1. Mandatory startup protocol

1. Read this file.
2. Read `docs/MASTER-EXECUTION-PLAN.md`.
3. Read `docs/MASTER-EXECUTION-STATUS.json`.
4. Read `certification/pkg02-usable-local-beta-v1.json` while PKG-02 is active.
5. Read `docs/release-candidate-current.json` and record the candidate ID.
6. Query live GitHub for `main`, the active acceptance PR/branch, relevant preparation PRs, review blockers, and exact-head workflow runs.
7. Work only on the current canonical task. A future task may be inspected only as an audit lead; do not implement/count it before its prerequisite is integrated.
8. Mark DONE only with genuine acceptance evidence on the required environment.
9. Reconcile machine-readable state and this handoff only after genuine acceptance.
10. Do not merge an acceptance PR unless explicit merge authorization is given.

## 2. Source-of-truth precedence

1. Live GitHub repository / PR / CI state.
2. Exact source checked out by the relevant CI job.
3. Active package certification tracker.
4. `docs/MASTER-EXECUTION-STATUS.json`.
5. This continuation ledger.
6. `docs/MASTER-EXECUTION-PLAN.md`.
7. Historical PR comments, old branches/runs/artifacts, and chat history.

## 3. Current canonical `main`

Repository: `Vertex-Systems-Network/vsn-local-server`

Canonical `main` HEAD:

`45d04b051c92a65ef247bccddc1375d69df3a7c1`

That signed/verified merge commit integrated PR #87 (`PKG-02: certify 02.19 persistent pipe terminal sessions`).

Canonical machine-readable state on `main` is:

- active package: `PKG-02 — Usable Local Server Beta`
- progress: `19/27 = 70.37%`
- `02.01` through `02.19`: `DONE`
- canonical active task: `02.20 — Interactive PTY/ConPTY session lifecycle`
- `02.21+`: blocked until accepted 02.20 state is integrated
- package complete: `false`

Current release candidate remains unchanged:

- candidate ID: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`
- product version: `0.38.1`
- profile: `release-inputs-v1`

The previous merge left stale PR #87 prose in several narrative files even though the machine-readable canonical state correctly advanced to 19/27 active 02.20. PR #88 reconciles that stale prose together with the accepted 02.20 branch projection; the frozen roadmap and denominator are unchanged.

## 4. Active acceptance PR and branch

PR #88 — `PKG-02: certify 02.20 PTY ConPTY lifecycle`

Branch:

`pkg02/0220-pty-conpty-main-sync`

Base is canonical `main` `45d04b051c92a65ef247bccddc1375d69df3a7c1`.

PR #88 is open, draft and mergeable at the time of this reconciliation. It must **not** be merged without explicit merge authorization.

No 02.21+ product implementation is included in PR #88.

## 5. Frozen 02.20 acceptance wording

`02.20 — Interactive PTY/ConPTY session lifecycle: start/write/read-wait/resize/status/stop/remove plus bounded scrollback/recovery behavior.`

The frozen 27-task master plan remains authoritative. The denominator, wording and task order were not changed.

Package guardrails remain:

- execution/mutation stays behind authenticated `vsn-agent` / Core authorization;
- PTY cwd/program containment fails closed;
- read-wait, live output and scrollback remain bounded;
- recovery metadata remains durable and replace-safe;
- stop/remove and recovery/scrollback cleanup are explicit;
- unsupported input is not guessed;
- installer/updater/release/security/resilience/pentest work remains in later packages;
- no 02.21 implementation may begin before accepted 02.20 state is merged and canonical state is re-read.

## 6. 02.20 scoped implementation

Fresh post-02.19 current-main audit and exact-head Windows acceptance identified and fixed only 02.20-scoped issues:

1. PTY writes could hold the global PTY-session registry mutex while performing blocking writer I/O. The writer is now held behind `Arc<Mutex<Box<dyn Write + Send>>>`, allowing the global map lock to be released before blocking write/flush while same-session writes stay serialized.
2. PTY recovery checkpoint replacement on Windows needed replace-safe semantics. Recovery writes now use a Windows replacement helper backed by `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`; non-Windows keeps `std::fs::rename`.
3. PTY resize now persists the updated rows/cols into the recovery checkpoint.
4. `terminal.pty.read-wait` now receives the same bounded 7-second IPC response margin needed for a server-side long-poll.
5. Certification now behaves as a real ConPTY host by answering the initial `ESC[6n` DSR query with raw `ESC[1;1R`, and it deterministically waits for durable scrollback instead of relying on a fixed sleep.
6. Evidence hardening corrected the audit-event measurement and explicitly removes/verifies recovery and scrollback metadata for the temporary idle PTY.

No 02.21 product functionality was added.

## 7. Genuine 02.20 behavioral acceptance

Accepted behavioral/state-preprojection source:

`dfd20329434d7df59f571d3f3b858d9856733019`

GitHub-hosted Windows acceptance:

- workflow run `32714436581`
- job `97392588770`
- result: `SUCCESS`
- artifact `9515572215` (`pkg02-0220-pty-conpty-lifecycle-github-hosted`)
- artifact digest `sha256:7b9f15c1401aa6613c02d62f60a9064e9e63511ef5b110131783c7488002049b`
- independently verified `evidence.json` digest `sha256:3d92e4a3d86138cc4a470b7da36b3e767b3553f42d5bc04574d14c11f44db072`
- runner environment: GitHub-hosted Windows/X64
- IPC: `127.0.0.1:39731`
- audit chain: valid, 41 events
- cleanup: Agent stopped; LOCALAPPDATA and IPC key restored; junction, sessions, recovery metadata, scrollback metadata and sandbox removed

Accepted evidence proves:

- exact source and GitHub-hosted Windows execution;
- ConPTY terminal-host DSR handshake handling;
- PTY start/list/write/read-wait/resize/status/stop/remove;
- resize persistence into live status and the recovery checkpoint;
- bounded no-output read-wait at 3053 ms;
- bounded live output with 374116 dropped bytes reported after oversized output;
- durable scrollback retained at 1423091 bytes independently of live truncation;
- first scrollback read bounded to 262144 bytes;
- running and stopped recovery checkpoints;
- replacement of an already-existing recovery checkpoint on Windows;
- direct outside cwd, Windows-junction escape and outside-program rejection;
- invalid/missing resize rejection;
- valid audit chain and complete session/recovery/scrollback cleanup.

### Evidence hardening

An earlier successful behavioral head was not used as final accepted evidence after independent artifact inspection found two evidence-quality issues: the harness counted the scalar audit field rather than its value, and it did not explicitly remove/verify the temporary idle PTY recovery/scrollback metadata. Source `dfd20329434d7df59f571d3f3b858d9856733019` corrected both and was re-certified successfully. Only artifact `9515572215` is accepted for final 02.20 behavioral evidence.

## 8. Required same-head regressions on accepted behavioral source

All required gates passed on `dfd20329434d7df59f571d3f3b858d9856733019`:

- Repository Governance run `32714436268`, job `97392585394`
- PKG-02 Acceptance Sequence run `32714436551`, job `97392585875`
- 02.02 Authenticated IPC run `32714436420`, job `97392586064`
- 02.08 Windows GitHub-Hosted Certification run `32714436240`, job `97392585990`
- 02.16 Workspace Text Files run `32714436116`, job `97392585339`
- 02.17 Resumable Binary Workspace Transfer run `32714436205`, job `97392585666`
- 02.18 Bounded Direct Terminal Execution run `32714436487`, job `97392586043`
- 02.19 Persistent Pipe Terminal Sessions run `32714436292`, job `97392585863`
- 02.20 PTY ConPTY Lifecycle run `32714436581`, job `97392588770`

Known legacy PKG-01 npm-graph failures are outside the frozen 02.20 acceptance set and do not redefine this task's criteria.

## 9. PR-branch state projection after genuine 02.20 acceptance

The PR branch projects:

- PKG-02 progress: `20/27 = 74.07%`
- `02.20`: `DONE`
- projected next task: `02.21 — Read-only local preview fetch`
- `02.22+`: blocked by sequence

Matching branch-only projections are recorded in:

- `certification/pkg02-usable-local-beta-v1.json`
- `docs/MASTER-EXECUTION-STATUS.json`
- `docs/MASTER-EXECUTION-PLAN.md`
- `README.md`
- this handoff ledger

This is a **PR-branch projection only**. Canonical `main` remains `19/27 = 70.37%`, active `02.20`, until PR #88 is explicitly authorized and integrated.

Because state-projection reconciliation creates a newer source head than `dfd20329434d7df59f571d3f3b858d9856733019`, the final PR head must receive a fresh exact-head certification wave before PR #88 can be called acceptance-ready.

## 10. Frozen next task after successful integration

`02.21 — Read-only local preview fetch against loopback development servers with bounded response handling.`

Do **not** implement 02.21 on this PR branch. After PR #88 receives final-head acceptance, an explicitly authorized merge must occur first; then canonical `main`, machine-readable state, open PRs and frozen 02.21 acceptance must be re-read before any 02.21 engineering.

## 11. Exact continuation point

1. Re-read live PR #88 after the state-reconciliation commit and capture its exact final head.
2. Verify all five branch projections agree on `20/27`, `02.20 DONE`, projected `02.21 active` while canonical `main` remains `45d04b051c92a65ef247bccddc1375d69df3a7c1` / `19/27 active 02.20`.
3. Require fresh exact-head success on the final PR head for Repository Governance, PKG-02 Acceptance Sequence, 02.02, 02.08, 02.16, 02.17, 02.18, 02.19 and 02.20.
4. Download and independently verify the final-head 02.20 artifact: exact `source_commit`, artifact digest, evidence digest, all checks, audit validity and complete cleanup.
5. Update PR #88 metadata with final-head evidence only after all required gates pass; avoid another source commit after final evidence.
6. Do **not** merge PR #88 without explicit merge authorization.
7. After an explicitly authorized merge, re-read canonical `main` and machine-readable state. Only if canonical state is then `20/27 = 74.07%` with `02.21` active may 02.21 implementation begin.

## 12. Activity log

### 2026-08-24 — 02.19 integration and 02.20 activation

- PR #87 was integrated as signed/verified `main` commit `45d04b051c92a65ef247bccddc1375d69df3a7c1`.
- Canonical machine-readable state was re-read and confirmed `19/27 = 70.37%`, active `02.20` before 02.20 engineering.
- Fresh branch `pkg02/0220-pty-conpty-main-sync` was created from integrated main.

### 2026-08-24 — 02.20 implementation and genuine acceptance

- Isolated PTY writer locking from the global session registry.
- Made Windows recovery checkpoint replacement durable and replace-safe.
- Persisted resize state into recovery checkpoints.
- Added the PTY read-wait IPC timeout margin.
- Added exact-head GitHub-hosted Windows 02.20 certification and corrected ConPTY terminal-host handshake handling.
- Rejected an earlier successful artifact as final evidence after independent inspection found inaccurate audit counting and incomplete explicit idle metadata cleanup.
- Accepted source `dfd20329434d7df59f571d3f3b858d9856733019`; all nine required same-head gates passed.
- Independently verified artifact `9515572215`, artifact digest `sha256:7b9f15c1401aa6613c02d62f60a9064e9e63511ef5b110131783c7488002049b`, evidence digest `sha256:3d92e4a3d86138cc4a470b7da36b3e767b3553f42d5bc04574d14c11f44db072`, valid 41-event audit and complete cleanup.
- Advanced branch-only state projection to `20/27 = 74.07%`, `02.20 DONE`, projected `02.21 active`.
- Final-head exact-source recertification is required before PR #88 can be considered acceptance-ready.
