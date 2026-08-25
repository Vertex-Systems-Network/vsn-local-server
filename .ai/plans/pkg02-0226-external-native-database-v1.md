# PKG-02 02.26 Frozen Plan — External/Native Database Beta Adapters

Feature ID: `pkg02-0226-external-native-database-adapters`
Version: `1.0.0`
Canonical base SHA: `836feb4171a9eb882208a6d666600cea4abe3f42`
Approval reference: `docs/MASTER-EXECUTION-PLAN.md — frozen PKG-02 task 02.26`
Approved date: `2026-08-25`

## Outcome

Genuinely certify:

`02.26 — External/native database beta adapters: client detection plus PostgreSQL/MySQL/MariaDB/MongoDB/Redis declared-capability handling, with loopback/TLS and unsupported-capability fail-closed rules.`

## In scope

- deterministic detection declarations for `psql`, `mysql`, `mariadb`, `mongosh`, `redis-cli`;
- truthful declared beta capabilities for PostgreSQL, MySQL, MariaDB, MongoDB, Redis;
- structural exact-loopback validation for plaintext profiles;
- verified remote TLS profiles and insecure-option rejection;
- existing native read/structured-write functionality required by the declared local beta matrix;
- existing PostgreSQL/MySQL verified-TLS native read profiles exposed through a usable local CLI path;
- bounded external child execution and bounded native read result materialization;
- credential/CA file containment inside registered workspace roots or VSN-owned data;
- preservation of DatabaseView/DatabaseQuery/DatabaseWrite and unsupported-capability fail-closed behavior;
- exact-source GitHub-hosted Windows evidence, audit verification and cleanup;
- only bug fixes directly required by these ACs.

## Explicit non-goals

- no 02.27 implementation;
- no database server installation/service lifecycle acceptance;
- no arbitrary MongoDB JavaScript or Redis command surface;
- no import/export/backup/restore expansion;
- no remote Control Plane production database acceptance or new remote command permissions;
- no DatabaseDestructive grant;
- no password-bearing VSN-generated argv;
- no global trust-store modification;
- no Desktop redesign;
- no installer/updater/release work;
- no denominator/order/product/candidate change.

## Dependencies

- canonical 02.01–02.25 integrated DONE;
- PKG-02 `25/27 = 92.59%`, active `02.26`;
- canonical base `836feb4171a9eb882208a6d666600cea4abe3f42`;
- Rust/cargo exact `1.97.1`;
- authenticated IPC `127.0.0.1:39731`;
- candidate `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`, product `0.38.1`;
- existing policy split and Core credential containment;
- existing PostgreSQL/MySQL native TLS connectors.

## User-visible / operator behavior

- `vsn db clients` reports all five external beta clients with availability/version and never hangs indefinitely on a broken executable.
- Plaintext profiles are allowed only to exact `localhost`, `127.0.0.1`, or `::1`.
- Remote PostgreSQL/MySQL/MariaDB/MongoDB/Redis operations are accepted only through verified TLS profiles.
- PostgreSQL/MySQL native TLS inspect/browse/query are reachable through a documented local CLI path without VSN putting connection secrets into password arguments.
- MongoDB/Redis arbitrary query/script remains explicitly unsupported.
- Unknown engines/transports/capabilities fail closed.
- External/native read output is bounded below the authenticated IPC frame.

## Security / transport constraints

### Loopback

Loopback classification is structural. Exact accepted plaintext host identities:
- `localhost`
- `127.0.0.1`
- `::1`

Lookalikes (`localhost.evil.invalid`), embedded userinfo tricks, remote aliases, ambiguous multi-host plaintext forms and port `0` fail before connect/spawn.

### Remote verified TLS

- PostgreSQL external: force `sslmode=verify-full` + trusted CA.
- PostgreSQL native TLS: preserve CA and hostname/certificate verification; no danger bypass.
- MySQL external: force `--ssl-mode=VERIFY_IDENTITY` + trusted CA.
- MariaDB external: force TLS + trusted CA + server-certificate verification.
- MySQL native TLS: preserve CA and hostname/certificate verification; no danger bypass.
- MongoDB: remote must be TLS-capable; reject `tls=false`, `ssl=false`, insecure TLS, invalid-hostname and invalid-certificate options.
- Redis: remote must use TLS + trusted CA; reject insecure URL/TLS modifiers.
- strict TLS failure may not retry plaintext/weaker TLS.

### Secrets/files

Credential/CA paths used by authenticated operations must resolve under a configured workspace or VSN-owned data. VSN must not synthesize password-bearing command args. Certification contains no production secrets.

### Resource bounds

- client detection <=5 seconds/executable;
- synchronous external operation <=30 seconds;
- external stdout <=512 KiB;
- external stderr <=256 KiB;
- native materialized text/string cell <=256 KiB;
- native serialized read result <=512 KiB;
- authenticated IPC remains 1 MiB.

Child stdout/stderr are concurrently drained before/while waiting; timeout kills/reaps the child.

## Acceptance criteria

- `AC-01 Exact source/toolchain`: GitHub-hosted Windows/X64 verifies checkout source == `EXPECTED_SHA`; rustc/cargo exactly 1.97.1; evidence binds base, feature/plan IDs, plan digest, product/candidate and IPC.
- `AC-02 Declared capabilities`: authenticated local conformance output contains exactly one deterministic entry for each 02.26 engine PostgreSQL, MySQL, MariaDB, MongoDB, Redis; declarations truthfully match supported inspect/browse/query/write/index/relation/statistics/job/transaction/transport surfaces. Unknown engine/provider/capability fails closed. SQLite is not counted as a 02.26 engine.
- `AC-03 Client detection`: five executable mappings deterministic; absent clients report unavailable; hostile fake clients prove `--version` detection <=5 seconds, bounded stdout/stderr and no hang/deadlock.
- `AC-04 Plaintext loopback boundary`: PostgreSQL/MySQL/MariaDB plaintext accepts exact `localhost`, `127.0.0.1`, `::1` and rejects `localhost.evil.invalid`, `127.0.0.1.evil.invalid`, userinfo/prefix tricks, remote names, ambiguous multi-host plaintext and port 0 before connect/spawn. No client/connect attempt occurs after rejection.
- `AC-05 Verified remote TLS`: deterministic construction/validation proves PostgreSQL `verify-full`+CA, MySQL `VERIFY_IDENTITY`+CA, MariaDB TLS+CA+server-cert verification, MongoDB TLS/SRV without insecure overrides, Redis TLS+CA without insecure modifiers. Missing CA where required, TLS disable, invalid-host/cert bypass, plaintext remote or weaker/fallback profiles fail before connect/spawn.
- `AC-06 Unsupported capability fail-closed`: MongoDB/Redis arbitrary script/query stays rejected; undeclared write/query/transaction/job surfaces return explicit unsupported errors; unknown transport/provider rejected; no path downgrades TLS or substitutes engine/capability.
- `AC-07 Authenticated local read/operator path`: release Agent/CLI authenticated IPC exposes client detection and supported local adapter reads with correct DatabaseView/DatabaseQuery authorization. PostgreSQL/MySQL verified-TLS inspect/browse/query have a public local CLI request path. Fake/loopback fixtures may prove transport construction where no real server is required.
- `AC-08 Write and permission truthfulness`: existing structured PostgreSQL/MySQL/MongoDB/Redis writes stay behind DatabaseWrite; update/delete filter protections remain; local_authenticated retains DatabaseView/DatabaseQuery/DatabaseWrite and lacks DatabaseDestructive; no remote command permission or privileged mutation surface is added.
- `AC-09 Resource/process safety`: fake clients prove sync detection/operation timeout, concurrent drains, output ceilings and child cleanup. Native PostgreSQL/MySQL/MongoDB/Redis read materialization enforces 256 KiB string-cell and 512 KiB serialized-result ceilings with truthful truncation metadata or bounded provider error. Maximum successful release CLI payload is measured <1 MiB.
- `AC-10 Credential/CA and secret safety`: external credential files and all 02.26 CA files reject missing/outside-workspace-or-VSN-data and Windows junction escape paths before use; contained fixture files succeed. VSN-generated client argv contains no password/secret from request; temp credential/CA fixtures removed.
- `AC-11 Audit/cleanup/non-mutation`: valid nonzero audit chain; Agent stopped; IPC key and LOCALAPPDATA restored; fake clients/jobs/sandbox/CA/credential fixtures removed; no hosts/resolver/system trust-store mutation; `privileged_system_mutation_performed=false`; no production/remote database mutation.
- `AC-12 Evidence integrity`: evidence binds feature/plan/base/source/product/candidate/runner/toolchain, five-engine matrix, transport cases, timing/output/payload measurements, permission proof, Agent/CLI hashes, audit/cleanup/non-mutation flags, artifact ZIP SHA-256 and independently recomputable evidence SHA-256.

## Required implementation / certification files

Primary planned product files:
- `crates/vsn-database/src/lib.rs`
- `crates/vsn-database-cli/src/lib.rs`
- `crates/vsn-database-native/src/lib.rs`
- `crates/vsn-core/src/lib.rs`
- `apps/cli/src/main.rs`

Conditional product file only if mapped AC requires Agent parse/dispatch adjustment:
- `apps/agent/src/main.rs`

Focused tests may be added under:
- `crates/vsn-database/tests/`
- `crates/vsn-database-cli/tests/`
- `crates/vsn-database-native/tests/`
- `crates/vsn-core/tests/`

Certification:
- `scripts/self-hosted/pkg02-0226.ps1`
- `.github/workflows/pkg02-0226-external-native-databases.yml`

Cargo manifests / `Cargo.lock` may change only if an AC cannot be met with existing dependencies. No other product file changes without AC mapping or approved addendum.

## Required commands

- `cargo fmt --all -- --check`
- `cargo clippy --locked --package vsn-database --package vsn-database-cli --package vsn-database-native --package vsn-core --package vsn-policy --package vsn-agent --package vsn --all-targets --no-deps -- -D warnings`
- `cargo test --locked --package vsn-database --package vsn-database-cli --package vsn-database-native --package vsn-core --package vsn-policy`
- `cargo build --locked --release --package vsn-agent --package vsn`
- `pwsh -NoProfile -File scripts/self-hosted/pkg02-0226.ps1`
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
- PKG-02 02.25 SQLite Database Studio
- PKG-02 02.26 External/Native Database Adapters

## Evidence artifact

`pkg02-0226-external-native-database-github-hosted`

Expected contents:
- evidence.json + independently recomputable SHA-256;
- exact source/base/feature/plan binding;
- five-engine capability/conformance;
- client detection plus slow/high-output fake-client timings;
- exact-loopback positive and spoof/remote/multi-host/port-zero rejections;
- per-engine verified-TLS construction/rejection;
- unsupported/no-downgrade evidence;
- contained/outside/junction credential/CA path evidence;
- native large-cell/result + final CLI payload measurements;
- permission/remote-non-expansion proof;
- audit;
- cleanup/non-system-mutation JSON;
- Agent/CLI hashes.

## Rollout / rollback

After genuine 02.26 acceptance/merge, a separate state-only projection may advance `25/27`, active 02.26 -> `26/27`, active 02.27. Until then canonical state remains 25/27 active 02.26.

Rollback is PR closure/revert. Certification uses disposable local fixtures/fake clients and leaves system trust/hosts/resolver and production/remote databases unchanged.

## Change control

This plan is frozen by SHA-256 in the feature manifest. Do not edit in place after manifest records its digest. Material scope, permission, transport, acceptance, resource-budget or rollout changes require an approved addendum or new plan version.
