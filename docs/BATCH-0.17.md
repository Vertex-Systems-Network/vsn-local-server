# Batch 0.17 — interactive preview, native PostgreSQL control and release certification source

## Implemented

- Permission-separated interactive localhost WebSocket preview relay. Bidirectional preview requires `project.edit` and the device-local `allow_remote_preview_interactive` opt-in; snapshot/SSE remain read-oriented.
- Native loopback PostgreSQL server-cancel jobs using the driver's cancellation token, with `cancel_requested` separated from terminal job state.
- Native PostgreSQL bounded `BEGIN READ ONLY` transaction sessions with VSN TTL, statement-count cap and server statement/idle timeouts.
- Desktop + Agent + CLI surfaces for native PostgreSQL jobs and transaction sessions.
- Windows WiX/MSI, Linux deb and macOS pkg source for Agent/CLI/updater helper, plus separate Windows signing and macOS signing/notarization scripts.
- Service-aware updater handoff scripts for Windows service, Linux user systemd and macOS LaunchAgent.
- Multi-OS release-gate package jobs, scheduled cargo-fuzz/RustSec workflow and bounded Control Plane load probe.
- Formal 0.17 contracts for WebSocket preview, native PostgreSQL controls and runtime package artifacts.

## Deliberately not claimed complete

- WebSocket preview is a bounded localhost development relay, not a generic TCP/VPN tunnel or complete cookie/asset proxy.
- PostgreSQL cancellation is asynchronous and may race query completion; `cancel_requested` is not itself proof of cancellation.
- Native PostgreSQL transaction sessions live in the Agent process and are intentionally not reconstructed after restart.
- WiX/MSI/deb/pkg and signing scripts are release engineering source until real target runners, signing/notarization and installer acceptance pass.
- cargo-fuzz/load workflows are defined, but no external runner evidence is claimed by this offline artifact build.
