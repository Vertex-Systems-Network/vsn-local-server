# VSN roadmap status — 0.11.0

The machine-readable source of truth is `docs/roadmap-status.json`.

- Done / usable baseline: **8** phases
- Meaningful partial: **22** phases
- Pending: **1** phase (`P30`)

0.11 materially advances P12–P18, P28 and P29. The most important new product slice is a real browser-to-Agent live PTY/ConPTY stream through the Control Plane while retaining device-local permission checks. OIDC now reaches verified callback/session creation for explicitly linked subjects. PostgreSQL-backed deployments gain Control Plane instance and Agent-stream route visibility, but not cross-instance frame forwarding.

`partial` does not mean nearly finished: production streaming resume, distributed coordination, cloud provisioning, security testing and signed platform release work remain substantial.
