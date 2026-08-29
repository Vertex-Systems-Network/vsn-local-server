# PKG-03 03.15 Research — Installer diagnostics and exit semantics

Reviewed: 2026-08-27
Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Linear: `ABD-90`
Change required: **false**

## Canonical findings

- 03.06, 03.07 and 03.08 are canonically DONE, so 03.15 is dependency-satisfied and READY on the independent `diagnostics` lane.
- Microsoft documents `msiexec` as returning Windows system error codes and supports explicit install/uninstall logging through `/L` switches. `/L*V <path>` captures all standard logging plus verbose output when the destination directory already exists.
- Windows Installer system error `1602` (`ERROR_INSTALL_USEREXIT`) means the user cancelled installation; successful `msiexec` operations return `0`. Reboot-required codes belong to 03.20 and must not be claimed here.
- NSIS documents native process error levels: `0` normal execution, `1` installation aborted by the user, and `2` installation aborted by script. This gives a deterministic setup-cancellation contract without inventing a custom code.
- NSIS persistent `LogSet` support requires an NSIS build compiled with `NSIS_CONFIG_LOG`, which is not enabled by default. 03.15 must not replace/recompile the repository-local Tauri/NSIS toolchain merely to imitate MSI logging.
- NSIS uninstallers self-copy to a temporary directory; NSIS explicitly warns that the error level set by the inner uninstaller is not normally available to the original executing process. Therefore 03.15 must not claim an uninstaller-cancellation code that stock NSIS cannot expose reliably.
- Tauri 2 permits NSIS installer hooks and custom templates, but 03.15 does not require either to certify the accepted stock installer behavior. Custom NSIS template/logging-engine work would be a material product mutation and is not authorized by this task.
- 03.21 owns silent/passive deployment, 03.18 owns rollback/interrupted recovery, 03.20 owns reboot semantics, and 03.16 owns repair/reinstall. Those boundaries must remain nonclaims in 03.15 evidence.

Official references:
- https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec
- https://learn.microsoft.com/en-us/windows/win32/msi/command-line-options
- https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--1300-1699-
- https://v2.tauri.app/distribute/windows-installer/
- https://nsis.sourceforge.io/Docs/AppendixD.html
- https://nsis.sourceforge.io/Reference/LogSet

## Frozen diagnostics model

1. **MSI/WiX success path**
   - run genuine visible install and uninstall with `/L*V` to task-owned evidence paths;
   - require process exit `0`;
   - require non-empty logs and bind their SHA-256 to exact-head evidence.
2. **MSI/WiX cancellation path**
   - start genuine visible install against a clean machine state;
   - cancel from the installer UI before committed installation;
   - require `msiexec` exit `1602`;
   - require a non-empty verbose log and verify no accepted install root/ARP state was created.
3. **NSIS success path**
   - reuse accepted interactive current-user and per-machine package semantics;
   - require setup process exit `0`;
   - capture visible UI observations/actions as operator diagnostics.
4. **NSIS cancellation path**
   - cancel the genuine setup UI before file-copy commit;
   - require setup process exit `1`;
   - verify no accepted install root/shortcut/ARP residue.
5. **NSIS uninstall cancellation**
   - may be observed diagnostically, but no deterministic parent-process exit-code claim is allowed because of stock NSIS self-copy semantics.

No task-owned product mutation is required. `change_required=false`
