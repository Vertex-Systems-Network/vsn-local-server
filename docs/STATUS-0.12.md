# VSN roadmap status — 0.12.0

The machine-readable source of truth is `docs/roadmap-status.json`.

- Done / usable baseline: **8** phases
- Meaningful partial: **22** phases
- Pending: **1** phase (`P30`)

0.12 is primarily a distributed-runtime and remote-development hardening batch. It adds browser relay resume checkpoints, Agent-processed input acknowledgements, a PostgreSQL cross-instance stream bus, shared command leases/results, shared transactional approvals, shared signed audit continuity, shared rate limiting, live SQLite DB streaming, resume-capable browser file transfers, a sandboxed live preview panel, and cloud CLI VM lifecycle operations for AWS/Azure/GCP.

`partial` still means substantial work remains. The largest production gaps are Agent reconnect/durable relay state, shared account/session/auth-transaction storage, full HTTP/WebSocket/SSE preview tunneling, external DB streaming semantics, native cloud-provider APIs/snapshots/migrations, SAML/SCIM, and native build/security/load testing.
