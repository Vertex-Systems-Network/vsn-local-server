# Remote stream relay — protocol v2 / 0.16

The relay keeps browser, Control Plane and Agent as separate trust boundaries.

## Browser resume

The Control Plane issues a rotating opaque resume token after a relay opens. Only a SHA-256 hash of the resume token is persisted in shared PostgreSQL state. Unexpected browser disconnect detaches the browser for a bounded resume window. A resume request is bound to the same device, principal, permission and exact stream request.

Terminal/file input uses an Agent-generated `InputAck`. The Control Plane advertises the next input sequence acknowledged by the Agent, not merely the next frame accepted from the browser socket. Bounded output replay history, Agent-ACK progress, committed upload bytes and resource progress can be checkpointed in PostgreSQL, allowing resume metadata to be loaded on another Control Plane instance.

## Agent reconnect

Recovery is family-specific and fail-closed:

- File upload: may reopen from Agent-confirmed committed bytes / acknowledged sequence.
- File download: may reopen from emitted-byte/output-sequence progress.
- Preview/database: untouched read-only one-shot requests may be reopened.
- Terminal: PTY/ConPTY is not automatically recreated after Agent loss because command side effects may be unknown. The user must explicitly create a new terminal.

## Cross-instance routing

With `VSN_CONTROL_POSTGRES_DSN` configured, Agent socket ownership is registered in PostgreSQL and bounded stream envelopes can be forwarded through the shared PostgreSQL relay bus. Relay checkpoints and replay frames are also persisted with bounded retention, so reconnect is no longer tied exclusively to the original Control Plane process. Active WebSocket objects remain process-local.

## Live resource families

- Terminal: bidirectional PTY/ConPTY while Agent resource exists.
- File upload: browser → Agent, workspace sandbox, resumable.
- File download: Agent → browser, workspace sandbox, resumable/replayable.
- Database: Agent → browser, workspace-contained SQLite browse/read query only.
- Preview: Agent → browser, localhost GET/HEAD snapshots plus bounded `text/event-stream` SSE with Last-Event-ID, duration/byte caps and fail-fast queue backpressure.

## Remaining boundaries

Terminal process reconstruction after Agent restart is intentionally unsupported. Full preview asset rewriting, cookies and generic local WebSocket forwarding, large-file direct-to-disk browser flow and native external DB transaction/cancellation streams remain pending. Durable terminal scrollback and cancellable external read-job processes already exist.
