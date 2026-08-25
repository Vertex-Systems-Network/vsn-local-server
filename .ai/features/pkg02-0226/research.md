# PKG-02 02.26 Research — External/Native Database Beta Adapters

Feature ID: `pkg02-0226-external-native-database-adapters`
Canonical base SHA: `836feb4171a9eb882208a6d666600cea4abe3f42`
Reviewed: `2026-08-25`

## Canonical scope

Frozen task:

`02.26 — External/native database beta adapters: client detection plus PostgreSQL/MySQL/MariaDB/MongoDB/Redis declared-capability handling, with loopback/TLS and unsupported-capability fail-closed rules.`

Canonical machine state is PKG-02 `25/27 = 92.59%`, active `02.26`. `02.27` remains blocked. Product version `0.38.1` and release candidate `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474` are unchanged.

## Stale preparation review

Open PR #60 was inspected only as historical preparation. It is stacked on an obsolete preparation base from the earlier 02.08-era execution state and is not an implementation baseline or acceptance authority. Useful audit leads were revalidated against current canonical `main`.

Open PR #61 is a future 02.27 scaffold. It remains blocked and is out of scope.

## Current repository audit

The current repository already contains substantial 02.26 functionality:
- `vsn-database-cli` declares PostgreSQL/`psql`, MySQL/`mysql`, MariaDB/`mariadb`, MongoDB/`mongosh`, Redis/`redis-cli`;
- `vsn-database-native` implements PostgreSQL, MySQL/MariaDB, MongoDB and Redis native read/structured-write surfaces;
- PostgreSQL and MySQL verified-TLS native read profiles already exist in Core/Agent;
- `DatabaseView`, `DatabaseQuery`, and `DatabaseWrite` are present for the local authenticated principal while `DatabaseDestructive` remains absent/high-risk;
- `vsn-database` already has capability metadata and fail-closed unsupported behavior.

02.26 is transport/capability hardening plus genuine operator certification, not a provider rewrite.

Acceptance-blocking defects on canonical main:
1. PostgreSQL plaintext native loopback validation is substring-spoofable (`localhost.evil.invalid` can be misclassified).
2. MySQL plaintext native loopback validation is similarly substring-spoofable.
3. External `ConnectionSpec` has no explicit TLS/transport policy, so remote client invocations cannot prove verified TLS.
4. Client detection and synchronous inspect/query use unbounded `Command::output()`, permitting hangs and pre-bound output growth.
5. Native read results are row-bounded but not byte-bounded, so a valid result can exceed the 1 MiB authenticated IPC frame.
6. MongoDB `mongodb+srv://` is accepted without rejecting explicit TLS-disable or invalid-host/certificate options.
7. Redis remote TLS should explicitly reject insecure URL/TLS modifiers instead of relying on the current feature graph.
8. External credential files are Core-contained, but native TLS CA paths are passed directly to the provider.
9. Core/Agent expose PostgreSQL/MySQL verified-TLS read commands, but the public CLI lacks a corresponding local operator path.

Behavior to preserve:
- MongoDB/Redis arbitrary query/script execution remains unsupported.
- Native structured update/delete retains non-empty-filter safety.
- Remote Control Plane database expansion is not part of PKG-02 02.26.
- No password-bearing VSN-generated argv.
- IPC remains exactly 1 MiB.

## Primary-source delta research

Official provider documentation reviewed on 2026-08-25:
- PostgreSQL SSL: https://www.postgresql.org/docs/18/libpq-ssl.html — `verify-full` verifies trusted CA and server hostname.
- PostgreSQL connection parameters: https://www.postgresql.org/docs/18/libpq-connect.html — weaker/default modes can omit hostname verification or permit fallback.
- MySQL connection options: https://dev.mysql.com/doc/refman/8.4/en/connection-options.html — `VERIFY_IDENTITY` verifies both CA trust and server hostname.
- MariaDB client: https://mariadb.com/kb/en/mariadb-command-line-client/ — verified identity requires TLS + CA + server-certificate verification.
- MongoDB connection strings/TLS: https://www.mongodb.com/docs/manual/reference/connection-string/ — SRV enables TLS by default but TLS may be explicitly disabled and insecure TLS options exist.
- Redis TLS: https://redis.io/docs/latest/operate/oss_and_stack/management/security/encryption/ — TLS clients use TLS plus a trusted CA.
- Rust redis crate: https://docs.rs/redis/1/redis/ — `rediss://` is TLS and insecure TLS is separately represented.
- Rust mysql crate `SslOpts`: https://docs.rs/mysql/28/mysql/struct.SslOpts.html — invalid-cert/hostname danger overrides default to disabled.

Market/API delta: **no roadmap expansion**. The material delta is security correctness: explicit transport, verified identity, fail-closed downgrade prevention and resource bounds consistent with the existing authenticated IPC contract.

## Planning conclusions

- Parse endpoints structurally; never classify loopback by substring.
- Plaintext is permitted only for exact `localhost`, `127.0.0.1`, and `::1`; ambiguous/multi-host plaintext and port zero fail closed.
- Remote external PostgreSQL forces `sslmode=verify-full` + trusted CA.
- Remote external MySQL forces `VERIFY_IDENTITY` + trusted CA.
- Remote external MariaDB forces TLS + CA + server-certificate verification.
- Remote MongoDB rejects TLS-disable/invalid-cert/invalid-host/insecure options.
- Remote Redis uses verified TLS and rejects insecure modifiers.
- External client detection uses a 5-second bounded child helper.
- External synchronous operations use a 30-second ceiling, concurrent stdout/stderr draining, <=512 KiB stdout and <=256 KiB stderr.
- Native materialized text/string cells use <=256 KiB and serialized read results <=512 KiB.
- Credential/CA files must resolve inside configured workspace roots or VSN-owned data.
- Add the minimum public CLI path for existing PostgreSQL/MySQL TLS read commands without synthesizing secret-bearing argv.
- Preserve DatabaseView/DatabaseQuery/DatabaseWrite and absence of DatabaseDestructive; do not broaden remote permissions.
- GitHub-hosted Windows/X64 exact-head evidence remains the acceptance authority.
