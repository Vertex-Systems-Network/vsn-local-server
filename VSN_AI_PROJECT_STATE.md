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
3. This continuation ledger.
4. Active package certification tracker.
5. `docs/MASTER-EXECUTION-STATUS.json`.
6. `docs/MASTER-EXECUTION-PLAN.md`.
7. Historical PR comments, old branches/runs/artifacts, and chat history.

## 3. Current canonical `main`

Repository: `Vertex-Systems-Network/vsn-local-server`

Canonical `main` HEAD:

`37711ba685bccfc0c7a4dd66b286ac8dac2e5ee6`

That signed/verified merge commit integrated PR #86 (`PKG-02: certify 02.18 bounded direct terminal execution`).

Canonical machine-readable state on `main` remains:

- active package: `PKG-02 — Usable Local Server Beta`
- progress: `18/27 = 66.67%`
- `02.01` through `02.18`: `DONE`
- canonical active task: `02.19 — Persistent pipe terminal sessions`
- `02.20+`: blocked until accepted 02.19 state is integrated
- package complete: `false`

Current release candidate remains unchanged:

- candidate ID: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`
- product version: `0.38.1`
- file count: `414`
- profile: `release-inputs-v1`
- source fingerprint: `c60d0f3ac47ac0eb89a133591e7b243c34c0f045737e407ea4f5ae7fba244d10`

## 4. Active acceptance PR and branch

PR #87 — `PKG-02: certify 02.19 persistent pipe terminal sessions`

Branch:

`pkg02/0219-persistent-pipe-main-sync`

Base is canonical `main` `37711ba685bccfc0c7a4dd66b286ac8dac2e5ee6`.

The PR is open, non-draft and mergeable at the time of this reconciliation. It must **not** be merged without explicit merge authorization.

No 02.20+ product implementation is included in PR #87.

## 5. Frozen 02.19 acceptance wording

`02.19 — Persistent pipe terminal sessions: start/write/read-wait/status/stop/list/remove with bounded output.`

The frozen 27-task master plan remains authoritative. The denominator, wording and task order were not changed.

Package guardrails remain:

- execution/mutation stays behind authenticated `vsn-agent` / Core authorization;
- session cwd/program containment fails closed;
- long-poll and retained output stay bounded;
- child-stdin backpressure must not make unrelated session lifecycle operations unresponsive;
- unsupported input is not guessed;
- installer/updater/release/security/resilience/pentest work remains in later packages;
- no 02.20 implementation may begin before accepted 02.19 state is merged and canonical state is re-read.

## 6. 02.19 scoped implementation

Fresh post-02.18 current-main audit found two concrete 02.19 gaps:

1. `write_session()` held the global session-registry mutex while performing potentially blocking child-stdin `write_all/flush`, allowing one backpressured child to stall unrelated session operations.
2. `terminal.session.read-wait` could wait up to 5 seconds server-side while the generic IPC client response timeout was also 5 seconds, leaving no transport/serialization margin.

Scoped product fixes:

- per-session stdin is held behind `Arc<Mutex<ChildStdin>>`; the global session map is released before blocking write/flush while same-session writes remain serialized;
- only `terminal.session.read-wait` gets a bounded 7-second IPC client response timeout; unrelated IPC commands retain 5 seconds and `terminal.exec` retains 65 seconds.

Historical preparation PR #53 was inspected only as an audit lead. It is stale stacked preparation and is not the implementation baseline.

A stale 02.18 certification source-syntax invariant was also made behavior-preserving after the IPC timeout refactor. 02.19 strict Clippy is scoped to changed crates (`vsn-terminal` and `vsn-ipc`); `vsn-core` tests and full release Agent/CLI behavioral acceptance remain required.

## 7. Genuine 02.19 behavioral acceptance

Accepted behavioral/state-preprojection source:

`bd1687c0e04e1ba022cd8d02b78a15546ebb9290`

Corrected GitHub-hosted Windows acceptance:

- workflow run `32669585231`
- job `97268208441`
- result: `SUCCESS`
- artifact `9501114320` (`pkg02-0219-persistent-pipe-terminal-github-hosted`)
- artifact digest `sha256:78f0e94c266a0837f1f89df9a7b758226438d93a0f566c72565cacb02132ac68`
- independently verified `evidence.json` digest `sha256:808f06539d0d1f6b816b9030939bb883fc3ed6c54a33dadecb8c9fa09eca4e85`
- runner environment: GitHub-hosted Windows/X64
- IPC: `127.0.0.1:39731`
- audit chain: valid, 26 events
- cleanup: Agent stopped, LOCALAPPDATA restored, IPC key restored, sandbox removed

Accepted evidence proves:

- start/write/read-wait/status/stop/list/remove;
- separate stdout/stderr interaction;
- bounded long-poll at about 3.06 seconds;
- bounded retained output of 65,536 characters per stream with 425,984 dropped bytes reported per stream after oversized output;
- genuine 192 KiB sub-limit stdin write blocked by a deliberately non-reading child;
- unrelated session status remained responsive at 75 ms while that write was blocked;
- stop of the same blocked session remained responsive at 52 ms;
- workspace cwd containment, Windows-junction containment and outside-program rejection;
- valid audit chain and complete cleanup.

### Rejected false-positive artifact

An earlier workflow-success artifact on source `36b33bd4824e7e4339d7e5d56df2551234ea4f7b` is **not** accepted as 02.19 evidence. Independent artifact inspection showed the supposed blocked writer was actually rejected because its input exceeded the 256 KiB terminal input bound. The certification was corrected to use a 192 KiB sub-limit payload and to prove the writer was genuinely in-flight before responsiveness assertions. Only run `32669585231` / artifact `9501114320` is accepted.

## 8. Required same-head regressions on accepted behavioral source

All required gates passed on `bd1687c0e04e1ba022cd8d02b78a15546ebb9290`:

- Repository Governance run `32669585128`, job `97268207834`
- PKG-02 Acceptance Sequence run `32669585159`, job `97268208147`
- 02.02 Authenticated IPC run `32669585173`, job `97268208183`
- 02.08 Windows GitHub-Hosted Certification run `32669585145`, job `97268208071`
- 02.16 Workspace Text Files run `32669585123`, job `97268208052`
- 02.17 Resumable Binary Workspace Transfer run `32669585124`, job `97268207814`
- 02.18 Bounded Direct Terminal Execution run `32669585176`, job `97268208332`
- 02.19 Persistent Pipe Terminal Sessions run `32669585231`, job `97268208441`

Known legacy PKG-01 npm-graph failures are outside the frozen 02.19 acceptance set and do not redefine this task's criteria.

## 9. PR-branch state projection after genuine 02.19 acceptance

The PR branch projects:

- PKG-02 progress: `19/27 = 70.37%`
- `02.19`: `DONE`
- projected next task: `02.20 — Interactive PTY/ConPTY session lifecycle`
- `02.21+`: blocked by sequence

Matching branch-only projections are recorded in:

- `docs/MASTER-EXECUTION-STATUS.json`
- `certification/pkg02-usable-local-beta-v1.json`
- `docs/MASTER-EXECUTION-PLAN.md`
- `README.md`
- this handoff ledger

This is a **PR-branch projection only**. Canonical `main` remains `18/27 = 66.67%`, active `02.19`, until PR #87 is explicitly authorized and integrated.

Because these state projection commits create a newer source head than `bd1687c0e04e1ba022cd8d02b78a15546ebb9290`, the resulting final PR head must receive a fresh exact-head certification wave before PR #87 can be called acceptance-ready.

## 10. Frozen next task after successful integration

`02.20 — Interactive PTY/ConPTY session lifecycle: start/write/read-wait/resize/status/stop/remove plus bounded scrollback/recovery behavior.`

Do **not** implement 02.20 on this PR branch. After PR #87 receives final-head acceptance, an explicitly authorized merge must occur first; then canonical `main`, machine-readable state, open PRs and frozen 02.20 acceptance must be re-read before any 02.20 engineering.

## 11. Exact continuation point

1. Re-read live PR #87 after this handoff commit and capture its exact new head.
2. Verify all five branch projections agree on `19/27`, `02.19 DONE`, projected `02.20 active` while canonical `main` remains `37711ba685bccfc0c7a4dd66b286ac8dac2e5ee6` / `18/27 active 02.19`.
3. Require fresh exact-head success on the final PR head for Repository Governance, PKG-02 Acceptance Sequence, 02.02, 02.08, 02.16, 02.17, 02.18 and 02.19.
4. Download and independently verify the final-head 02.19 artifact: exact `source_commit`, artifact digest, evidence digest, all behavioral checks, genuine blocked-write markers/latencies, audit validity and cleanup.
5. Update PR #87 metadata with final-head evidence only after all required gates pass; avoid another source commit after final evidence.
6. Do **not** merge PR #87 without explicit merge authorization.
7. After an explicitly authorized merge, re-read canonical `main` and machine-readable state. Only if canonical state is then `19/27 = 70.37%` with `02.20` active may 02.20 implementation begin.

## 12. Activity log

### 2026-08-24 — 02.18 integration and 02.19 activation

- PR #86 was explicitly authorized and merged as signed/verified `main` commit `37711ba685bccfc0c7a4dd66b286ac8dac2e5ee6`.
- Canonical state was re-read and confirmed `18/27 = 66.67%`, active `02.19` before any 02.19 implementation.
- Fresh branch `pkg02/0219-persistent-pipe-main-sync` was created from integrated main; stale PR #53 was used only as an audit lead.

### 2026-08-24 — 02.19 implementation and genuine acceptance

- Fixed global-registry-lock retention across blocking child stdin writes with per-session stdin locking.
- Added a bounded 7-second IPC response margin only for `terminal.session.read-wait`.
- Added GitHub-hosted Windows 02.19 behavioral certification.
- Corrected a stale 02.18 certification source-syntax invariant and removed unrelated transitive Clippy overreach while retaining required tests/behavioral gates.
- Rejected the first apparently-green 02.19 artifact after independent inspection proved it did not actually exercise backpressure.
- Corrected the certification to prove genuine 192 KiB blocked-write behavior and same/unrelated-session responsiveness.
- Accepted corrected source `bd1687c0e04e1ba022cd8d02b78a15546ebb9290`; all eight required same-head gates passed.
- Independently verified artifact `9501114320`, artifact digest `sha256:78f0e94c266a0837f1f89df9a7b758226438d93a0f566c72565cacb02132ac68`, evidence digest `sha256:808f06539d0d1f6b816b9030939bb883fc3ed6c54a33dadecb8c9fa09eca4e85`, valid 26-event audit and complete cleanup.
- Advanced branch-only state projection to `19/27 = 70.37%`, `02.19 DONE`, projected `02.20 active`.
- Final-head exact-source recertification is now required before PR #87 can be considered acceptance-ready.
