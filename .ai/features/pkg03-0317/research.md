# PKG-03 03.17 Research — Uninstall cleanup and user-data preservation

Reviewed: 2026-08-30
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Linear: `ABD-92`
Change required: **false (certification-first)**

## Canonical findings

- Canonical PKG-03 is `15/25 = 60%`; `03.17` is READY because dependencies `03.11`, `03.12`, and `03.13` are canonically DONE.
- 03.05/03.10 define installer-owned payload under the selected install root. 03.09 owns shortcut/application-registration lifecycle. 03.11 owns Agent service removal. 03.12 freezes machine security and process-context data/config separation. 03.13 freezes firewall/hosts/resolver/trust-store non-mutation.
- 03.12 intentionally did not claim the comprehensive dirty-user-data uninstall matrix. It established that mutable `ProjectDirs("dev","VSN","VSN Platform")` data/config remain outside the install root and that uninstall must not delete arbitrary user/project data.
- Tauri 2 NSIS exposes `NSIS_HOOK_PREUNINSTALL` and `NSIS_HOOK_POSTUNINSTALL` when a bounded extension is actually required. A full custom NSIS template is not justified by current evidence.
- Windows Installer's `RemoveFiles` action removes files installed by the package/component model and can remove explicitly authored miscellaneous files. Empty folders can be removed through the package tables/actions. This reinforces a narrow ownership-driven cleanup model rather than recursive deletion of mutable data trees.
- Microsoft Windows Installer guidance treats clean uninstall as a first-class requirement, but preserving intentional user customization/data requires an explicit component/data policy rather than deleting broad directories.
- Current accepted behavior already removes the machine Agent service and package payload; 03.17 should first certify exact stock uninstall behavior against dirty-state fixtures before any installer hook/fragment mutation.
- The machine IPC security material under `%PROGRAMDATA%\VSN\security` is product security state, not user workspace/config data. 03.17 will classify it separately from preserved user data and require cleanup only if the exact accepted service/security ownership contract proves it is exclusively product-owned and safe to remove. If that classification is not provable from exact-head evidence, the task fails closed for bounded change control instead of deleting it heuristically.
- 03.18 owns failure rollback/interrupted install recovery; 03.19 owns live-running process coordination; 03.20 owns reboot semantics; 03.21 owns silent deployment. None are implied by a successful ordinary uninstall.

Official references:
- https://v2.tauri.app/distribute/windows-installer/
- https://learn.microsoft.com/en-us/windows/win32/msi/removefiles-action
- https://learn.microsoft.com/en-us/windows/win32/msi/removefile-table
- https://learn.microsoft.com/en-us/windows/win32/msi/removefolders-action
- https://learn.microsoft.com/en-us/windows/win32/msi/removing-stranded-files
- https://learn.microsoft.com/en-us/windows/win32/msi/windows-installer-best-practices

## Frozen cleanup model

1. **Ownership-first cleanup**
   - build/install exact current-user NSIS, per-machine NSIS and MSI/WiX candidates;
   - enumerate accepted package-owned files, shortcuts, ARP/product registration and service identity before uninstall;
   - uninstall through the genuine format-specific path;
   - require package-owned artifacts and registrations to be absent afterward.

2. **Dirty user-data preservation**
   - create deterministic marker files in resolved mutable data/config roots and a dedicated user workspace/project fixture outside the install root;
   - record exact bytes, SHA-256, ACL/security descriptor where applicable, and canonical path;
   - run uninstall;
   - require markers and workspace/project content to remain byte-identical and path-stable.

3. **Boundary/safety proof**
   - no recursive cleanup outside explicitly owned paths;
   - no deletion through junction/reparse-point escape from an owned directory into a preserved fixture;
   - no mutation of firewall, hosts, resolver or trust store;
   - current-user uninstall must not create or delete machine-wide security/service state it did not own.

4. **Machine security classification**
   - record `%PROGRAMDATA%\VSN\security` ownership/state separately;
   - remove it only when the task can prove the directory/key are exclusively product-owned and no preserved user/project data lives beneath it;
   - never broaden ACLs or move secrets as part of cleanup.

No product/config/template/toolchain mutation is authorized by this planning conclusion. `change_required=false` means certification-first. A real exact-head cleanup or preservation failure may justify a separate minimum-scope change-control amendment, but acceptance may not be weakened.
