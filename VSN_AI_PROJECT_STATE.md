# READ THIS FIRST — VSN Canonical AI Project State & Handoff

> **Purpose:** This is the canonical AI continuation ledger for `Vertex-Systems-Network/vsn-local-server`.
>
> **Critical rule:** live GitHub repository / PR / CI state wins over this file. If this file and live GitHub disagree, reconcile the file on the active working branch before doing product implementation.
>
> **Reconciliation note (2026-08-23):** this file was compacted because its previous body still described PKG-01 / PR #7 as current even though live `main` had already advanced to PKG-02 task 02.16. Detailed historical content remains available in Git history. The current-state assertions below replace those stale assertions; they do not change the roadmap or acceptance criteria.

## 1. Mandatory startup protocol

1. Read this file.
2. Read `docs/MASTER-EXECUTION-PLAN.md`.
3. Read `docs/MASTER-EXECUTION-STATUS.json`.
4. Read the active package tracker (`certification/pkg02-usable-local-beta-v1.json` while PKG-02 is active).
5. Read `docs/release-candidate-current.json` and record the candidate ID.
6. Query live GitHub for `main`, the active PR/branch head, open PRs relevant to the active task, review blockers, and workflow runs on the exact head.
7. If CI source differs from the visible branch, inspect the exact checkout/synthetic merge SHA before editing.
8. Work only on the current active task. Future task preparation is not authority to start or count a future task.
9. Do not mark a task DONE without real acceptance evidence on the required execution environment.
10. After a meaningful task/PR/head/candidate/blocker/state change, reconcile this ledger and the machine-readable state as appropriate.

## 2. Source-of-truth precedence

When sources disagree, use this order:

1. Live GitHub repository / PR / CI state.
2. Exact source checked out by the relevant CI job.
3. This continuation ledger.
4. Active package certification tracker.
5. `docs/MASTER-EXECUTION-STATUS.json`.
6. `docs/MASTER-EXECUTION-PLAN.md`.
7. Historical PR comments, old runs/artifacts, old handoffs, and chat history.

A PR branch may project the next task after accepting the current task, but canonical `main` does not advance until that accepted state is merged. Do not confuse branch projection with canonical-main state.

## 3. Current live repository state

Repository: `Vertex-Systems-Network/vsn-local-server`

Default branch: `main`

Authoritative `main` HEAD verified before this reconciliation commit:

`abbd6e8a7f59cafaf8695f4455738b90f41a81f0`

That commit merged PR #83 and certified PKG-02 task 02.15.

Canonical machine-readable state on `main`:

- active package: `PKG-02 — Usable Local Server Beta`
- PKG-02 progress: `15/27 = 55.56%`
- canonical active task: `02.16`
- `02.01` through `02.15`: `DONE`
- `02.16`: `IN_PROGRESS`
- `02.17+`: blocked by sequence
- package complete: `false`

Current release candidate ID on `main`:

`c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`

Product version: `0.38.1`

## 4. Active task — frozen acceptance

Canonical active task on `main` is:

`02.16 — Workspace text-file operations: list/read/write/mkdir/move/delete with root-protection and path-containment checks.`

The roadmap/denominator is frozen. Do not redefine this task and do not begin 02.17 implementation until 02.16 is accepted and the accepted state is integrated.

Required 02.16 behavior is bounded to the existing Agent-controlled local file boundary:

- authenticated Agent → IPC → Core → `vsn-files` → CLI path;
- list/read/write/mkdir/move/delete inside registered workspace roots;
- 1 MiB text-write bound;
- workspace-root move/delete protection;
- direct outside-workspace read/write rejection;
- Windows junction/reparse escape rejection;
- moving/deleting an in-workspace link/junction must mutate the link entry itself rather than its target;
- existing move destinations, including link entries, must be rejected;
- valid audit chain;
- runner-local state, IPC key state, sandbox, and Agent process must be cleaned/restored.

## 5. Relevant open PRs

### PR #84 — active 02.16 certification PR

Title: `PKG-02: certify 02.16 contained workspace text files`

Base: `main`

Base SHA: `abbd6e8a7f59cafaf8695f4455738b90f41a81f0`

Branch: `pkg02/0216-workspace-text-files-main-sync`

Pre-reconciliation head certified before this ledger fix:

`d132e51ed54cfd37aa6c6d70cc39ecd85f3e2c15`

State at verification: OPEN, not draft, mergeable, no unresolved review threads.

Important: PR #84 explicitly states that it must **not** be merged without explicit merge authorization.

PR #84 contains no 02.17 implementation. Its branch state projects `16/27 = 59.26%`, `02.16 DONE`, `02.17 IN_PROGRESS`, and `02.18+ BLOCKED`; that projection becomes canonical only after integration into `main`.

### Stale/future preparation PRs

Older stacked preparation PRs (including #45 for 02.16, #50 for 02.17, #51 for 02.18, and later-task preparation PRs) are not the implementation baseline for the active task. Do not merge/count them out of sequence.

Dependency-update and code-quality PRs are unrelated to the active PKG-02 acceptance task unless a verified prerequisite/regression requires them.

## 6. 02.16 implementation scope on PR #84

The accepted implementation scope is limited to:

- preserve final filesystem-entry identity for move/delete by validating the canonical parent instead of canonicalizing the mutation target itself;
- treat Windows reparse points as link entries so recursive delete cannot follow a junction into its target;
- preserve existing read/write containment and Agent/Core/CLI permission boundaries;
- reject existing move destinations including link entries;
- add cross-platform regression coverage for symlink/junction mutation identity and parent-link outside-workspace rejection;
- add exact-source GitHub-hosted Windows certification for the frozen 02.16 contract;
- repair the stale 02.01 workflow IPC expectation from `127.0.0.1:49731` to the current `vsn_ipc::IPC_ADDRESS` value `127.0.0.1:39731` so current-head regression certification remains valid.

No future-task implementation belongs in this PR.

## 7. Real acceptance evidence already obtained

PR #84 had exact-head GitHub-hosted Windows certification on pre-reconciliation head `d132e51ed54cfd37aa6c6d70cc39ecd85f3e2c15`:

- 02.16 run: `32643474502`
- job: `97204049324`
- conclusion: `SUCCESS`
- artifact: `9494373799` (`pkg02-0216-workspace-text-files-github-hosted`)
- artifact digest: `sha256:9fed95019149a8e800afdff7ca482339e4a2d826f40c58bb834f57d1bc9dd93f`
- `evidence.json` digest: `sha256:ea12cd06f43c051718397306fd31d1c30d655db644aa518b994260089cf3ce68`
- evidence source commit: exactly `d132e51ed54cfd37aa6c6d70cc39ecd85f3e2c15`
- runner: GitHub-hosted Windows/X64
- IPC: `127.0.0.1:39731`
- audit: valid, 20 events
- cleanup: Agent stopped, local state restored, IPC key restored, sandbox removed

`evidence.json` asserts all of these checks true:

- `list_read_write_mkdir_move_delete`
- `text_limit_1_mib`
- `workspace_root_protection`
- `direct_outside_rejected`
- `junction_outside_rejected`
- `junction_delete_preserves_target`
- `junction_move_preserves_target`
- `audit_chain_valid`

Required regressions on that same head were also successful:

- Repository Governance run `32643474512`, job `97204036282`
- PKG-02 Acceptance Sequence run `32643474507`, job `97204036169`
- 02.01 Agent Lifecycle run `32643474471`, job `97204036073`
- 02.02 Authenticated IPC run `32643474469`
- 02.08 Windows GitHub-Hosted Certification run `32643474609`
- 02.11–02.13 Runtime and Service Lifecycle run `32643474552`
- 02.14 Local Diagnostics run `32643474465`
- 02.15 Container Baseline run `32643474453`

Some unrelated legacy PKG-01 workflows still auto-run on PR heads and may report failures. They are not allowed to silently redefine the frozen PKG-02 02.16 acceptance contract; use Repository Governance, PKG-02 sequence enforcement, the task-specific gate, and the explicitly required regressions as the authority for this task.

## 8. Reconciliation status and current blocker

The previous version of this ledger was contradictory because it still claimed PKG-01 / PR #7 was active. That contradiction is the reason for this state-only commit.

This ledger update does **not** change product behavior or acceptance criteria. Because it changes the PR branch HEAD, the exact-head 02.16/current-regression gates must be checked again on the new head before calling PR #84 acceptance-ready at its latest HEAD. Do not start 02.17 while that reconciliation verification is pending.

If those fresh exact-head gates pass, the remaining blocker is integration authorization: PR #84 may then be acceptance-ready, but must not be merged without explicit merge authorization.

## 9. Exact next action

1. Read the new live PR #84 head after this reconciliation commit.
2. Fetch workflow runs bound to that exact head.
3. Require success for Repository Governance, PKG-02 Acceptance Sequence, 02.16 Workspace Text Files, and the required current-head PKG-02 regressions listed above.
4. If a required gate fails, inspect the exact failing job/checkout SHA and fix only that blocker; do not start 02.17.
5. If all required gates pass, update the PR conversation/body with the exact latest-head evidence if needed, but do not make another source/state commit solely to copy volatile run IDs.
6. Do not merge PR #84 without explicit merge authorization.
7. Only after accepted 02.16 state is integrated into canonical `main` does `02.17 — Resumable binary workspace transfer and digest` become the canonical active task.

## 10. Activity log

### 2026-08-23 — canonical state reconciliation before further engineering

- Verified live `main` at `abbd6e8a7f59cafaf8695f4455738b90f41a81f0`.
- Verified canonical `main` state: PKG-02 `15/27`, active `02.16`.
- Verified current release candidate `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`.
- Verified PR #84 as the relevant active-task PR, base `main`, pre-reconciliation head `d132e51ed54cfd37aa6c6d70cc39ecd85f3e2c15`, open/mergeable/not-draft, no unresolved review threads.
- Verified 02.16 exact-head GitHub-hosted Windows acceptance artifact and evidence directly; all frozen checks, audit validity, and cleanup passed.
- Identified the stale `VSN_AI_PROJECT_STATE.md` as the remaining repository-state contradiction.
- Reconciled this ledger only; no 02.17 implementation and no roadmap/acceptance changes were made.
