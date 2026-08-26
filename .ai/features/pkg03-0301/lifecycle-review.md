# PKG-03 03.01 — Lifecycle Review

Status: complete for task planning/architecture authority.

## Architecture

- Packaging controller: Tauri v2 Desktop bundle configuration plus repository-owned Windows packaging/certification scripts.
- Supported package families in PKG-03: NSIS setup executable and MSI/WiX.
- Product identity source remains the canonical Desktop/Tauri config; task 03.03 owns publisher/upgrade metadata completion.
- Owned runtime payload classes: Desktop application, `vsn` CLI, `vsn-agent`, installer-created registrations/shortcuts/service metadata. Exact path/file enumeration is deferred to 03.05.
- Mutable user/project/config/state data is not installer-owned by default and must not be silently deleted.

## Data flow

Build inputs -> locked repository source/config -> Windows bundle builder -> NSIS/MSI artifacts -> task-specific evidence. No production signing private key material is written to the repository or evidence.

## Security

- No firewall, hosts, resolver or trust-store side effects.
- Per-machine privilege is explicit and deferred to 03.04 acceptance.
- Signing integration is deferred to 03.22; secrets remain external handles.
- PKG-04 update/apply/rollback logic is out of scope.

## Design

Installer UX must support Windows-native interactive installation while preserving a future deterministic silent/enterprise path. Exact UI/flags are accepted by later tasks.

## QA

03.01 proves only that architecture, format, identity-source and ownership boundaries are explicit, internally consistent with the frozen 25-task DAG, and bound to exact source SHA. It does not claim installation success.

## Performance

No installer performance budget is introduced by this architecture-only task. Later bundle/install tasks must record artifact size and install/runtime timing where relevant.

## Development authorization

Development for 03.01 is limited to governance/architecture/evidence assets and canonical package activation state after task evidence succeeds. Product installer implementation begins only in dependency-ready later tasks.
