# VSN Platform Batch 0.3

This batch expands P2 into a materially usable local core while laying cryptographic foundations for remote control.

## Added
- P1 config/event/CI baselines
- P2 Linux/macOS deployment templates and Windows least-privilege service adjustment
- P3 process/port/service/health/log core
- P4 runtime detection baseline
- P5 networking/domain planning baseline
- P6 project detection baseline
- P7 database driver SDK
- P8 schema-driven Database Studio UI model
- P9 multi-model database workspaces
- P11 expanded CLI
- P12 device-enrollment + signed remote-command verifier core (no internet listener)
- P19 permission baseline refinements
- P24 Docker/Podman detection baseline
- P25 extension/provider manifest contract

## Security corrections in this batch
- Windows Agent service account changed from LocalSystem to LocalService.
- Baseline local principal no longer receives `network.manage`.
- OS service mutations are restricted to `VSN-*` service names.
- Log reads are restricted to VSN-owned data paths.
- Existing Windows IPC ACLs are not recalculated under the service SID on each read.
- Device enrollment verification checks that device ID is derived from the supplied public key.
- Remote commands are device-bound, signed, short-lived and replay-protected before any future transport is enabled.

## Validation available in this artifact environment
- Cargo TOML parsing
- JSON/JSON-Schema parsing and provider validation
- CI YAML parsing
- macOS plist parsing
- shell script syntax
- Node contract parsing

Rust compilation is not claimed because `cargo`/`rustc` are not installed in this artifact environment. Run the included Windows smoke script or CI workflow for compiler-backed verification.
