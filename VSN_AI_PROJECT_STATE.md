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
6. Query live GitHub for `main`, active-task PRs/branches, relevant preparation PRs, review blockers, and exact-head workflow runs.
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

`2f46665d3da58b8537ffc34288348b7fcd744d90`

That signed/verified merge commit integrated PR #85 (`PKG-02: certify 02.17 resumable binary workspace transfer`).

Canonical machine-readable state on `main` remains:

- active package: `PKG-02 — Usable Local Server Beta`
- progress: `17/27 = 62.96%`
- `02.01` through `02.17`: `DONE`
- canonical active task: `02.18`
- `02.19+`: blocked until accepted 02.18 state is integrated
- package complete: `false`

Current release candidate remains unchanged:

- candidate ID: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`
- product version: `0.38.1`
- file count: `414`
- profile: `release-inputs-v1`
- source fingerprint: `c60d0f3ac47ac0eb89a133591e7b243c34c0f045737e407ea4f5ae7fba244d10`

## 4. Active acceptance PR and branch

PR #86 — `PKG-02: certify 02.18 bounded direct terminal execution`

Branch:

`pkg02/0218-bounded-terminal-main-sync`

Base is canonical `main` `2f46665d3da58b8537ffc34288348b7fcd744d90`.

PR #86 is OPEN, non-draft and mergeable. It must **not** be merged without explicit merge authorization.

No 02.19 product implementation is included in PR #86.

## 5. Frozen 02.18 acceptance wording

`02.18 — Bounded direct terminal execution inside an allowed workspace, including timeout/output limits and invalid-command handling.`

The frozen 27-task master plan remains authoritative. The denominator, wording and task order were not changed.

Package guardrails remain:

- execution/mutation stays behind authenticated `vsn-agent` / Core authorization;
- cwd/workspace containment fails closed;
- timeout and output handling stay bounded;
- invalid/missing commands fail cleanly;
- unsupported input is not guessed;
- installer/updater/release/security/resilience/pentest work remains in later packages;
- no 02.19 product work may begin before accepted 02.18 state is merged and canonical state is re-read.

## 6. 02.18 implementation and accepted behavioral source

Current-main audit found three concrete gaps in the direct execution path:

1. `read_bounded()` stopped reading a child pipe after the retention cap, which could change child pipe/backpressure/exit semantics.
2. Independent 512 KiB stdout/stderr retention could produce a response too close to the authenticated local IPC 1 MiB frame ceiling once JSON escaping/envelope overhead was included.
3. The generic IPC client read timeout was 5 seconds while direct terminal execution defaults to a 30-second process timeout, so a genuine process timeout could not return its result through the CLI.

Scoped product fixes:

- direct stdout/stderr readers retain at most 64 KiB per stream but continue draining to EOF;
- worst-case escaped retained output remains frame-safe;
- only `terminal.exec` receives a bounded 65-second IPC response wait; other IPC commands retain their existing 5-second client timeout.

Focused regressions prove drain-after-cap behavior, worst-case escaped JSON response budget and command-specific IPC timeout selection.

Historical preparation PR #51 was inspected only as an audit lead. It is stale stacked preparation and is not the implementation baseline.

Exact accepted behavioral/state-preprojection source:

`95046275b6bb984dd7a9475403f04931204b4041`

Task-specific GitHub-hosted Windows acceptance:

- workflow run `32664473279`
- job `97255635060`
- result: `SUCCESS`
- artifact `9499814557` (`pkg02-0218-bounded-direct-terminal-github-hosted`)
- artifact digest `sha256:d1019a03a425e38a6f09636d82af05af25815bdc75b7b93b3a5b25ac6e550f41`
- independently verified `evidence.json` digest `sha256:8831f63a5b2a726e83cdcab4ea64e7234494b257ff1cdf2fdcedd778010b59f5`
- runner: `GitHub Actions 1000015765`
- runner environment: GitHub-hosted Windows/X64
- IPC: `127.0.0.1:39731`
- audit chain: valid, 10 events
- cleanup: Agent stopped, LOCALAPPDATA restored, IPC key restored, sandbox removed

All 02.18 evidence checks were true:

- child exit status preserved;
- simultaneous stdout/stderr verified;
- drain-safe truncation verified;
- frame-safe output verified;
- timeout verified;
- invalid command rejected;
- workspace cwd containment verified;
- Windows-junction cwd containment verified;
- outside program rejected;
- audit chain valid.

Independent artifact sanity checks additionally showed:

- retained stdout: `65536` bytes, truncated true;
- retained stderr: `65536` bytes, truncated true;
- high-output CLI result size: `786702` bytes, below the 900 KiB acceptance budget;
- high-output child exit code: `0`;
- timeout result: `timed_out=true`;
- process-reported duration: `30022 ms`;
- end-to-end elapsed time: about `30087 ms`.

## 7. Required same-head regressions on accepted behavioral head

All required current-head gates passed on `95046275b6bb984dd7a9475403f04931204b4041`:

- Repository Governance run `32664473266`, job `97255634835`
- PKG-02 Acceptance Sequence run `32664473208`, job `97255634685`
- 02.02 Authenticated IPC run `32664473264`, job `97255634696`
- 02.08 Windows GitHub-Hosted Certification run `32664473267`, job `97255634833`
- 02.16 Workspace Text Files run `32664473299`, job `97255635020`
- 02.17 Resumable Binary Workspace Transfer run `32664473247`, job `97255634834`
- 02.18 Bounded Direct Terminal Execution run `32664473279`, job `97255635060`

Additional PKG-02 regressions in the same wave were also green.

Legacy PKG-01 Desktop/Dashboard npm-graph and fresh-checkout workflows still report their known failures. They are not frozen 02.18 acceptance gates and must not silently redefine 02.18 acceptance.

## 8. Branch state projection after genuine 02.18 acceptance

After the genuine behavioral acceptance above, the PR branch projects:

- PKG-02 progress: `18/27 = 66.67%`
- `02.18`: `DONE`
- projected next task: `02.19 — Persistent pipe terminal sessions`
- `02.20+`: blocked by sequence

The matching projection is recorded in:

- `docs/MASTER-EXECUTION-STATUS.json`
- `certification/pkg02-usable-local-beta-v1.json`
- `docs/MASTER-EXECUTION-PLAN.md`
- `README.md`
- this handoff ledger

This is a **PR-branch projection only**. Canonical `main` remains 17/27 active 02.18 until PR #86 is integrated.

Because these state-only projection commits create a newer source head than `95046275b6bb984dd7a9475403f04931204b4041`, the final PR head must receive a fresh exact-head certification wave before PR #86 can be called acceptance-ready.

## 9. Frozen next task after integration

`02.19 — Persistent pipe terminal sessions: start/write/read-wait/status/stop/list/remove with bounded output.`

Do not implement 02.19 on this branch before 02.18 is final-head re-certified, explicitly authorized for merge, merged into `main`, and canonical state is re-read as 18/27 active 02.19.

## 10. Exact next actions

1. Re-read live PR #86 head after this state reconciliation and verify all five projections agree on `18/27`, `02.18 DONE`, projected `02.19 active`.
2. Verify canonical `main` remains `2f46665d3da58b8537ffc34288348b7fcd744d90`; if it moved, stop and reconcile.
3. Require fresh exact-head success on the final PR head for Repository Governance, PKG-02 Acceptance Sequence, 02.18, 02.02, 02.08, 02.16 and 02.17.
4. Verify the final-head 02.18 artifact binds `source_commit` to that exact final PR head and independently verify evidence digest/checks/cleanup.
5. Update PR #86 metadata with final-head evidence only after those gates pass; avoid another source commit after final evidence.
6. Do **not** merge PR #86 without explicit merge authorization.
7. After an explicitly authorized merge, re-read canonical `main` and machine-readable state. Only if canonical state is then `18/27` with `02.19` active may 02.19 product implementation begin.

## 11. Activity log

### 2026-08-24 — 02.17 final acceptance and integration

- Re-verified PR #85 exact head `efdd65b3680cdcdb2c02c02a59a8c0f113339af7` unchanged, mergeable and acceptance-ready.
- Merged PR #85 with expected-head protection as `2f46665d3da58b8537ffc34288348b7fcd744d90`.
- Re-read canonical machine-readable state: PKG-02 `17/27 = 62.96%`, active `02.18`.
- Created fresh 02.18 branch directly from canonical main and reconciled the stale post-merge handoff before product work.

### 2026-08-24 — 02.18 implementation, acceptance and state projection

- Audited current-main direct terminal execution and independently verified the two stale PR #51 defect leads.
- Found a third current-main gap: generic 5-second IPC response timeout made the 30-second direct-exec timeout unreturnable through CLI.
- Added only the three scoped 02.18 fixes and focused regressions.
- Added a fresh exact-head GitHub-hosted Windows 02.18 workflow and certification using IPC `127.0.0.1:39731`.
- Exact source `95046275b6bb984dd7a9475403f04931204b4041` passed 02.18 and all required same-head regressions.
- Independently verified artifact `9499814557`, artifact digest `sha256:d1019a03a425e38a6f09636d82af05af25815bdc75b7b93b3a5b25ac6e550f41`, evidence digest `sha256:8831f63a5b2a726e83cdcab4ea64e7234494b257ff1cdf2fdcedd778010b59f5`, all checks true, valid 10-event audit and complete cleanup.
- Advanced the PR-branch state projection to `18/27 = 66.67%`, `02.18 DONE`, projected `02.19 active`.
- Final-head exact-source recertification remains required before PR #86 can be called acceptance-ready.
