# PKG-02 02.24 Development Preflight

Feature ID: `pkg02-0224-domain-https`  
Version: `1.0.0`  
Prepared: `2026-08-25`

## Frozen contract

- plan: `.ai/plans/pkg02-0224-domain-https-v1.md`
- approval reference: `docs/MASTER-EXECUTION-PLAN.md — frozen PKG-02 task 02.24`
- canonical base: `265bd17895231fc145ccd435c48def0a38bfd98d`
- active package/task: `PKG-02 / 02.24`
- canonical progress: `23/27 = 85.19%`
- product/candidate: `0.38.1 / c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`

Plan SHA-256 is bound by the feature manifest and must be recomputed/verified immediately before any product mutation.

## Canonical-state comparison

Live canonical `main`, `docs/MASTER-EXECUTION-STATUS.json`, `certification/pkg02-usable-local-beta-v1.json`, README, and the reconciled `docs/MASTER-EXECUTION-PLAN.md` agree that 02.23 is integrated DONE and 02.24 is active. The stale master-plan blocker was reconciled through PR #95 before this feature branch was created.

Open PR #58 is historical preparation only and is not acceptance authority.

Result: `match`

## Completed prerequisite stages

- Research: current source + official-source review complete.
- Plan: frozen task mapped to AC-01..AC-12.
- Architecture: existing component seams and replacement/trust boundaries reviewed.
- Data Flow: IPC, hosts-file, Caddy subprocess, persistence and cleanup paths reviewed.
- Security: NetworkManage/elevation, destructive-file, implicit-trust and evidence risks reviewed.
- Design: existing CLI/elevated operator surfaces retained; no new UI.
- QA: positive/negative/exact-source evidence map defined.
- Performance: bounded agent/hosts/helper/evidence budgets defined.

Artifact digests are recorded in `.ai/manifests/pkg02-0224-domain-https.v1.json`.

## Research freshness

- baseline review: `2026-08-25`
- delta review: `2026-08-25`
- market/tooling delta: `none`
- official sources: IANA/RFC 6761, Rust std fs, Microsoft ReplaceFileW/TOKEN_ELEVATION, Caddy TLS/global options, mkcert README
- external content treated as reference data only.

## Expected product files

Primary:

- `crates/vsn-network/src/lib.rs`
- `crates/vsn-network/tests/pkg02_hosts_safety.rs`
- `crates/vsn-core/tests/pkg02_domain_policy.rs`
- `scripts/self-hosted/pkg02-0224.ps1`
- `.github/workflows/pkg02-0224-domain-https.yml`

Conditional only if a mapped AC requires a Windows elevation implementation correction:

- `apps/agent/src/main.rs`
- `apps/agent/Cargo.toml`
- `Cargo.lock`

No 02.25+ files.

## Allowed tools / network / privilege class

Allowed development:
- repository code edits;
- Rust/Cargo build, fmt, Clippy and tests;
- GitHub-hosted Windows certification;
- loopback IPC;
- disposable sandbox files;
- deterministic fake Caddy helper;
- read-only official documentation research.

Normal certification privilege:
- non-elevated/user-level product execution;
- ordinary authenticated IPC;
- read-only system-host existence/hash check where permitted.

## Actions requiring separate explicit operator approval

Not approved by this plan alone:

- mutate real OS hosts file;
- apply/remove NRPT or OS resolver state;
- install/remove CA roots in system/Firefox/Java trust stores;
- execute `mkcert -install`, `caddy trust`, or equivalent;
- any other OS-global privileged network mutation.

Normal 02.24 acceptance must pass without these actions and must record `privileged_system_mutation_performed=false`.

## Acceptance commands

- `cargo fmt --all -- --check`
- `cargo clippy --locked --package vsn-network --package vsn-core --package vsn-policy --package vsn-agent --all-targets --no-deps -- -D warnings`
- `cargo test --locked --package vsn-network --package vsn-core --package vsn-policy`
- `cargo build --locked --release --package vsn-agent --package vsn`
- `pwsh -NoProfile -File scripts/self-hosted/pkg02-0224.ps1`
- `git diff --check`

## Required regressions

AI Planning Governance; Repository Governance; PKG-02 Acceptance Sequence; 02.02; 02.08; 02.14; 02.16; 02.17; 02.18; 02.19; 02.20; 02.21; 02.22; 02.23; and the new 02.24 workflow on the final exact head.

## Mutation gate

Development may start only after:

1. the manifest records the exact SHA-256 of the frozen plan and every lifecycle artifact;
2. the planning commit is based on canonical `265bd17895231fc145ccd435c48def0a38bfd98d`;
3. AI Planning Governance, Repository Governance, and PKG-02 Acceptance Sequence pass on that planning head;
4. live canonical `main` is refreshed immediately before the first product mutation and has not moved to a contradictory task/state.

If any condition fails, stop and reconcile.
