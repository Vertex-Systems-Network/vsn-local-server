# Durable database query artifacts — VSN 0.15

External PostgreSQL/MySQL/MariaDB read jobs keep a bounded in-memory preview. Successful result output larger than the preview threshold is spooled into VSN-owned job storage instead of being silently discarded. Artifacts are capped at 64 MiB and carry a SHA-256 digest.

Clients read completed output with `database.cli.job.output` using an explicit byte offset and a maximum chunk of 256 KiB. `database.cli.job.output-remove` is mutation-gated and refuses to treat a running query as a completed artifact. Query cancellation still terminates the exact client process; restart recovery marks an orphaned running journal as `interrupted` instead of re-executing it.
