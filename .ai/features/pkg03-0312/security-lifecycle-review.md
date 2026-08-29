# PKG-03 03.12 — Security / Lifecycle Review

## Entry invariants

- Canonical PKG-03 remains exactly 25 tasks and `11/25 = 44%`.
- 03.12 is READY with 03.07/03.10 DONE.
- Product identity remains `VSN Dev Platform` version `0.38.1`.
- 03.10 retains sole ownership of `bin/vsn.exe` and `bin/vsn-agent.exe`.
- 03.11 service identity remains `VSN-Agent`, display `VSN Agent`, automatic start, `NT AUTHORITY\LocalService`, runtime `--service-run`.

## Data separation contract

### Class A — immutable install payload
The selected install root contains installer-owned executable/resources only. Mutable runtime state, audit, secrets, and configuration must not be written under that root.

### Class B — machine-shared IPC security
`%PROGRAMDATA%\VSN\security\ipc.key` is shared between the installed client and LocalService Agent. Runtime `vsn-security` remains the secret/ACL authority. Installer integration may prepare/support the parent location only if it preserves the exact SID contract and does not weaken the runtime's final file ACL.

### Class C — process-context mutable state/config
VSN `data_local_dir()` and `config_dir()` are resolved by ProjectDirs for the executing identity. For the Windows service that means LocalService context. Evidence must record these paths from the running installed system and prove they are not under the immutable install root.

## Current-user NSIS negative boundary

A clean current-user package install, before running product components that demand shared machine state, must not create `%PROGRAMDATA%\VSN\security` or mutate machine ACLs. Package installation/uninstallation remains user-scoped. If runtime execution later creates user-context data, it must remain outside the package install root.

## Per-machine NSIS lifecycle

- build/install exact-head per-machine NSIS;
- preserve 03.11 service lifecycle;
- prove the service runs as LocalService;
- exercise a bounded authenticated probe so shared IPC exists;
- verify `%PROGRAMDATA%\VSN\security\ipc.key` exists;
- inspect directory and file ACLs by SID and reject unexpected write grants to LocalService/general users;
- record Agent-resolved mutable data path and config path;
- prove mutable paths are outside `%ProgramFiles%\VSN Dev Platform`;
- uninstall package and verify only task-bounded installer-owned integration cleanup claims.
Comprehensive dirty-data preservation/cleanup remains 03.17.

## MSI/WiX lifecycle

Apply the same separation and ACL assertions to the accepted MSI/WiX per-machine path. The task may use a bounded WiX fragment/feature rather than a full template. Existing 03.11 service actions remain intact.

## Failure policy

Fail closed on:
- mutable path inside install root;
- machine-shared secret outside ProgramData authority;
- ACL inheritance/ACE mismatch that weakens the accepted SID contract;
- current-user package install creating machine security state;
- duplicate Agent/secret ownership;
- any requirement to mutate runtime storage semantics without change control.

## Nonclaims

03.12 does not certify comprehensive dirty-data uninstall preservation (03.17), repair (03.16), rollback/interruption (03.18), running-process coordination (03.19), silent deployment (03.21), signing (03.22), updater/recovery (PKG-04), or deep security certification (PKG-06).
