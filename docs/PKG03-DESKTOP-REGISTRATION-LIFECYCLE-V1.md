# PKG-03 Windows Desktop Registration Contract v1

Task: `03.09`
Linear: `ABD-84`

## Owned shell artifacts

03.09 owns certification of installer-created Desktop application shortcuts only.

### NSIS
- Start Menu shortcut: required in the accepted interactive flow.
- Desktop shortcut: positive-path certification requires explicitly selecting the installer GUI option.
- Both shortcut targets must resolve to the installed `VSN Dev Platform.exe`.
- Shortcut AppUserModelID is `dev.vsn.platform` where emitted/readable.
- Uninstall removes the owned shortcuts.

### MSI/WiX
- Start Menu shortcut: required.
- Desktop shortcut: required by the stock WiX shortcut feature in the accepted package.
- Shortcut targets resolve to `VSN Dev Platform.exe`.
- Start Menu `System.AppUserModel.ID` is `dev.vsn.platform`.
- MSI uninstall removes the owned shortcut components.

## Explicit exclusions

- ARP lifecycle: 03.08.
- CLI/Agent placement/PATH: 03.10.
- Service, ACL, repair/rollback, reboot, silent deployment, signing and updater/recovery: later tasks.
- No file-association or deep-link registration is claimed because accepted source config declares none.
- No custom NSIS/WiX template is authorized.

## Fail-closed rules

Evidence must fail if:
- a required shortcut targets outside the accepted install root;
- shortcut target is not the accepted Desktop executable;
- a required shortcut survives uninstall;
- evidence claims undeclared file/deep-link registration;
- CLI/Agent/service/signing/updater behavior appears in 03.09 acceptance.
