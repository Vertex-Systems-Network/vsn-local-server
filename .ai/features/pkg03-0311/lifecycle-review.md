# PKG-03 03.11 Lifecycle Review

## Lifecycle under test

03.11 certifies installer ownership of the existing production `VSN-Agent` Windows service using the Agent payload already owned and placed by 03.10.

## Frozen service identity

- service name: `VSN-Agent`
- display name: `VSN Agent`
- executable: installed `<install-root>\bin\vsn-agent.exe`
- SCM arguments: `--service-run`
- start type: automatic
- service account: `NT AUTHORITY\LocalService`
- service type: own-process
- accepted control: STOP
- health endpoint: authenticated local Agent IPC as exercised through the installed CLI

These values are accepted inputs from the current Agent implementation. 03.11 does not rename the service or create a competing service runtime.

## Current-user NSIS negative boundary

A current-user NSIS lifecycle must:
- install the 03.10 Agent payload as already certified;
- leave `VSN-Agent` absent from SCM;
- never invoke machine service install/start/stop/remove actions;
- uninstall without mutating a machine service.

## Per-machine NSIS lifecycle

A visible elevated per-machine NSIS lifecycle must:
1. install the owned Agent payload;
2. register `VSN-Agent` only after the installed Agent executable exists;
3. configure exact image path `<install-root>\bin\vsn-agent.exe --service-run`;
4. configure automatic start under `NT AUTHORITY\LocalService`;
5. start the service and reach SCM `RUNNING`;
6. pass an installed `vsn.exe ping` health probe against the running service;
7. support bounded stop then start and return to healthy `RUNNING`;
8. on uninstall, stop and remove the service before the Agent payload is deleted;
9. leave SCM reporting service-not-found and leave the owned Agent payload removed.

## MSI/WiX lifecycle

A visible/default per-machine MSI lifecycle must prove the same service identity, start, health, stop/start and uninstall removal contract as per-machine NSIS.

The MSI integration must not install a second copy of `bin/vsn-agent.exe`. Any WiX fragment must consume the already-owned 03.10 payload path and compile/link through supported Tauri WiX extension points.

## Failure and cleanup boundary

03.11 requires deterministic failure when service registration/start cannot complete and requires best-effort task-owned cleanup in the certification harness. It does not claim transactional installer rollback or interrupted-install recovery; those belong to 03.18.

## Nonclaims

03.11 does not own or certify:
- ACL/state/config/user-data separation (03.12);
- firewall, hosts, resolver or trust-store mutation (03.13);
- payload tamper/repair semantics (03.14/03.16);
- installer diagnostics/exit-code policy beyond evidence needed for this lifecycle (03.15);
- running-process/Restart Manager coordination (03.19);
- reboot semantics (03.20);
- silent/passive deployment (03.21);
- signing, updater or recovery.

No custom full NSIS/WiX installer template is authorized.
