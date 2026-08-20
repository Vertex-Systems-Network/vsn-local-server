# VSN Batch 0.16

Batch 0.16 concentrates on production-state normalization, safer updater execution, long-lived localhost preview events, provider image migration and measurable roadmap progress.

## Delivered

- Shared PostgreSQL normalization for API tokens, fleet groups, environments and per-device fleet metadata, including guarded one-time backfill and cross-node refresh.
- Dedicated `vsn-updater-helper` process with exclusive update lock, status, rollback and explicit stale-lock recovery. Core update apply/rollback also use the same locked wrappers so concurrent update entry points fail closed.
- Bounded localhost SSE (`text/event-stream`) preview relay with redirects disabled, `Last-Event-ID`, 5–300 second duration bounds, 16 MiB total cap, 64 KiB chunks and fail-fast queue backpressure.
- Browser live-preview SSE mode over the existing authenticated Browser ↔ Control Plane ↔ Agent relay.
- AWS AMI region-copy operation, AWS/GCP clone target-location support, explicit confirmation and continued local `RemoteManage` boundary for infrastructure mutation.
- Formal contracts for SSE preview, updater helper, cloud image-copy/target-location and roadmap completion percentages.
- Machine-readable P0–P30 `completion_percent` values plus `overall_completion_percent`.

## Deliberate boundaries

- Generic local WebSocket proxying, asset rewriting/cookies and hot reload are still not equivalent to a full browser tunnel.
- Azure deterministic cross-region/full-VM clone remains fail-closed.
- Time-based stale updater-lock recovery requires explicit confirmation and a lock older than ten minutes; it does not guess cross-platform process liveness.
- Percentages measure estimated completion against the intended production scope, not whether a phase merely has an initial implementation.
