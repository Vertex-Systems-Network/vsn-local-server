; PKG-03 03.11 — machine-service lifecycle hook.
; The current-user installer is intentionally a compile-time no-op for SCM mutation.

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
