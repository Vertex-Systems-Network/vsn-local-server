# PKG-02 02.26 Lifecycle Review — Architecture, Data Flow, Security, Design, QA and Performance

Feature ID: `pkg02-0226-external-native-database-adapters`
Canonical base SHA: `836feb4171a9eb882208a6d666600cea4abe3f42`

## Architecture

Retain the existing local-beta architecture:

`vsn CLI -> authenticated local IPC -> vsn-agent -> vsn-core -> (vsn-database-cli | vsn-database-native) -> selected database client/provider`

No new privileged helper, database daemon, remote control protocol, installer component or plugin subsystem is introduced.

Transport policy is enforced before a provider/client is allowed to connect:
- Core owns authenticated permission and local file-containment checks.
- CLI adapter owns external-client transport construction and bounded child execution.
- Native adapter owns structural endpoint parsing, exact loopback policy, provider TLS construction and native result bounds.
- `vsn-database` owns declared capability truthfulness/conformance.

## Data flow

External client flow:
1. CLI sends an engine connection request over authenticated IPC.
2. Agent creates the normal local authenticated principal.
3. Core checks `DatabaseView` or `DatabaseQuery`.
4. Core validates any credential/CA file against configured workspace roots or VSN-owned data.
5. CLI adapter validates the declared transport profile before spawning a client.
6. Plaintext requires exact loopback; remote requires verified TLS.
7. Adapter constructs engine-specific args/env without password-bearing argv, concurrently drains stdout/stderr, enforces timeout/output limits and returns a bounded result.
8. Agent serializes the bounded response through the existing 1 MiB IPC frame.

Native flow:
1–3 are the same.
4. Core validates CA-file containment before provider use.
5. Native adapter structurally parses the connection target.
6. Plaintext native connections are exact-loopback only; remote accepted profiles verify TLS and identity.
7. Existing query/identifier/mutation safety remains.
8. Materialized strings/cells and serialized results are bounded before returning to Agent.

A failed strict transport validation may never fall back to a weaker transport or another provider.

## Security review

Threats and controls:
- hostname prefix/userinfo spoof -> structural parsing and exact host equality;
- plaintext remote connection -> explicit transport profile; remote requires verified TLS;
- weak external-client defaults -> force strict engine-specific modes;
- arbitrary credential/CA reads -> Core workspace-or-VSN-data containment;
- insecure Mongo URI options -> explicit rejection before driver connection;
- Redis insecure TLS modifier -> explicit rejection before client creation;
- password exposure -> no VSN-synthesized password argv;
- hostile fake client hangs -> bounded timeout and child reap;
- output pipe deadlock -> concurrent drains with byte ceilings;
- oversized native value/result -> 256 KiB materialized text + 512 KiB serialized result ceilings;
- unsupported capability -> explicit fail-closed error without fallback;
- permission escalation -> DatabaseView/DatabaseQuery/DatabaseWrite retained; DatabaseDestructive absent;
- remote scope creep -> no new remote database command permissions;
- stale prep authority -> PR #60 is research only; fresh work binds to canonical `836feb4171a9eb882208a6d666600cea4abe3f42`.

## Declared capability contract

02.26 emits a deterministic unique five-engine beta matrix for PostgreSQL, MySQL, MariaDB, MongoDB and Redis. It must distinguish supported local operations/transport profiles and must not claim remote plaintext, arbitrary Mongo/Redis query execution, unavailable transactions/jobs, or a write surface that the exposed local operator path does not provide.

SQLite remains separately accepted under 02.25 and is not counted as an 02.26 engine.

## Design / operator contract

Preserve existing `vsn db` concepts. Add only the minimum operator surface needed to make existing verified-TLS native PostgreSQL/MySQL read profiles usable through the public local CLI.

Use JSON-over-stdin for rich/TLS connection specs where this avoids connection secrets in VSN CLI process argv. Existing simple loopback commands may remain compatible.

Client detection reports all five engines deterministically even when clients are absent.

Unsupported capability responses explicitly name unsupported operation/transport; they never silently downgrade TLS or substitute an engine.

## QA mapping

- AC-01: exact source/toolchain/base/plan binding.
- AC-02: deterministic declared capability matrix and unknown-engine failure.
- AC-03: five-client detection plus hostile slow/high-output fake clients.
- AC-04: exact-loopback allow/deny matrix and no-connect-on-reject proof.
- AC-05: strict verified remote TLS construction/validation.
- AC-06: unsupported capability and no-downgrade behavior.
- AC-07: authenticated local read/operator path across declared adapters/TLS surfaces.
- AC-08: structured write/permission truthfulness and no remote-permission widening.
- AC-09: process/native result resource ceilings plus real IPC payload measurement.
- AC-10: credential/CA containment and secret-safe command construction.
- AC-11: audit, cleanup and non-system-mutation proof.
- AC-12: exact evidence/artifact/hash integrity.

Real internet database services are not required for normal certification. Deterministic fake client executables and loopback fixtures may prove external command construction, timeout, output and fail-closed behavior. Provider-local tests may prove transport parsing/TLS option construction without trusting external SaaS.

## Performance / resource review

Frozen budgets:
- authenticated IPC frame: existing 1 MiB;
- client detection timeout: <=5 seconds per executable;
- synchronous external operation timeout: <=30 seconds;
- external stdout: <=512 KiB;
- external stderr: <=256 KiB;
- native materialized text/string cell: <=256 KiB;
- native serialized read result: <=512 KiB;
- native browse row limit: existing max 1000;
- certification: GitHub-hosted Windows with workflow timeout.

Output readers begin before waiting for child exit so pipe backpressure cannot deadlock the timeout path. Timeout handling kills/reaps the child.

## Failure behavior

- malformed/unknown engine -> explicit error;
- plaintext non-loopback/spoof/ambiguous host -> reject before connect/spawn;
- remote without verified TLS/CA -> reject before connect/spawn;
- insecure Mongo/Redis options -> reject before connect/spawn;
- missing/outside credential/CA path -> reject before provider/client use;
- unavailable client -> deterministic unavailable state;
- client timeout -> bounded error with child reaped;
- output ceiling -> bounded error, never IPC overflow;
- native oversized cell -> explicit truncation metadata;
- native aggregate over ceiling -> deterministic provider-bound rejection;
- unsupported operation -> explicit unsupported error;
- cleanup failure -> acceptance failure.

## Rollout review

Until final exact-head AC-01..AC-12 evidence and all frozen regression gates pass, canonical state remains `25/27`, active `02.26`.

Only after accepted 02.26 integration may a separate state-only projection advance to `26/27`, active `02.27`. No 02.27 implementation belongs to this task.
