# PKG-03 03.09 Lifecycle Review

## Lifecycle under test

03.09 certifies owned Windows shell integration created by the accepted Desktop installers and removed by their corresponding uninstall paths.

### NSIS current-user path
- interactive install only;
- Start Menu shortcut must be observed after selecting/accepting the installer Start Menu step;
- Desktop shortcut is certified only when the interactive desktop-shortcut option is selected;
- shortcut targets must resolve to the installed `VSN Dev Platform.exe`;
- observed shortcut AppUserModelID must match `dev.vsn.platform` where exposed by the shell;
- uninstall must remove task-owned shortcuts and leave no stale task-owned shortcut target.

### MSI/WiX path
- visible/default MSI install only;
- Start Menu shortcut and Desktop shortcut are observed as WiX-owned components;
- Start Menu shortcut application identity must be bound to `dev.vsn.platform`;
- uninstall must remove the WiX-owned shortcuts.

## Nonclaims

03.09 does not own or certify:
- Add/Remove Programs semantics (03.08);
- CLI/Agent placement or PATH discovery (03.10);
- service lifecycle (03.11);
- ACL/data separation (03.12);
- file associations/deep links that are not configured in accepted source;
- silent/passive deployment (03.21);
- signing or updater/recovery.

No product/config mutation is required.
