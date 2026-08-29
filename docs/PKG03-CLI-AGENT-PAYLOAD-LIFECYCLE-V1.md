# PKG-03 CLI/Agent Payload Placement & Discovery Contract v1

Task: `03.10`
Linear: `ABD-85`

## Owned binaries

Exactly two payloads are added by this task:
- `${INSTALL_ROOT}/bin/vsn.exe`
- `${INSTALL_ROOT}/bin/vsn-agent.exe`

Their source packages are respectively `vsn` and `vsn-agent`, both version `0.38.1`.

## Packaging

- Release binaries are built from the locked workspace graph.
- The staged inputs are hash-bound to those release outputs.
- Tauri resource mapping is used to preserve the frozen `bin/*.exe` destination names.
- No target-triple suffix is exposed in the installed file names.
- No undeclared executable, updater helper or runtime payload is added.

## Discovery

The canonical discovery rule is:
`<accepted install root> + \bin\<binary-name>`

03.10 does not require or authorize PATH/environment mutation.

## Launch proof

Certification must prove the installed files are executable Windows PE payloads and can be started by their exact installed absolute paths with bounded termination/exit handling. A launch probe must not be treated as Agent service certification.

## Install roots

- NSIS current-user: `%LOCALAPPDATA%\VSN Dev Platform`
- MSI/WiX per-machine: `%ProgramFiles%\VSN Dev Platform`

## Cleanup

After the corresponding accepted uninstall path:
- `bin/vsn.exe` must be absent;
- `bin/vsn-agent.exe` must be absent;
- task-created staging material must not escape the repository/build workspace.

## Explicit nonclaims

No service registration, automatic service start, PATH mutation, ACL policy, silent deployment, signing, updater or recovery behavior is certified here.
