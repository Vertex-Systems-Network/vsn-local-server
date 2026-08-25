# PKG-02 02.24 Frozen Plan — Local Domain/HTTPS and Privileged Network Boundary

Feature ID: `pkg02-0224-domain-https`  
Version: `1.0.0`  
Canonical base SHA: `265bd17895231fc145ccd435c48def0a38bfd98d`  
Approval reference: `docs/MASTER-EXECUTION-PLAN.md — frozen PKG-02 task 02.24`  
Approved date: `2026-08-25`

## Outcome

Genuinely certify the frozen task:

`02.24 — Local domain/HTTPS planning and privileged network boundary: domain plan, hosts apply/remove/reload behavior and fail-closed elevation requirements.`

## In scope

- authenticated `.test` domain planning with loopback target and TLS posture;
- explicit proof that ordinary authenticated IPC lacks `NetworkManage`;
- hosts apply/remove semantics using the production implementation against a disposable hosts file;
- preservation of unmanaged hosts content and other VSN-managed entries;
- idempotent apply/remove;
- fail-closed hosts reads/decoding;
- failure-safe destination replacement without pre-deleting the target;
- VSN-generated Caddy configuration that cannot silently install local CA trust;
- bounded Caddy validate-then-reload behavior against a VSN-owned sandbox config and deterministic helper;
- exact-source GitHub-hosted Windows evidence;
- valid audit-chain and cleanup proof;
- minimum bug fixes required when these criteria expose defects.

## Explicit non-goals

- no real `System32\\drivers\\etc\\hosts` or `/etc/hosts` mutation during normal certification;
- no `mkcert -install`, `caddy trust`, Windows certificate-store mutation, Firefox/Java trust-store mutation, or other CA trust installation during normal certification;
- no OS DNS resolver/NRPT apply/remove/status acceptance;
- no port 53 requirement;
- no public listener or external reverse-proxy target;
- no relaxation of `NetworkManage` or remote high-risk policy;
- no Desktop/web UI;
- no installer/updater work;
- no 02.25+ product work;
- no task denominator/order changes.

## Dependencies

- canonical `02.01`–`02.23` integrated DONE;
- canonical PKG-02 state `23/27 = 85.19%`, active `02.24`;
- canonical base `265bd17895231fc145ccd435c48def0a38bfd98d`;
- Rust/cargo exact `1.97.1`;
- authenticated IPC on `127.0.0.1:39731`;
- release candidate `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`, product `0.38.1`.

## User-visible / operator behavior

- `vsn domain plan <name.test> <port>` is non-mutating and returns normalized `.test` domain, `127.0.0.1`, the requested nonzero target port, `tls=true`, conflict information, and `requires_admin_for_hosts_file=true`.
- invalid/non-`.test` domains and port zero fail.
- ordinary `vsn domain apply/remove/reload` requests do not gain high-risk authority; the Agent/Core returns permission denial because `local_authenticated` lacks `NetworkManage`.
- privileged OS network operations remain under `vsn-agent network-admin` and require OS elevation before the `local_network_admin` principal is created.
- VSN-generated Caddy configuration must explicitly disable automatic local-root trust installation.

## Security / privilege constraints

- `NetworkManage` remains high-risk and absent from `Principal::local_authenticated()`.
- `Principal::local_network_admin()` remains a distinct local OS-elevation principal.
- remote delegation of `NetworkManage` remains rejected.
- non-elevated `network-admin` must fail before any mutation call.
- hosts mutation only accepts `127.0.0.1` or `::1`.
- hosts file read/decode errors fail closed and preserve existing bytes.
- destination replacement never intentionally deletes the live file before a replacement operation is known to succeed.
- Caddy upstreams remain loopback-only.
- generated Caddy global options include `skip_install_trust`.
- explicit trust-store installation is not performed without separate operator approval.
- certification may read/hash system state for non-mutation proof but must not alter it.

## Acceptance criteria

- `AC-01 Exact source/toolchain`: GitHub-hosted Windows/X64 verifies checkout source head equals `EXPECTED_SHA`; rustc/cargo are exactly `1.97.1`; artifact binds canonical base and frozen plan digest.
- `AC-02 Domain plan`: authenticated CLI plan for `demo.test` on a disposable nonzero port returns normalized domain, target `127.0.0.1`, exact port, `tls=true`, `requires_admin_for_hosts_file=true`, and deterministic conflicts; `example.com`, shell-like/invalid labels, and port zero fail.
- `AC-03 Permission split`: authenticated plan succeeds; ordinary authenticated apply/remove/reload fail specifically because `network.manage` is denied. Tests prove `local_authenticated` lacks `NetworkManage`, `local_network_admin` has it, and remote high-risk delegation remains rejected.
- `AC-04 Hosts apply sandbox`: using the production path-scoped mutation on a disposable hosts file preserves unmanaged lines/comments, creates exactly one VSN-managed block, adds only loopback mapping, preserves unrelated managed entries, and repeated apply is idempotent with no duplicate target entry.
- `AC-05 Hosts remove sandbox`: path-scoped removal removes only the requested domain, preserves unmanaged content and other managed entries, and a second remove reports no change.
- `AC-06 Fail-closed hosts read`: invalid UTF-8/unreadable input causes nonzero/error behavior; original bytes remain unchanged; no empty/default replacement is written.
- `AC-07 Failure-safe replacement`: source/tests prove no destination pre-delete. A deterministic replacement-failure case leaves the original target present and byte-identical; stale temporary replacement state is cleaned or explicitly reported. Windows replacement preserves security metadata where the chosen OS primitive supports it.
- `AC-08 HTTPS config safety`: rendered VSN Caddyfile accepts only loopback upstreams, contains global `skip_install_trust`, does not contain `tls_insecure_skip_verify`, and explicit-cert and internal-CA rendering cannot silently request trust-store installation.
- `AC-09 Reload`: reload rejects relative/missing config, validates before reload, never runs reload after validation failure, executes validate then reload on success, stays within bounded helper time/output limits, and returns truthful `validated/reloaded` state. Acceptance uses a sandbox/fake helper, not a system trust mutation.
- `AC-10 Elevation boundary`: source/unit/E2E proof shows `network-admin` checks OS elevation before creating/using the elevated network principal; failure to establish elevation denies the operation. Normal certification records `privileged_system_mutation_performed=false` and does not mutate hosts/trust/resolver state.
- `AC-11 Audit/cleanup/non-mutation`: authenticated operations leave a valid nonzero audit chain; Agent is stopped; IPC key and `LOCALAPPDATA` state are restored; sandbox and fake helper are removed; system hosts pre/post hash is unchanged where readable; no trust-store/resolver mutation was requested.
- `AC-12 Evidence integrity`: evidence binds feature/plan IDs, canonical base, exact source, candidate/product, runner/toolchain, AC checks, measurements, cleanup, system-mutation flag and artifact/evidence SHA-256 values that are independently recomputable.

## Required implementation / certification files

Primary planned files:

- `crates/vsn-network/src/lib.rs`
- `crates/vsn-network/tests/pkg02_hosts_safety.rs`
- `crates/vsn-core/tests/pkg02_domain_policy.rs`
- `scripts/self-hosted/pkg02-0224.ps1`
- `.github/workflows/pkg02-0224-domain-https.yml`

Conditional files only if a mapped acceptance criterion requires them:

- `apps/agent/src/main.rs`
- `apps/agent/Cargo.toml`
- `Cargo.lock`

No other product file may change without mapping to an AC or approved addendum.

## Required commands

- `cargo fmt --all -- --check`
- `cargo clippy --locked --package vsn-network --package vsn-core --package vsn-policy --package vsn-agent --all-targets --no-deps -- -D warnings`
- `cargo test --locked --package vsn-network --package vsn-core --package vsn-policy`
- `cargo build --locked --release --package vsn-agent --package vsn`
- `pwsh -NoProfile -File scripts/self-hosted/pkg02-0224.ps1`
- `git diff --check`

## Required regression gates on final exact head

- AI Planning Governance
- Repository Governance
- PKG-02 Acceptance Sequence
- PKG-02 02.02 Authenticated IPC
- PKG-02 02.08 Windows GitHub-Hosted Certification
- PKG-02 02.14 Local Diagnostics
- PKG-02 02.16 Workspace Text Files
- PKG-02 02.17 Resumable Binary Workspace Transfer
- PKG-02 02.18 Bounded Direct Terminal Execution
- PKG-02 02.19 Persistent Pipe Terminal Sessions
- PKG-02 02.20 PTY ConPTY Lifecycle
- PKG-02 02.21 Loopback Preview Fetch
- PKG-02 02.22 Advanced Preview Requests
- PKG-02 02.23 `.test` DNS Responder
- PKG-02 02.24 Local Domain/HTTPS Boundary

## Evidence artifact

`pkg02-0224-domain-https-github-hosted`

Expected contents include:

- `evidence.json` and SHA-256;
- exact source/base/plan binding;
- domain-plan positive/negative outputs;
- ordinary IPC permission-denial outputs;
- sandbox hosts before/apply/reapply/remove/negative-read/replacement-failure evidence;
- generated Caddyfile and fake-helper validate/reload transcripts;
- system hosts pre/post hash or explicit unreadable status;
- audit output;
- cleanup JSON with every required field;
- `privileged_system_mutation_performed=false`.

## Privileged action approval boundary

The frozen task authorizes development and deterministic sandbox certification of the privileged boundary. It does **not** itself authorize execution against the real machine hosts file, OS resolver, certificate/trust stores, `mkcert -install`, `caddy trust`, or equivalent OS-global state.

Any such actual mutation requires a separate explicit repository-operator approval at execution time. Absence of approval is not an acceptance blocker because normal ACs are designed to prove fail-closed behavior and production semantics without mutating global state.

## Rollout / rollback

Rollout is merge of a genuinely accepted 02.24 PR, followed by machine-state projection from 23/27 active 02.24 to 24/27 active 02.25. Until final evidence and merge, canonical state remains 23/27 active 02.24.

Rollback is PR closure/revert. Normal certification must leave system-global hosts/trust/resolver state unchanged.

## Change control

This plan is frozen by its SHA-256 in the feature manifest. Do not edit it in place after the manifest records its digest. Material scope, permission, acceptance, privileged-action, or rollout changes require an approved addendum or new plan version.
