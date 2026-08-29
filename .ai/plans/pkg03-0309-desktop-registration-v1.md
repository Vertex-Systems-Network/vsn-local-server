# PKG-03 03.09 — Desktop Registration Lifecycle Plan v1

## Goal

Certify the real Windows shell integration created by the accepted Desktop installers: Start Menu shortcut, optionally selected NSIS Desktop shortcut, WiX Desktop shortcut, executable targeting, application identity metadata where the installer emits it, and uninstall cleanup.

## Acceptance sequence

1. Validate canonical 03.09 authority, planning digests, unchanged accepted Tauri config and ownership manifest.
2. Build the accepted Desktop installers on GitHub-hosted `windows-2025` with locked Node/Rust/Tauri inputs.
3. NSIS current-user lifecycle:
   - run visible interactive installer;
   - complete the normal Start Menu flow;
   - explicitly select the Desktop-shortcut GUI option for positive certification;
   - verify Start Menu and Desktop `.lnk` targets resolve to the installed Desktop executable;
   - record shortcut AppUserModelID when readable and require `dev.vsn.platform` where the installer emits it;
   - run visible uninstall and verify owned shortcuts are removed.
4. MSI/WiX lifecycle:
   - run visible/default MSI install;
   - verify WiX-owned Start Menu and Desktop shortcuts and exact target;
   - require Start Menu `System.AppUserModel.ID=dev.vsn.platform`;
   - uninstall through exact ProductCode and verify owned shortcuts are removed.
5. Verify no CLI/Agent placement, service registration, ACL mutation, silent/passive deployment, signing or updater mutation.
6. Bind evidence to exact source SHA/run/job/artifact and verify zero tracked repository drift.
7. Only after genuine acceptance, reconcile PKG-03 to 9/25 = 36% and cursor 03.10.

## Exit-state graph

After 03.09 is DONE:
- done: 9/25
- percent: 36.0
- cursor: 03.10
- READY: 03.10, 03.13, 03.15
- 03.11/03.12 remain blocked on 03.10
- 03.14 remains blocked on 03.10
