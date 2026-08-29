# PKG-03 03.10 Lifecycle Review

## Lifecycle under test

03.10 certifies the two non-Desktop executable payloads declared by the frozen ownership manifest.

### Build/stage contract
- build `vsn` and `vsn-agent` from the locked Cargo graph;
- stage only those two exact release executables for bundling;
- bind staged hashes to the source build outputs before installer build;
- no updater helper or undeclared executable may be staged.

### NSIS current-user placement
- install through the accepted current-user interactive NSIS path;
- require `%LOCALAPPDATA%\VSN Dev Platform\bin\vsn.exe`;
- require `%LOCALAPPDATA%\VSN Dev Platform\bin\vsn-agent.exe`;
- verify installed hashes match the staged release binaries;
- directly launch the installed binaries using bounded probes;
- uninstall and require both owned `bin` executables to be removed.

### MSI/WiX per-machine placement
- install through the accepted visible/default MSI path;
- require `%ProgramFiles%\VSN Dev Platform\bin\vsn.exe`;
- require `%ProgramFiles%\VSN Dev Platform\bin\vsn-agent.exe`;
- verify installed hashes match the staged release binaries;
- directly launch the installed binaries using bounded probes;
- uninstall and require both owned `bin` executables to be removed.

## Discovery contract

03.10 proves deterministic discovery from the application install root and exact `bin` relative paths. It does not add a machine/user PATH entry unless a separately reviewed contract explicitly authorizes that mutation.

## Nonclaims

03.10 does not own or certify:
- Agent Windows service registration/start/health/removal (03.11);
- ACL/state/config/user-data separation (03.12);
- repair/tamper behavior (03.14);
- installer diagnostics/exit-code policy (03.15);
- silent deployment (03.21);
- signing, updater or recovery.

No custom NSIS/WiX template is planned.
