# PKG-02 02.27 Frozen Plan — Fresh-State Local Beta Final Gate

Feature ID: `pkg02-0227-fresh-state-local-beta-final-gate`
Version: `1.0.0`
Canonical base SHA: `e6e981f106ff3685ab1694261991e5e97a3b738d`
Approval reference: `docs/MASTER-EXECUTION-PLAN.md — frozen PKG-02 task 02.27`
Approved date: `2026-08-26`

## Outcome

Genuinely certify:

`02.27 — Fresh-state local beta final gate: CLI + Desktop end-to-end smoke over all accepted local capabilities, zero unintended file/lock drift, and evidence that tasks 02.01–02.26 are DONE.`

This is the final PKG-02 acceptance gate. It does not itself mark PKG-02 complete; completion state is projected separately only after genuine exact-head acceptance and merge.

## Canonical entry state

- canonical `main`: `e6e981f106ff3685ab1694261991e5e97a3b738d`;
- PKG-02 `26/27 = 96.30%`, `complete=false`, active task `02.27`;
- tasks `02.01` through `02.26` are integrated `DONE`;
- product version `0.38.1`;
- release candidate `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`;
- Rust/cargo exact `1.97.1`;
- authenticated Agent IPC `127.0.0.1:39731`;
- stale PR #61 is research input only and is not an implementation baseline or acceptance authority.

## Market-delta decision

Current local-development products emphasize multi-service local environments, shareable/version-controlled configuration, and reproducible setup. Current GitHub Actions guidance distinguishes GitHub-hosted fresh virtual machines from persistent self-hosted runners that do not guarantee a clean instance per job.

Decision: `certification_reproducibility_only_no_roadmap_expansion`.

No competitor-inspired product feature is added in 02.27. The delta only reinforces fresh-state, exact-source, clean-runner, end-to-end and no-drift evidence already required by the frozen task.

Reviewed official sources:
- https://herd.laravel.com/windows
- https://docs.ddev.com/en/stable/
- https://docs.ddev.com/en/stable/users/configuration/config/
- https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- https://docs.github.com/en/actions/concepts/runners/self-hosted-runners
- https://docs.github.com/en/actions/reference/security/secure-use

## Acceptance architecture

02.27 acceptance is the conjunction of two evidence layers on one exact final source SHA:

1. **Dedicated fresh-state integrated gate**
   - GitHub-hosted Windows/X64 only;
   - exact source, product, candidate, tracker and plan binding;
   - clean checkout before execution;
   - locked Rust full-workspace format/Clippy/tests;
   - release Agent/CLI build;
   - locked Desktop install/build;
   - real authenticated Agent/CLI core smoke;
   - real Desktop authenticated bridge/Overview online/offline smoke using the accepted 02.04 architecture;
   - prerequisite evidence-chain verification;
   - audit, cleanup, binary/evidence hashes and zero repository/lock drift.

2. **Same-head hardened capability regressions**
   - reuse the existing dedicated acceptance workflows for the accepted high-risk capability families rather than copying or weakening their harnesses;
   - all frozen required regressions below must be green on the exact same final source SHA.

The dedicated gate must not claim that `vsn version`/`vsn help` alone prove all local capabilities. Capability breadth is established by the dedicated integrated smoke plus the exact-head hardened regression matrix.

## In scope

- a fresh 02.27 GitHub-hosted Windows final-gate workflow;
- a fresh 02.27 certification/orchestration harness;
- exact validation that 02.01–02.26 are each present exactly once and `DONE`;
- validation that PKG-02 is exactly 26/27, active 02.27, incomplete;
- locked Rust 1.97.1 full-workspace quality/build verification;
- locked Desktop dependency install and production build without lock drift;
- authenticated Agent/CLI smoke for status, machine, security, config, audit and workspace registration;
- Desktop authenticated bridge/Overview smoke for Agent online and unavailable/offline behavior;
- evidence-chain binding for all 26 accepted predecessor tasks;
- same-head regression of the existing accepted local capability families;
- exact pre/post repository status and tracked lockfile hashes proving zero unintended drift;
- evidence and binary hashing, cleanup and non-mutation proof;
- bug fixes only when a failing 02.27 AC proves a concrete defect in the current accepted product.

## Explicit non-goals

- no PKG-03 installer/signing work;
- no updater/recovery expansion;
- no Linux/macOS packaging;
- no Remote Control Plane production acceptance;
- no new runtime/database/container/service/file/terminal/preview/domain capability;
- no new permissions or widening of existing permission sets;
- no privileged system mutation;
- no production or remote database mutation;
- no hosts/resolver/trust-store mutation outside existing separately sandboxed regression gates;
- no Desktop redesign;
- no roadmap denominator/order change;
- no product-version or release-candidate change;
- no marking PKG-02 complete inside the implementation PR.

## Security and isolation constraints

- final integrated gate runs on `windows-latest` and requires `RUNNER_ENVIRONMENT=github-hosted`;
- checkout is bound to the exact expected final SHA;
- IPC port 39731 must be free before Agent launch;
- Agent authentication/authorization boundaries remain unchanged;
- all temporary workspace/data fixtures are disposable and contained under runner temp or VSN-owned test data;
- no production secret is used;
- no privileged operation is performed by the integrated 02.27 harness;
- all system state touched for Agent test isolation is backed up/restored exactly as in accepted prior gates;
- failure at any prerequisite/evidence/source/runner/cleanup invariant is fail-closed.

## Fresh-state and drift contract

Before any build/smoke:
- `git status --porcelain=v1 --untracked-files=all` must be empty;
- `Cargo.lock`, `apps/desktop/package-lock.json`, canonical tracker/status/plan, and other explicitly bound lock/state files are hashed.

After all build/smoke/cleanup:
- repository status must equal the clean baseline;
- all bound tracked-file hashes must be byte-identical;
- generated evidence must live only in the ignored certification artifact path;
- no unexpected tracked/untracked file may remain.

A known unrelated workflow failure is not silently imported into this contract. 02.27 proves its own fresh-state/lock-drift invariants directly.

## Acceptance criteria

- `AC-01 Exact source/runner/toolchain binding`: GitHub-hosted Windows/X64 proves checkout source equals expected final SHA; Rust/cargo are 1.97.1; evidence binds canonical base, feature/plan IDs, plan digest, product `0.38.1`, candidate `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474` and IPC.
- `AC-02 Canonical prerequisite chain`: tracker is PKG-02, required=27, done=26, percent=96.30, complete=false, active_task=02.27; exactly one task entry exists for each 02.01–02.27; 02.01–02.26 are DONE with non-empty accepted evidence where the tracker schema carries evidence; 02.27 alone is IN_PROGRESS.
- `AC-03 Fresh checkout and tracked-state baseline`: repository begins clean; bound lock/state files are hashed before execution; no stale PR #61 source/base is treated as authority.
- `AC-04 Locked Rust product verification`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and locked release Agent/CLI build pass on the exact source.
- `AC-05 Locked Desktop verification`: `apps/desktop/package-lock.json` exists; `npm ci` and production build pass; package-lock hash is unchanged; the gate does not regenerate or rewrite dependency locks.
- `AC-06 Authenticated Agent/CLI integrated smoke`: release Agent starts on the disposable test state and the release CLI proves ping/status/machine/security/config/audit plus workspace add/list/remove through authenticated IPC, including Agent-unavailable fail-closed behavior.
- `AC-07 Desktop integrated smoke`: the current Desktop production/Tauri path proves authenticated status/machine bridge and deterministic online/offline/partial Overview behavior using the accepted 02.04 architecture; a static frontend build alone is insufficient.
- `AC-08 Accepted local capability breadth`: the exact final source also passes the frozen hardened regression matrix for bootstrap, diagnostics, files, binary transfer, direct/persistent/PTY terminals, preview, DNS, domain/HTTPS boundary, SQLite and external/native database adapters. These same-head regressions, together with AC-06/AC-07, constitute the required end-to-end smoke over accepted local capabilities without duplicating weaker copies of their harnesses.
- `AC-09 Fail-closed and permission preservation`: unknown providers/capabilities remain fail-closed; no permission widening, no DatabaseDestructive grant, no unsafe privilege fallback, and no production/remote mutation path is introduced by 02.27.
- `AC-10 Cleanup and non-mutation`: Agent/process fixtures stop; IPC key/LOCALAPPDATA/test workspace state are restored; temporary fixtures are removed; no privileged system, hosts, resolver, trust-store, production, or remote-database mutation is performed by the integrated gate.
- `AC-11 Evidence integrity`: evidence records exact source/base/feature/plan/product/candidate/runner/toolchain, prerequisite count, integrated CLI/Desktop results, regression gate names, clean-state hashes, Agent/CLI hashes, audit result, cleanup/non-mutation flags and evidence SHA-256; artifact contents are independently recomputable.
- `AC-12 Zero unintended repository/lock drift and final matrix`: post-cleanup repository status is byte-for-byte equivalent to the clean baseline, bound lock/state hashes match, `git diff --check` is clean, and every required regression including the dedicated 02.27 gate is SUCCESS on the exact final head.

## Planned implementation/certification files

Primary planned files:
- `scripts/self-hosted/pkg02-0227.ps1`
- `.github/workflows/pkg02-0227-fresh-state-final-gate.yml`

Planning/lifecycle files are frozen separately before implementation.

No product file is planned. If an AC exposes a real product defect, only the minimum AC-mapped product/test change may be added after the failure is recorded. Such a change invalidates all prior exact-head results and requires a fresh final matrix.

The stale PR #61 files may be consulted for tracker/build ideas, but its branch/base is not reused and its minimal `version/help` smoke is not accepted as the final contract.

## Required commands

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- `cargo build --locked --release --package vsn-agent --package vsn`
- `npm ci` in `apps/desktop`
- `npm run build` in `apps/desktop`
- `pwsh -NoProfile -File scripts/self-hosted/pkg02-0227.ps1`
- `git diff --check`

## Required final exact-head regressions

1. `AI Planning Governance`
2. `Repository Governance`
3. `PKG-02 Acceptance Sequence`
4. `PKG-02 02.02 Authenticated IPC`
5. `PKG-02 02.08 Windows GitHub-Hosted Certification`
6. `PKG-02 02.14 Local Diagnostics`
7. `PKG-02 02.16 Workspace Text Files`
8. `PKG-02 02.17 Resumable Binary Workspace Transfer`
9. `PKG-02 02.18 Bounded Direct Terminal Execution`
10. `PKG-02 02.19 Persistent Pipe Terminal Sessions`
11. `PKG-02 02.20 PTY ConPTY Lifecycle`
12. `PKG-02 02.21 Loopback Preview Fetch`
13. `PKG-02 02.22 Advanced Preview Requests`
14. `PKG-02 02.23 .test DNS Responder`
15. `PKG-02 02.24 Local Domain/HTTPS Boundary`
16. `PKG-02 02.25 SQLite Database Studio`
17. `PKG-02 02.26 External/Native Database Adapters`
18. `PKG-02 02.27 Fresh-State Local Beta Final Gate`

Unrelated PKG-01 npm-graph/fresh-checkout workflows are not silently added to this frozen set. Any failure that maps directly to AC-03/AC-05/AC-12 is still a 02.27 blocker through the dedicated gate itself.

## Evidence artifact

Artifact name:
`pkg02-0227-fresh-state-local-beta-final-gate`

Expected path:
`dist-self-hosted/02.27`

Expected contents include:
- `evidence.json` and `evidence.json.sha256`;
- exact source/base/feature/plan/product/candidate/runner/toolchain binding;
- prerequisite task/evidence-chain summary for 02.01–02.26;
- pre/post repository status and bound-file SHA-256 values;
- Rust command logs;
- Desktop install/build logs and lock hash;
- authenticated CLI core smoke outputs;
- Desktop bridge/Overview smoke outputs;
- audit verification;
- Agent/CLI SHA-256;
- cleanup/non-mutation JSON;
- required regression-name manifest.

The workflow artifact ZIP digest and `evidence.json` digest must be independently recomputed before final acceptance.

## Rollout / completion

After genuine exact-head 02.27 acceptance:
1. merge only the accepted implementation/certification PR;
2. re-read live canonical `main`;
3. create a separate state-only projection;
4. only that state projection may set PKG-02 `27/27 = 100%`, `status=COMPLETE`, `complete=true`;
5. the next active package/task is derived from live canonical plan/state at that time and must not be guessed in advance.

## Rollback

Before canonical projection, rollback is PR closure/revert. The 02.27 integrated harness uses disposable local fixtures and must leave no system/production mutation. Any product bug fix can be reverted independently from the certification scaffolding.

## Change control

This plan is frozen by SHA-256 in the feature manifest. Do not edit it in place after the manifest records its digest. Material scope, permission, acceptance, runner, evidence, resource or completion-rule changes require an approved addendum or a new plan version.
