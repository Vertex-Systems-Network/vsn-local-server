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

Canonical machine-readable state is:

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

## 5. Frozen active task

`02.21 — Read-only local preview fetch against loopback development servers with bounded response handling.`

Task dependency:

- `02.20` is integrated and DONE, so 02.21 is now legitimately active.
- `02.22+` remain blocked until 02.21 is genuinely accepted and integrated.

PKG-02 guardrails remain unchanged:

- local beta workflows only;
- `vsn-agent` remains the execution/mutation boundary;
- loopback-only preview behavior must fail closed for external targets;
- response handling must be bounded and representable through authenticated local IPC;
- redirects must not silently escape loopback;
- unsupported inputs are not guessed;
- installer/updater/release/security/resilience/pentest work remains in later packages.

## 6. Current 02.21 working branch

Fresh branch from integrated main:

`pkg02/0221-loopback-preview-main-sync`

Base:

`c1924b0ad109bebd07b18228a56c275b281abc36`

An older stacked preparation PR exists:

- PR #55 — `PKG-02: prepare 02.21 loopback preview fetch`
- old base: pre-02.20 preparation branch
- old head: `3f84d98cd2e936072ede5f267365367f9a17b002`
- changed files are only an old 02.21 workflow and PowerShell harness
- treat it as an audit lead only; do not merge or use it as acceptance evidence

The old preparation audit identified a likely IPC-frame defect in `PreviewResponse`: textual direct-preview responses carry both a full `body_base64` and a duplicate full `text` string, so a near-limit text response can exceed the authenticated 1 MiB IPC frame even when raw response bytes are within the direct-preview 512 KiB body bound. Re-audit this against current integrated source before changing product code.

## 7. Exact continuation point

1. Finish post-02.20 narrative reconciliation on the fresh 02.21 branch so branch state consistently says canonical `20/27`, 02.20 DONE/integrated, 02.21 active.
2. Re-read current `crates/vsn-preview`, Agent/CLI routing and authenticated IPC frame limits from integrated source.
3. Re-audit the old PR #55 harness against current source; copy only still-valid acceptance intent, not stale branch assumptions.
4. Implement only 02.21-scoped fixes required by the frozen acceptance wording.
5. Add a fresh exact-head GitHub-hosted Windows 02.21 certification workflow/harness with source binding, bounded loopback fixture behavior, audit validity and cleanup evidence.
6. Require current-head regressions appropriate to the active task before projecting 02.21 DONE.
7. Do not implement 02.22.
8. Do not merge the eventual 02.21 acceptance PR without explicit merge authorization.

## 8. Activity log

### 2026-08-24 — 02.20 integration and 02.21 activation

- PR #88 final head `3c7c5391f3e6f524be435a99ed9ec7eef2d92b1f` passed all nine required exact-head gates.
- Final artifact `9516688715` was independently verified.
- PR #88 was explicitly authorized and merged as signed/verified canonical commit `c1924b0ad109bebd07b18228a56c275b281abc36`.
- Canonical machine state was re-read and confirmed `20/27 = 74.07%`, active `02.21`.
- Fresh branch `pkg02/0221-loopback-preview-main-sync` was created from integrated main.
- Old stacked PR #55 is preparation-only and not accepted evidence.