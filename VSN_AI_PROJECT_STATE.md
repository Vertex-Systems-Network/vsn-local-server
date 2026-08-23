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

That signed/verified merge commit integrated PR #85 (`PKG-02: certify 02.17 resumable binary workspace transfer`) with parents:

- previous canonical main `2364a5cf7460b6ee8bb0be31f8405181401335cb`
- exact accepted PR head `efdd65b3680cdcdb2c02c02a59a8c0f113339af7`

Canonical machine-readable state on `main`:

- active package: `PKG-02 — Usable Local Server Beta`
- progress: `17/27 = 62.96%`
- `02.01` through `02.17`: `DONE`
- active task: `02.18`
- `02.19+`: blocked by sequence
- package complete: `false`

Current release candidate:

- candidate ID: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`
- product version: `0.38.1`
- file count: `414`
- profile: `release-inputs-v1`
- source fingerprint: `c60d0f3ac47ac0eb89a133591e7b243c34c0f045737e407ea4f5ae7fba244d10`

## 4. 02.17 final acceptance and integration record

Final exact PR head:

`efdd65b3680cdcdb2c02c02a59a8c0f113339af7`

Required final-head gates all passed:

- Repository Governance run `32662309642`, job `97250236505`
- PKG-02 Acceptance Sequence run `32662309614`, job `97250236461`
- 02.02 Authenticated IPC run `32662309695`, job `97250236704`
- 02.08 Windows GitHub-Hosted Certification run `32662309566`, job `97250238024`
- 02.16 Workspace Text Files run `32662309488`, job `97250255845`
- 02.17 Resumable Binary Workspace Transfer run `32662309493`, job `97250248523`

Final 02.17 artifact:

- artifact `9499266983` (`pkg02-0217-resumable-binary-transfer-github-hosted`)
- artifact digest `sha256:685338455513216cb3f2a5209391fa3fb7b03b76fce9de5ddbbfbbd73818b36e`
- independently verified `evidence.json` digest `sha256:b2aaae2ca7e9dfb2a73c0839a91fadc88d29bbfb4fe3f5ee1ef9a8857563f0a1`
- source commit exactly `efdd65b3680cdcdb2c02c02a59a8c0f113339af7`
- GitHub-hosted Windows/X64
- IPC `127.0.0.1:39731`
- all 14 checks true
- audit chain valid with 25 events
- cleanup fully true

PR #85 merged with expected-head protection as:

`2f46665d3da58b8537ffc34288348b7fcd744d90`

Legacy automatic PKG-01 npm-graph/fresh-checkout failures were recorded as unrelated to the frozen 02.17 contract and did not redefine its acceptance.

## 5. Active task — frozen canonical acceptance

`02.18 — Bounded direct terminal execution inside an allowed workspace, including timeout/output limits and invalid-command handling.`

This wording comes directly from the frozen 27-task master execution plan. Do not broaden it silently.

Package guardrails remain:

- execution/mutation remains behind authenticated `vsn-agent` / Core authorization;
- cwd/workspace containment must fail closed;
- timeout and output handling must remain bounded;
- invalid/missing commands must fail cleanly;
- unsupported input must not be guessed;
- installer/updater/release/security/resilience/pentest work remains later packages;
- `02.19+` must not be counted or implemented as active work before 02.18 genuine acceptance/integration.

## 6. Fresh 02.18 working branch

Fresh branch created directly from canonical main `2f46665d3da58b8537ffc34288348b7fcd744d90`:

`pkg02/0218-bounded-terminal-main-sync`

The first commit on this branch only reconciles this handoff after the 02.17 merge. It does not implement 02.18 behavior.

## 7. Relevant open preparation PR

PR #51 — `PKG-02: prepare 02.18 bounded direct terminal execution`

Live state:

- OPEN
- DRAFT
- mergeable
- stale base `pkg02/0217-resumable-binary-transfer` @ `d3b6c711f10c8291a99ab0d74e00918e7d1038c1`
- head `pkg02/0218-bounded-direct-terminal` @ `efe9ad904d883366145c90c2990a2c6edd370592`
- 3 commits, 2 changed files, 216 additions

PR #51 is historical preparation only. It is **not** the implementation baseline and must not be merged/count as 02.18 acceptance.

Its body is useful only as an audit lead. It reports two possible current-path defects that must be independently verified against current canonical main:

1. bounded stdout/stderr capture may stop draining a pipe after the retention cap, potentially causing child broken-pipe/backpressure and changing child exit semantics;
2. independent 512 KiB stdout and stderr retention caps can approach the 1 MiB IPC frame limit before JSON/envelope/authentication overhead, potentially making a nominally bounded result unrepresentable.

Treat those as unverified leads until current-main source and tests prove them.

## 8. Exact next actions

1. Re-read canonical main and confirm it remains `2f46665d3da58b8537ffc34288348b7fcd744d90`, PKG-02 `17/27`, active `02.18`.
2. Inspect current-main direct terminal execution implementation, Core/Agent/CLI wiring, workspace/cwd containment, timeout behavior, stdout/stderr capture, process termination semantics, response/frame bounds, and invalid-command handling.
3. Inspect PR #51 patches only as historical preparation and compare every proposed change against current main.
4. Identify the smallest real gap relative to the frozen 02.18 wording; do not import extra requirements merely because PR #51 mentions them.
5. Add focused tests that prove any real defect before/with the fix.
6. Implement only required 02.18 fixes; no 02.19 persistent-session work.
7. Run format, strict task-scope Clippy/tests, locked release Agent/CLI build, and a fresh exact-head GitHub-hosted Windows 02.18 certification with required regressions.
8. Do not mark 02.18 DONE or advance to `18/27` without real exact-head acceptance evidence.
9. After genuine acceptance, update machine-readable state and this handoff, then re-certify the final state head before requesting merge authorization.

## 9. Activity log

### 2026-08-24 — 02.17 final acceptance and integration

- Re-verified PR #85 exact head `efdd65b3680cdcdb2c02c02a59a8c0f113339af7` unchanged, mergeable, with no unresolved review threads.
- Re-verified required final-head gates green.
- Merged PR #85 with expected-head protection.
- Merge commit: `2f46665d3da58b8537ffc34288348b7fcd744d90`.
- Re-read canonical machine-readable state: PKG-02 `17/27 = 62.96%`, active `02.18`.
- Verified the release candidate remains unchanged.
- Detected that the merged handoff still described pre-merge `main`/PR #85 state.
- Stopped 02.18 product implementation and created fresh branch `pkg02/0218-bounded-terminal-main-sync` directly from canonical main.
- Reconciled this handoff before any 02.18 product change.
