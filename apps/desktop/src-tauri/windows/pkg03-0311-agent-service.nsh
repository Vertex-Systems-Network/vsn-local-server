; PKG-03 03.11 — machine-service lifecycle hook.
; The current-user installer is intentionally a compile-time no-op for SCM mutation.

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
    ; The uninstaller's completion predicate (payload + ARP removal) is finalized
    ; only when the NSIS process exits. Per-machine UI automation cannot reliably
    ; invoke the elevated finish-page Close control, so close automatically after
    ; a successful uninstall section while preserving Abort failure visibility.
    SetAutoClose true

    DetailPrint "Stopping VSN Agent Windows service"
    nsExec::ExecToLog '"$INSTDIR\bin\vsn-agent.exe" service stop'
    Pop $0
    StrCmp $0 "0" pkg0311_service_stop_ok
    Abort "VSN Agent service stop failed with exit code $0."

    pkg0311_service_stop_ok:
    ; Amendment 002: remove the service directly through SCM instead of routing
    ; deletion through the installed Agent executable. ERROR_SERVICE_DOES_NOT_EXIST
    ; (1060) is the only idempotent-success exception; all other failures abort.
    DetailPrint "Removing VSN Agent Windows service through SCM"
    nsExec::ExecToLog '"$SYSDIR\sc.exe" delete VSN-Agent'
    Pop $0
    StrCmp $0 "0" pkg0311_service_remove_verify
    StrCmp $0 "1060" pkg0311_service_remove_ok
    Abort "VSN Agent service removal failed with exit code $0."

    ; A successful DeleteService can transiently leave the service queryable while
    ; SCM closes outstanding handles. Do not permit payload/ARP cleanup to proceed
    ; until the service is genuinely no longer queryable. Keep this bounded and
    ; fail closed without changing the outer 03.16 acceptance timeout.
    pkg0311_service_remove_verify:
    StrCpy $1 0

    pkg0311_service_remove_verify_loop:
    nsExec::ExecToLog '"$SYSDIR\sc.exe" query VSN-Agent'
    Pop $0
    StrCmp $0 "1060" pkg0311_service_remove_ok
    StrCmp $0 "0" pkg0311_service_remove_still_present
    Abort "VSN Agent service removal verification failed with exit code $0."

    pkg0311_service_remove_still_present:
    IntOp $1 $1 + 1
    IntCmp $1 40 pkg0311_service_remove_wait pkg0311_service_remove_timeout pkg0311_service_remove_timeout

    pkg0311_service_remove_wait:
    Sleep 250
    Goto pkg0311_service_remove_verify_loop

    pkg0311_service_remove_timeout:
    Abort "VSN Agent service remained queryable after removal."

    pkg0311_service_remove_ok:
  !endif
!macroend
