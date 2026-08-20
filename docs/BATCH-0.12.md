# Batch 0.12.0 — resumable distributed streams, shared operational state, cloud VM lifecycle

## Delivered

- Relay protocol v2 with rotating resume tokens, 60-second browser reconnect window, bounded output replay history and sequence checkpoints.
- Agent-processed `InputAck`: browser input is considered acknowledged only after the Agent applies the side effect. Terminal input can replay unacknowledged sequence numbers after reconnect; file upload resumes from Agent-committed chunk/byte state.
- PostgreSQL cross-instance stream bus: browser-home and Agent-owner Control Plane instances can forward bounded relay frames instead of reporting a device as offline merely because its socket belongs to another instance.
- Shared PostgreSQL command delivery: queued → leased with `FOR UPDATE SKIP LOCKED` → completed/failed, with attempts, expiry and signed result payload persisted across instances.
- Shared device and pairing registry used as an authoritative fallback by polls, Agent stream enrollment, command validation and audit ingestion.
- Shared transactional approvals: approval decision and signed command insertion commit in one PostgreSQL transaction, preventing duplicate multi-instance approvals and half-committed approval states.
- Shared signed central audit chain: per-device advisory transaction lock, event-id/hash duplicate checking and previous-hash continuity prevent concurrent Control Plane nodes from forking a device audit chain.
- Shared rate-limit counters for PostgreSQL-backed deployments.
- Workspace-contained live SQLite read stream (`browse` / conservative read query) through the same browser relay.
- Browser live database panel for the SQLite stream.
- Resume-capable browser file upload/download UI with bounded in-memory limits; uploads derive restart offset from Agent acknowledgements.
- Live read-only preview dashboard. HTML is rendered in a sandboxed iframe without script or same-origin privileges.
- Preview and database responses are chunked to stream-frame size rather than emitted as oversized single frames.
- AWS, Azure and GCP CLI-backed VM create/status/start/stop/destroy lifecycle. Create paths are private-by-default where the provider CLI exposes the corresponding flag; operations use argument arrays, bounded output/timeouts and existing CLI authentication context.
- Cloud resource names are stricter identifiers and destructive cloud operations remain local-only behind `RemoteManage`; they are not added to the remote signed-command allowlist.
- AWS/Azure/GCP example cloud provider manifests.
- Shared result listing and shared device listing improve multi-instance dashboard consistency.

## Security boundaries retained

- Remote terminal/file writes/external database query remain local opt-in features on the Agent.
- Generic live DB relay remains workspace SQLite read-only. Native external DB mutation is not exposed through the live relay.
- Preview relay remains localhost-only GET/HEAD. Full WebSocket/SSE/cookie/asset tunnel semantics are not claimed.
- Cloud credentials are not accepted in VSN VM create requests; installed provider CLI authentication/config is used.
- Cloud create/start/stop/destroy are not remotely invokable through the signed Agent command allowlist.
- Browser resume state is still owned by the original Control Plane process; session affinity remains recommended during the resume window.

## Still partial

- Agent WebSocket disconnect does not yet reconstruct active relays automatically.
- Browser replay history is bounded; it is not a durable terminal scrollback or large-file durable spool.
- Account/session/OIDC/WebAuthn pending state is not yet fully normalized into shared multi-instance tables.
- Full preview asset rewriting, cookies, SSE and local WebSocket forwarding remain pending.
- External DB stream cancellation, transactions and row/binary framing remain pending.
- Cloud snapshots, cloning, migration, provider SDK/API implementations and zero-downtime orchestration remain pending.
