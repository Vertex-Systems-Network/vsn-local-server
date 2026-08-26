# PKG-02 02.27 Lifecycle Review

Feature: `pkg02-0227-fresh-state-local-beta-final-gate`
Reviewed: 2026-08-26

## Architecture

Retain the accepted architecture: `vsn-agent` is the authenticated mutation/execution boundary; CLI and Desktop are controllers. 02.27 adds certification orchestration, not a new product subsystem.

The final acceptance architecture has two layers:
1. a dedicated fresh-state integrated GitHub-hosted Windows gate;
2. the frozen same-head hardened predecessor regression matrix.

## Data flow

1. Bind exact source/base/plan/product/candidate/runner.
2. Parse canonical PKG-02 tracker and prove exactly 26 predecessor tasks DONE.
3. Capture clean git status and SHA-256 of bound lock/state files.
4. Run locked Rust verification and release Agent/CLI build.
5. Run locked Desktop install/build.
6. Isolate VSN test state, start release Agent, exercise authenticated CLI core path.
7. Exercise current Desktop/Tauri authenticated bridge and Overview online/offline behavior.
8. Verify audit chain and capture binary/evidence hashes.
9. Stop Agent, restore test state, remove disposable fixtures.
10. Recompute bound hashes and repository status; fail on any drift.
11. Upload evidence.
12. Final acceptance additionally requires every frozen same-head regression to be SUCCESS.

## Security

- GitHub-hosted Windows only; self-hosted is not accepted for the final integrated gate.
- No new permission is introduced.
- No privileged system mutation is needed by the integrated gate.
- No production secret or remote production/database mutation is used.
- Existing task-specific dangerous/privileged cases remain inside their already-sandboxed hardened regressions.
- Unknown/malformed prerequisite/evidence/state conditions fail closed.
- Any product fix discovered by the gate is minimal, AC-mapped and forces a new exact-head matrix.

## Design

Initial implementation is limited to:
- `scripts/self-hosted/pkg02-0227.ps1`;
- `.github/workflows/pkg02-0227-fresh-state-final-gate.yml`.

The stale PR #61 UI/workflow shape may inform ergonomics, but its base and acceptance coverage are not reused as authority.

## QA

Frozen AC-01..AC-12 cover:
- exact source/runner/toolchain;
- predecessor chain;
- clean checkout;
- Rust quality/build;
- Desktop locked build;
- authenticated CLI;
- Desktop bridge;
- accepted capability breadth through same-head regressions;
- fail-closed/permission preservation;
- cleanup/non-mutation;
- evidence integrity;
- final zero-drift + regression matrix.

## Performance

02.27 introduces no new product performance budget. Existing accepted resource/time/output limits remain authoritative in their hardened regressions.

The dedicated final workflow may use up to 120 minutes, but must not hide unbounded child/process waits. Agent readiness and any integrated smoke waits are explicitly bounded. Build commands retain normal CI timeout behavior.

## Decision

Lifecycle stages research, plan, architecture, data_flow, security, design, qa and performance are complete for planning. Development is ready only after the planning commit passes AI Planning Governance, Repository Governance and PKG-02 Acceptance Sequence on the exact planning head.
