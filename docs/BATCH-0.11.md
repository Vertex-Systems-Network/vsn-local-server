# Batch 0.11.0 — live remote streams, verified OIDC callback, cluster routing awareness

## Delivered

- Browser ↔ Control Plane ↔ Agent WebSocket stream relay with bounded per-hop queues.
- Browser token is sent in the first encrypted WebSocket application frame, not in the URL.
- Control Plane signs a short-lived `stream.relay.open` authorization bound to device, principal, permission, relay session and exact stream request.
- Agent re-verifies the signature, permission and local feature toggle before opening a resource.
- Live terminal stream can attach an existing PTY or create a new workspace-contained PTY/ConPTY from explicit program/cwd/args metadata.
- File download and resumable file upload streams use the existing workspace sandbox and chunk limits.
- Read-only preview streams use the existing localhost-only GET/HEAD preview path.
- Slow browser consumers cannot indefinitely block the shared Agent stream socket; Control Plane fan-out uses bounded queues and fail-closed relay cleanup.
- Control Plane pings Agent stream sockets to keep the connection active and refresh route ownership.
- Optional shared PostgreSQL store now tracks Control Plane instance heartbeats and Agent-stream route ownership. Cross-instance byte forwarding is intentionally not claimed yet.
- OIDC callback now consumes one-time server-side PKCE state, disables HTTP redirects for provider discovery/token calls, exchanges the authorization code, verifies the ID token with provider metadata/JWKS plus issuer/audience/nonce checks, and requires explicit provider-subject → VSN account linking.
- Unknown OIDC identity never auto-links by email.
- Dashboard now exposes live terminal streaming, OIDC explicit mapping controls and cluster visibility.

## Security boundaries retained

- Remote terminal and file writes still require local Agent opt-in.
- Database live streaming is not exposed until cancellation/transaction semantics are defined.
- Preview stream is GET/HEAD only; mutation-capable localhost proxy remains local-only.
- Shared PostgreSQL records route ownership but does not silently pretend cross-instance frame forwarding exists.
- OIDC provider identities are explicit mappings; validated email alone cannot take over an existing VSN account.

## Still partial

Reconnect/resume, dedicated browser file-transfer UI, full live preview assets/WebSocket/SSE tunneling, cross-instance relay bus, shared distributed queues/sessions, SAML and production HA remain future work.
