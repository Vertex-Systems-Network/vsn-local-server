# Durable cancellable external DB read jobs — VSN 0.14

VSN can run bounded read-only PostgreSQL, MySQL and MariaDB client queries as durable jobs through the Agent.

The Agent validates the database connection spec and read-only single-statement gate, writes a `Running` journal before spawning the native client, redirects stdout/stderr to bounded temporary files and records the exact child PID. The process is polled with a hard timeout. Operator cancellation sets the job cancellation flag and kills/waits the exact child process.

States are: `running`, `completed`, `failed`, `cancelled`, `interrupted`. If a persisted journal says `running` but the current Agent process has no active in-memory job, startup/status recovery marks it `interrupted`; it is never automatically re-executed because query side effects/cost cannot be assumed safe even under a read-only gate.

Local IPC commands:

- `database.cli.job.start`
- `database.cli.job.status`
- `database.cli.job.list`
- `database.cli.job.cancel`

The signed remote command path can expose the same read jobs only when the delegated permission includes `database.query` and the attached device has `allow_remote_database_query=true`.

Current boundary: this is native-client process cancellation, not a generalized server-side query-ID cancellation or binary row-streaming protocol for every database driver.

Recovery invariant: a stale persisted `Running` record is converted to **Interrupted**, never retried automatically.
