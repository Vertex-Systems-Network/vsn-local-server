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
6. Query live GitHub for canonical `main`, the active acceptance PR/branch, relevant preparation PRs, review blockers, and exact-head workflow runs.
7. Work only on the current canonical task. Future tasks may be inspected only as audit leads; do not implement or count them before prerequisites are integrated.
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

`c1924b0ad109bebd07b18228a56c275b281abc36`

That signed/verified merge commit integrated PR #88 (`PKG-02: certify 02.20 PTY ConPTY lifecycle`).

Canonical machine-readable state remains, until PR #89 merges:

- active package: `PKG-02 — Usable Local Server Beta`
- progress: `20/27 = 74.07%`
- `02.01` through `02.20`: `DONE`
- canonical active task: `02.21 — Read-only local preview fetch`
- `02.22+`: blocked by the frozen sequential task order
- package complete: `false`

Current release candidate remains unchanged:

- candidate ID: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`
- product version: `0.38.1`
- profile: `release-inputs-v1`

## 4. Integrated 02.20 acceptance

Frozen wording:

`02.20 — Interactive PTY/ConPTY session lifecycle: start/write/read-wait/resize/status/stop/remove plus bounded scrollback/recovery behavior.`

Final accepted PR head:

`3c7c5391f3e6f524be435a99ed9ec7eef2d92b1f`

Exact-head GitHub-hosted Windows acceptance:

- run `32716847684`
- job `97399839692`
- artifact `9516688715`
- artifact digest `sha256:d005f6420e5b01c7e69155cdda4d834b6f09ba02147c7b984ae8aaf279048e0b`
- independently verified `evidence.json` digest `sha256:180a76fd5b5b1d9a0a4c58fbc5123f9824f9fed216c172ebd995784e63db9b11`
- runner: GitHub-hosted Windows/X64
- audit chain: valid, 41 events
- cleanup: complete

Required final-head regressions all passed: Repository Governance, PKG-02 Acceptance Sequence, 02.02, 02.08, 02.16, 02.17, 02.18, 02.19 and 02.20.

PR #88 merged into canonical `main` as `c1924b0ad109bebd07b18228a56c275b281abc36`.

## 5. Accepted 02.21 on PR #89 branch

Frozen wording:

`02.21 — Read-only local preview fetch against loopback development servers with bounded response handling.`

Working PR/branch:

- PR #89 — `PKG-02: certify 02.21 loopback preview fetch`
- branch `pkg02/0221-loopback-preview-main-sync`
- base `c1924b0ad109bebd07b18228a56c275b281abc36`
- no 02.22 product implementation is included

Accepted behavioral/state-preprojection source:

`21611270a84edd9dc9eca1b7170dde3cef2f8002`

Exact-head GitHub-hosted Windows acceptance:

- run `32727078448`
- job `97430673636`
- artifact `9520409018`
- artifact digest `sha256:e06070b21b9945f3bf3003f354002c5af3ca2a9fa0564e9b236ce976f9e4d0dd`
- independently verified `evidence.json` digest `sha256:3f74066e6517ca69f50c91ec801bf3fc62f9c82d8195a430684e9046c8e6588c`
- runner: GitHub-hosted Windows/X64
- audit chain: valid, 10 events
- cleanup: complete

Acceptance proved small text and binary responses, 480 KiB frame-safe textual response handling, exact 512 KiB truncation for unknown-length oversized responses, known-length oversize rejection, external-target rejection, redirect non-following and a bounded ~6.05 second slow response through authenticated IPC.

Same-head Repository Governance, PKG-02 Acceptance Sequence, 02.02 Authenticated IPC, 02.08 Windows GitHub-Hosted, 02.16 Workspace Text Files, 02.17 Resumable Binary Transfer, 02.18 Bounded Direct Terminal, 02.19 Persistent Pipe Terminal, 02.20 PTY/ConPTY Lifecycle and 02.21 Loopback Preview Fetch all passed.

The acceptance run also verified the corrected PR-event evidence binding: evidence records the exact checked-out PR head rather than the synthetic pull-request merge SHA.

## 6. Branch state projection

After genuine 02.21 acceptance, the PR #89 branch projects:

- PKG-02 progress: `21/27 = 77.78%`
- `02.01` through `02.21`: `DONE`
- projected active task: `02.22 — Advanced local preview requests: allowed HTTP methods, bounded request/response bodies and filtered headers; loopback-only mutation boundary.`
- `02.23+`: `BLOCKED`
- package complete: `false`

This projection is not canonical until PR #89 is merged. Canonical `main` remains 20/27 active 02.21 while its HEAD remains `c1924b0ad109bebd07b18228a56c275b281abc36`.

## 7. 02.21 implementation details worth preserving

The accepted change keeps direct preview targeting at `http://127.0.0.1:<port>`, redirects disabled and direct GET/HEAD semantics bounded by a 512 KiB raw response limit.

Authenticated IPC changes are scoped to the transport boundary:

- `preview.fetch` client response timeout is 15 seconds so the direct preview's bounded HTTP response window fits inside authenticated IPC;
- the global authenticated IPC frame remains 1 MiB;
- large `preview.fetch` responses drop only the duplicate textual convenience copy when necessary, preserving authoritative `body_base64`, status, content type and truncation metadata;
- outbound authenticated responses fail closed if the encoded response frame exceeds the 1 MiB bound;
- non-preview commands are not compacted by this behavior.

Old PR #55 remains preparation-only and must not be merged or treated as acceptance evidence.

## 8. Exact continuation point

1. Re-read live PR #89 and canonical `main` before acting.
2. Confirm the branch tracker, execution status, README, master plan and this handoff consistently project `21/27 = 77.78%`, 02.21 DONE and 02.22 active, while explicitly preserving that canonical main is still 20/27 until merge.
3. Run/observe the fresh exact-head final projection gates on the final PR #89 head. At minimum require Repository Governance, PKG-02 Acceptance Sequence, 02.02, 02.08, 02.16, 02.17, 02.18, 02.19, 02.20 and 02.21 on the same final head.
4. Inspect any required-gate failure; do not silently redefine acceptance around unrelated legacy failures.
5. Update PR #89 body with the final head and accepted evidence, then mark it Ready for Review only if the final exact-head gate set is genuinely green.
6. Merge PR #89 only with explicit merge authorization and expected-head protection.
7. After merge, re-read canonical `main` and machine-readable state. Do not implement 02.22 before that reconciliation.
8. Only after 02.21 is integrated may a fresh 02.22 branch be created and the frozen 02.22 acceptance task be implemented.

## 9. Activity log

### 2026-08-24 — 02.20 integration and 02.21 activation

- PR #88 final head `3c7c5391f3e6f524be435a99ed9ec7eef2d92b1f` passed all nine required exact-head gates.
- Final artifact `9516688715` was independently verified.
- PR #88 was explicitly authorized and merged as signed/verified canonical commit `c1924b0ad109bebd07b18228a56c275b281abc36`.
- Canonical machine state was re-read and confirmed `20/27 = 74.07%`, active `02.21`.
- Fresh branch `pkg02/0221-loopback-preview-main-sync` was created from integrated main.
- Old stacked PR #55 is preparation-only and not accepted evidence.

### 2026-08-24 — 02.21 acceptance and state projection

- Direct preview/IPС transport audit found the duplicate text/base64 representation could exceed the authenticated 1 MiB frame and that the ordinary 5-second command timeout was shorter than the bounded direct-preview HTTP window.
- Scoped transport fixes and a GitHub-hosted Windows 02.21 harness were added without implementing 02.22.
- The first strict harness attempt exposed only an unrelated transitive `vsn-network` Clippy lint; strict Clippy scope was narrowed to the changed preview/IPC crates without changing product acceptance.
- A later successful behavioral run initially failed only because evidence used GitHub's PR synthetic `GITHUB_SHA`; source metadata binding was corrected to the exact checked-out PR head.
- Corrected run `32727078448`, job `97430673636` passed certification, evidence binding and cleanup.
- Artifact `9520409018` and its internal evidence digest were independently verified.
- Branch state is projected to `21/27 = 77.78%`, 02.21 DONE, 02.22 active pending final projected-head recertification and merge.