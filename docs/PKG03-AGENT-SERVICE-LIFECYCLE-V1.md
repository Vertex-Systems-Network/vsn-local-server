# PKG-03 Agent Service Lifecycle Contract v1

## Purpose

This contract defines the Windows installer-owned lifecycle for the existing VSN Agent service. It does not redefine the Agent runtime.

## Identity and configuration

| Field | Required value |
| --- | --- |
| Service name | `VSN-Agent` |
| Display name | `VSN Agent` |
| Executable | `<install-root>\bin\vsn-agent.exe` |
| Arguments | `--service-run` |
| Start type | Automatic |
| Account | `NT AUTHORITY\LocalService` |
| Service model | Own process |
| Stop control | Supported |

The installer must not register a different alias or legacy `VSNAgent` service.

## Scope behavior

### Current-user NSIS
The installer must not create, start, stop, replace or remove `VSN-Agent`.

### Per-machine NSIS and MSI/WiX
After successful install:
- exactly one `VSN-Agent` SCM registration exists;
- its image path resolves to the owned 03.10 Agent payload plus `--service-run`;
- service account/start type/display name match this contract;
- service reaches `RUNNING`;
- installed CLI health (`vsn.exe ping`) succeeds.

Before successful uninstall completes:
- the service is stopped;
- the service registration is removed;
- the Agent payload can then be deleted by the accepted installer ownership lifecycle.

After uninstall:
- SCM query for `VSN-Agent` reports service-not-found;
- the owned Agent payload is absent according to 03.10 cleanup semantics.

## Ownership rule

03.11 owns service registration/control metadata only. The Agent executable remains owned once, by the 03.10 payload mapping. MSI authoring must not add a second `File`/Component copy of `bin/vsn-agent.exe`.

## Evidence minimum

Exact-head GitHub-hosted Windows evidence must record:
- source SHA, workflow run/job/artifact identifiers;
- NSIS/MSI hashes;
- service `qc/query` observations after install;
- exact image path, account, start type and display name;
- health probe exit/result;
- stop/start bounded transitions;
- current-user non-registration result;
- uninstall service-not-found and payload-removal result;
- zero tracked repository drift.

## Explicit nonclaims

This contract does not certify ACLs, firewall/hosts/DNS/trust changes, repair, rollback, restart-manager coordination, reboot policy, unattended deployment, signing, updater or recovery behavior.
