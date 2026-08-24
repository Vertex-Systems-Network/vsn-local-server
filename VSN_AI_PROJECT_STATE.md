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
10. Do not merge an acceptance PR unless merge authorization exists; when authorized, still require exact-head acceptance and expected-head protection.

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

`98702614949afda4f5aa08ad032f01fd6613b3ef`

That signed/verified merge commit integrated PR #89 (`PKG-02: certify 02.21 loopback preview fetch`).

Canonical machine-readable state remains, until PR #90 merges:

- active package: `PKG-02 — Usable Local Server Beta`
- progress: `21/27 = 77.78%`
- `02.01` through `02.21`: `DONE`
- canonical active task: `02.22 — Advanced local preview requests: allowed HTTP methods, bounded request/response bodies and filtered headers; loopback-only mutation boundary.`
- `02.23+`: blocked by the frozen sequential task order
- package complete: `false`

Current release candidate remains unchanged:

- candidate ID: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`
- product version: `0.38.1`
- profile: `release-inputs-v1`

## 4. Integrated 02.21 acceptance

PR #89 was accepted and merged into canonical `main` as `98702614949afda4f5aa08ad032f01fd6613b3ef`.

The integrated capability preserves loopback-only direct preview targeting, redirects disabled, GET/HEAD semantics, bounded response handling, authenticated IPC response-frame enforcement and frame-safe duplicate-text suppression.

## 5. Accepted 02.22 on PR #90 branch

Frozen wording:

`02.22 — Advanced local preview requests: allowed HTTP methods, bounded request/response bodies and filtered headers; loopback-only mutation boundary.`

Working PR/branch:

- PR #90 — `PKG-02: certify 02.22 advanced preview requests`
- branch `pkg02/0222-advanced-preview-main-sync`
- base `98702614949afda4f5aa08ad032f01fd6613b3ef`
- no 02.23 product implementation is included

Accepted behavioral/state-preprojection source:

`0771aeb42cf5db7186dc76c501b75c63f48b7d07`

Exact-head GitHub-hosted Windows acceptance:

- run `32736810594`
- job `97461479774`
- artifact `9523954059`
- artifact digest `sha256:7821029e3229aaf4b6345e02dfc2271f3aaeb4b08db338fbdb2088e276a81bd5`
- independently verified `evidence.json` digest `sha256:bd581b21da39d58e2f3930618d1b2a178d71bb2be115cf87516d9e39ef2762d7`
- runner: GitHub-hosted Windows/X64
- audit chain: valid, 9 events
- cleanup: complete

Acceptance proved:

- allowed advanced preview methods including POST and DELETE are preserved;
- a 409600-byte POST body and approved `x-vsn-*` header round-trip correctly;
- disallowed request headers and GET bodies fail closed;
- network-path/external targeting fails closed while actual transport remains loopback-only;
- an advanced-preview request above the 480 KiB frame-safe body budget is rejected before generic IPC frame overflow;
- a 491520-byte large response is truncated/bounded before authenticated response serialization, with duplicate text removed and CLI output 655601 bytes;
- the advanced-preview 20-second HTTP window fits inside a bounded 23-second authenticated IPC response timeout;
- slow ~6.05 second loopback response succeeds within the bounded window;
- audit validity and complete cleanup are verified.

Same-head Repository Governance, PKG-02 Acceptance Sequence, 02.02 Authenticated IPC, 02.08 Windows GitHub-Hosted, 02.16 Workspace Text Files, 02.17 Resumable Binary Transfer, 02.18 Bounded Direct Terminal, 02.19 Persistent Pipe Terminal, 02.20 PTY/ConPTY Lifecycle, 02.21 Loopback Preview Fetch and 02.22 Advanced Preview Requests all passed.

## 6. Branch state projection

After genuine 02.22 acceptance, PR #90 projects:

- PKG-02 progress: `22/27 = 81.48%`
- `02.01` through `02.22`: `DONE`
- projected active task: `02.23 — Local .test DNS responder lifecycle and protocol behavior: plan/start/status/stop, A/AAAA loopback answers and refusal of non-.test names.`
- `02.24+`: `BLOCKED`
- package complete: `false`

This projection is not canonical until PR #90 is merged. Canonical `main` remains `21/27 = 77.78%`, active `02.22`, while its HEAD remains `98702614949afda4f5aa08ad032f01fd6613b3ef`.

## 7. 02.22 implementation details worth preserving

The global authenticated IPC frame remains exactly 1 MiB. The accepted fix does not widen that contract.

For authenticated `preview.request`:

- effective frame-safe request/response body budget is 480 KiB;
- oversized request bodies fail at the preview/IPC validation boundary before writing an impossible frame;
- outgoing request frames are explicitly size-checked;
- large successful responses are bounded before response signing/serialization while preserving authoritative base64 body, status, approved headers and truncation state;
- duplicate textual convenience data is removed when needed for frame safety;
- client response timeout is 23 seconds to contain the advanced preview's bounded 20-second HTTP timeout;
- direct `preview.fetch` behavior accepted in 02.21 remains intact.

Old PR #56 was preparation-only from a stale pre-02.21 stack and was closed as superseded by PR #90. It must not be merged or treated as 02.22 acceptance evidence.

## 8. Exact continuation point

1. Re-read live PR #90 and canonical `main` before acting.
2. Confirm tracker, execution status, README, master plan and this handoff consistently project `22/27 = 81.48%`, 02.22 DONE and 02.23 active while explicitly preserving canonical main at `21/27 = 77.78%`, active 02.22 until merge.
3. Require fresh exact-head final-projection gates on the final PR #90 head: Repository Governance, PKG-02 Acceptance Sequence, 02.02, 02.08, 02.16, 02.17, 02.18, 02.19, 02.20, 02.21 and 02.22.
4. Inspect the final-head 02.22 artifact independently: verify artifact ZIP SHA-256, internal evidence SHA-256, measurements, audit and cleanup.
5. Update PR #90 body with exact final-head evidence and mark Ready for Review only when all required gates are green.
6. Merge PR #90 only with expected-head protection and existing explicit merge authorization.
7. After merge, re-read canonical `main` and machine-readable state. Expected canonical state is `22/27 = 81.48%`, active 02.23.
8. Do not implement 02.23 before that integration/reconciliation is complete.

## 9. Activity log

### 2026-08-24 — 02.21 integration

- PR #89 completed final exact-head certification and was merged as signed/verified `98702614949afda4f5aa08ad032f01fd6613b3ef`.
- Canonical machine state was re-read and confirmed `21/27 = 77.78%`, active 02.22.
- Fresh branch `pkg02/0222-advanced-preview-main-sync` was created from integrated main.

### 2026-08-24 — 02.22 acceptance and state projection

- Fresh-main audit confirmed existing advanced preview methods/header filters/loopback boundary but exposed an impossible 2 MiB body contract over a 1 MiB authenticated IPC frame.
- Scoped IPC-boundary fixes introduced a 480 KiB effective frame-safe body budget, request preflight rejection, bounded response truncation/compaction, explicit outbound request frame checking and a 23-second preview-request response timeout without changing the global IPC frame.
- Exact-head run `32736810594`, job `97461479774` passed certification, evidence binding and cleanup on source `0771aeb42cf5db7186dc76c501b75c63f48b7d07`.
- Artifact `9523954059` and its internal evidence digest were independently verified.
- All eleven required same-head gates passed.
- Branch state is projected to `22/27 = 81.48%`, 02.22 DONE, 02.23 active pending final projected-head recertification and merge.
