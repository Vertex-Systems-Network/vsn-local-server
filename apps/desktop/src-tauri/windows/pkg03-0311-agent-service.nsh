; PKG-03 03.11 — machine-service lifecycle hook.
; The current-user installer is intentionally a compile-time no-op for SCM mutation.
;
; PKG-03 03.19 bounded change control (2026-08-31): Tauri bundler 2.9.4
; executes NSIS_HOOK_PREUNINSTALL before its own CheckIfAppIsRunning guard.
; Guard the exact Desktop and CLI process names here before any VSN-Agent SCM
; mutation so a deterministic running-resource block cannot leave ARP/payload
; installed while the service identity has already been removed. Reuse Tauri's
; own process guard; do not add custom kill logic or alter service identity.

!macro NSIS_HOOK_POSTINSTALL
  !if "${INSTALLMODE}" == "perMachine"
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

    DetailPrint "Stopping VSN Agent Windows service"
    nsExec::ExecToLog '"$INSTDIR\bin\vsn-agent.exe" service stop'
    Pop $0
    StrCmp $0 "0" pkg0311_service_stop_ok
    Abort "VSN Agent service stop failed with exit code $0."

    pkg0311_service_stop_ok:
    DetailPrint "Removing VSN Agent Windows service"
    nsExec::ExecToLog '"$INSTDIR\bin\vsn-agent.exe" service uninstall'
    Pop $0
    StrCmp $0 "0" pkg0311_service_remove_ok
    Abort "VSN Agent service removal failed with exit code $0."

    pkg0311_service_remove_ok:
  !endif
!macroend
