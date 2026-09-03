; PKG-03 03.11 — machine-service lifecycle hook.
; The current-user installer is intentionally a compile-time no-op for SCM mutation.
;
; PKG-03 03.19 bounded change control: Tauri bundler 2.9.4 invokes this
; pre-uninstall hook before its stock running-application guard. For per-machine
; uninstall, guard the exact Desktop and CLI process names before any VSN-Agent
; SCM mutation. Reuse Tauri's own CheckIfAppIsRunning macro; do not add custom
; process termination and do not alter the accepted service lifecycle semantics.

!macro NSIS_HOOK_POSTINSTALL
  !if "${INSTALLMODE}" == "perMachine"
    DetailPrint "Checking VSN Agent Windows service registration"
    nsExec::ExecToLog '"$SYSDIR\sc.exe" query VSN-Agent'
    Pop $0
    StrCmp $0 "0" pkg0311_service_install_ok

    DetailPrint "Registering VSN Agent Windows service"
    nsExec::ExecToLog '"$INSTDIR\bin\vsn-agent.exe" service install'
    Pop $0
    StrCmp $0 "0" pkg0311_service_install_ok
    Abort "VSN Agent service installation failed with exit code $0."

    pkg0311_service_install_ok:
    DetailPrint "Starting VSN Agent Windows service"
    nsExec::ExecToLog '"$INSTDIR\bin\vsn-agent.exe" service start'
    Pop $0
    StrCmp $0 "0" pkg0311_service_start_ok
    Abort "VSN Agent service start failed with exit code $0."

    pkg0311_service_start_ok:
  !endif
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !if "${INSTALLMODE}" == "perMachine"
    DetailPrint "Checking VSN running resources before Agent service mutation"
    !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
    !insertmacro CheckIfAppIsRunning "vsn.exe" "VSN CLI"

    ; The uninstaller's completion predicate (payload + ARP removal) is finalized
    ; only when the NSIS process exits. Per-machine UI automation cannot reliably
    ; invoke the elevated finish-page Close control, so close automatically after
    ; a successful uninstall section while preserving Abort failure visibility.
    SetAutoClose true

    ; The Agent service CLI intentionally maps every failed sc.exe invocation to
    ; the process-level ExitCode::FAILURE (1), so it cannot preserve native SCM
    ; idempotence codes such as ERROR_SERVICE_NOT_ACTIVE (1062). The frozen 03.16
    ; lifecycle already quiesces VSN-Agent before uninstall, therefore stop the
    ; same accepted service directly through SCM so the native result remains
    ; classifiable without changing Agent Rust or weakening fail-closed behavior.
    DetailPrint "Stopping VSN Agent Windows service through SCM"
    nsExec::ExecToLog '"$SYSDIR\sc.exe" stop VSN-Agent'
    Pop $0
    StrCmp $0 "0" pkg0311_service_stop_ok
    StrCmp $0 "1062" pkg0311_service_stop_ok
    Abort "VSN Agent service stop failed with exit code $0."

    pkg0311_service_stop_ok:
    ; Remove the stopped service directly through SCM. DeleteService/sc.exe delete
    ; is a mark-for-deletion operation: the SCM removes the record after the last
    ; open service handle closes. Waiting inside this uninstall section for the
    ; record to become unqueryable can self-block completion and, on Abort, leaves
    ; NSIS in its failed InstFiles state with only Cancel enabled. Therefore a
    ; successful delete request, an already-absent service, or a service already
    ; marked for deletion permits the section to continue. The frozen 03.16
    ; post-process acceptance still requires the service, payload and ARP
    ; registration all to be absent after process exit.
    DetailPrint "Removing VSN Agent Windows service through SCM"
    nsExec::ExecToLog '"$SYSDIR\sc.exe" delete VSN-Agent'
    Pop $0
    StrCmp $0 "0" pkg0311_service_remove_ok
    StrCmp $0 "1060" pkg0311_service_remove_ok
    ; ERROR_SERVICE_MARKED_FOR_DELETE (1072) means a prior DeleteService request
    ; already placed this same service record into the pending-deletion state.
    ; Treat only that specific native state as idempotent delete success.
    StrCmp $0 "1072" pkg0311_service_remove_ok
    Abort "VSN Agent service removal failed with exit code $0."

    pkg0311_service_remove_ok:
  !endif
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !if "${INSTALLMODE}" == "currentUser"
    ; Tauri's WiX 3 template searches the HKCU vendor/product default value before
    ; falling back to its per-machine Program Files directory. The current-user
    ; NSIS installer writes $INSTDIR to that default value, while its normal
    ; uninstall preserves it whenever "Delete the application data" is left off.
    ; Once the payload is genuinely uninstalled, that unnamed value is stale
    ; installer-location metadata rather than application data. Remove only that
    ; value so a subsequent per-machine MSI cannot inherit the old LocalAppData
    ; path. Preserve every named value (including Installer Language) and all user
    ; application data; do not perform any certification-side cleanup.
    DeleteRegValue HKCU "Software\${MANUFACTURER}\${PRODUCTNAME}" ""
  !endif
!macroend
