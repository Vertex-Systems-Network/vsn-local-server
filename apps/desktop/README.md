# VSN Desktop — 0.6 source application

Stack: Tauri v2 + React + TypeScript.

The Tauri backend is an authenticated bridge to the VSN Agent. The desktop UI does not directly perform operating-system mutations, preserving the Agent/policy boundary.

Current source screens:

- Overview / local machine security posture
- Projects and registered workspace roots
- VSN-managed OS services
- Runtime inventory plus catalog-driven install/uninstall
- Process snapshot
- Docker/Podman backend status
- Database Studio: SQLite plus PostgreSQL/MySQL/MariaDB/MongoDB/Redis client-adapter baseline
- Workspace-contained text file browser/editor
- Bounded direct-process terminal
- `.test` local networking planner
- Remote Control Plane configuration, device enrollment and per-machine remote opt-ins

Privileged hosts/CA/Caddy mutations remain behind `vsn-agent network-admin` and require OS elevation rather than silent desktop privilege escalation. Remote terminal, remote file writes and remote external-DB queries default to disabled on each attached machine.

Build:

```bash
npm install
npm run build
npm run tauri build
```

A full Tauri build was not executable in the artifact environment because Rust and npm project dependencies were not installed there.
