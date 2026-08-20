# VSN Protocol v0.2

## 1. Local IPC transport decision

P2 uses authenticated loopback TCP as the first transport implementation:

`127.0.0.1:49731`

This is not the permanent security boundary. Authentication and authorization are in the protocol/Agent layers, so native named pipes or Unix domain sockets may replace the transport later.

### Local request envelope

```json
{
  "version": 1,
  "timestamp_unix_ms": 1787000000000,
  "nonce": "96d00d...",
  "command": "status",
  "params": {},
  "mac": "base64-hmac-sha256"
}
```

### Local response envelope

```json
{
  "version": 1,
  "timestamp_unix_ms": 1787000000123,
  "request_nonce": "96d00d...",
  "ok": true,
  "payload": {},
  "mac": "base64-hmac-sha256"
}
```

Security rules:

- Requests older/newer than 30 seconds are rejected.
- A nonce cannot be accepted twice within the replay cache.
- Frames larger than 1 MiB are rejected.
- Responses are authenticated and bound to the originating request nonce.

## 2. Future remote Agent protocol

Remote commands will use a richer transport-independent envelope:

```json
{
  "request_id": "uuid",
  "issued_at": "RFC3339",
  "actor": "user-or-system-id",
  "machine_id": "device-id",
  "action": "service.start",
  "resource": "postgresql:17",
  "parameters": {},
  "approval": null
}
```

Result:

```json
{
  "request_id": "uuid",
  "status": "success",
  "result": {},
  "error": null,
  "audit_event_id": "uuid"
}
```

## Design rules

- Every command maps to a permission.
- Destructive actions are explicit actions, never inferred from generic write permission.
- Retries must not duplicate non-idempotent operations.
- The Agent validates authentication and authorization before execution.
- Transport identity, user identity, and authorization context remain separate concepts.
