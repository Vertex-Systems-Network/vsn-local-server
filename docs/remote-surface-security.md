# Remote surface security — 0.5 baseline

Remote capability is intentionally layered rather than equivalent to unrestricted remote desktop access.

## Read surfaces

`files.list`, `files.read`, project detection, process/runtime status, DB introspection and preview requests are bound to Agent permissions. Files must resolve inside configured workspace roots. Preview requests can only target `127.0.0.1`, GET/HEAD, with redirects disabled.

## Write/execution surfaces

Remote file writes, terminal execution and external DB queries each require both:

1. a scoped Control Plane token/role permitted to delegate the matching Agent permission; and
2. an explicit local machine opt-in in `RemoteConfig`.

The local defaults are all false.

The terminal runner supplies arguments directly to `Command`; it does not parse shell operators. This is not a security sandbox: a caller permitted to launch an interpreter can intentionally execute code under the Agent OS account. Production deployment should add MFA/approval and, where appropriate, command policy/isolated execution.

## Result delivery

A signed command is leased rather than removed during polling. If a result upload fails, the command is redelivered after the lease. The Agent detects the duplicate command ID and returns the cached semantic result without executing the command again, signing a fresh result envelope for transport freshness. The Control Plane treats a completed matching command/session result as idempotently acknowledged.

## Data-size protections

- Agent remote payload: 1.5 MiB maximum serialized payload.
- Control Plane JSON body: 2 MiB maximum.
- Preview body: 512 KiB maximum.
- Terminal stdout/stderr: 512 KiB each.
- External DB client stdout: 512 KiB maximum.
- Control Plane retains the most recent 100 full signed results in the single-instance baseline.

These are transport/storage safety bounds, not quotas for a future streaming data plane.
