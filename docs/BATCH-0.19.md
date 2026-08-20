# VSN Batch 0.19

## Completion-sprint scope

- **Runtime lifecycle:** staged extraction, executable verification, destination backup/swap, rollback on failed replace, tombstone uninstall, atomic registry/shim writes and `runtime repair`.
- **OS `.test` resolver:** privileged Windows NRPT, macOS `/etc/resolver/test`, and Linux/systemd-resolved apply/status/remove paths. DNS target remains loopback port 53.
- **Vault lifecycle:** encrypted pre-rotation recovery snapshots, key history, explicit generation restore and confirmed retirement of non-current recovery keys.
- **Executable extensions:** signed installed manifest is re-verified before Linux Bubblewrap execution; namespaces/mount/network/workspace access are policy-derived, direct argv only, bounded runtime/output. Non-Linux executable sandbox remains fail-closed.
- **AI evaluation:** deterministic evaluation cases verify expected tool calls, mutation constraints and the unrestricted-shell invariant.
- **Marketplace channels:** signed entries can declare release channels; update resolution never crosses the requested channel and still excludes revoked releases.
- **Terminal recovery:** durable PTY metadata identifies orphaned sessions and their scrollback after an Agent restart without automatically replaying side-effecting shell commands.
- **Release evidence:** successful release/nightly workflow jobs emit mergeable evidence records instead of requiring purely manual ledger updates.

## Safety boundaries retained

No arbitrary shell execution was added to AI/runtime/marketplace paths. Executable extensions are unavailable when a supported sandbox backend is unavailable. PTY commands are not automatically reconstructed after Agent restart. Release evidence remains pending until external jobs actually pass.
- **Containers:** bounded image pull/build plus container/image/volume/network removal; build context and optional Dockerfile remain inside configured workspace roots, and subprocess runtime/output are bounded.
