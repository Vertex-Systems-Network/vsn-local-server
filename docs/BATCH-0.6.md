# Batch 0.6.0 — Integration, fleet, approvals and safe orchestration

This batch moves VSN further from foundation/scaffold work into connected product slices.

## Implemented in 0.6

- Chunked binary workspace file transfer with offset enforcement, bounded chunks, optional final SHA-256 verification and staged backup/restore replacement.
- Multi-node fleet groups, device labels and environment topology records in the Control Plane.
- Sensitive remote command approval workflow with pending/approved/rejected/expired states.
- Central Agent audit ingestion: device identity, event signature/hash and per-device chain continuity are verified before persistence.
- In-memory scoped Control Plane request rate limiting baseline.
- Control Plane single-node JSON persistence now uses fsynced staging, backup/restore replacement and private Unix file/directory modes; it is still not a transactional HA datastore.
- Generic cloud workspace specification + fail-closed provisioning-plan abstraction; no provider credentials or destructive provision call is executed by the planner.
- Structured AI tool planner for machine/project/database diagnosis and project bootstrap planning. It emits VSN tool calls and explicitly disallows unrestricted shell access.
- Runtime archive extraction preflights entry paths and rejects path traversal/symlink packages; a separate trusted signed-catalog verify/install path is available
- Signed update manifest/artifact verification baseline; update application/rollback is not implemented yet.
- Local encrypted vault persistence is serialized and uses fsynced staged backup/restore writes with private Unix modes.
- Docker/Podman image, volume, network and bounded-log read surfaces.
- Extension install hardening: signature trust roots, permission allowlist, symlink rejection, package limits, staging/atomic install and persisted signer record.
- Local Agent/CLI wiring for extension verify/install/list/uninstall, AI planning, cloud planning, binary file transfer and update verification.
- Browser Control Plane dashboard now surfaces approvals, fleet topology and centrally synced audit events.
- Signed offline marketplace index verification/search baseline; package installation still passes through the separate signed extension trust boundary.

## Deliberately still incomplete

- Rust compilation/native Tauri build is not verified in the artifact-generation environment because Rust is unavailable there.
- Production account authentication, passkeys/MFA and recovery are not implemented.
- Control Plane persistence remains single-node JSON; PostgreSQL/transactional multi-instance storage is still pending.
- Cloud workspace provisioning is a validated plan only; AWS/Azure/GCP/VPS provider apply/destroy adapters are not implemented.
- AI is a structured deterministic planner boundary only; there is no LLM execution dependency in the Agent.
- PTY streaming terminal, streaming file transport protocol, native DB proxy and live WebSocket/SSE preview tunnel remain pending.
- Signed updater apply/rollback and release-key operational workflow remain pending.
