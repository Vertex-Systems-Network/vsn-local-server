# PKG-03 03.19 Research — Running-process and Restart Manager/service coordination

Reviewed: 2026-08-30
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Linear: `ABD-94`
Change required: **false (certification-first)**

## Canonical findings

- Canonical PKG-03 is `15/25 = 60%`; `03.19` is READY because `03.11` and `03.15` are canonically DONE.
- 03.11 freezes the `VSN-Agent` Windows service identity/lifecycle and explicitly hands final live-running install/uninstall coordination to 03.19. 03.15 supplies installer logging/exit-code/operator diagnostics required to distinguish safe coordination from hangs or opaque failures.
- Windows Installer integrates with Restart Manager to identify applications/services using files that must be updated. Restart Manager can request shutdown and restart of registered applications and services, reducing files-in-use failures and reboots.
- MSI exposes Restart Manager policy/properties such as `MSIRESTARTMANAGERCONTROL`; evidence must confirm the generated package is actually using the expected path rather than assuming default behavior.
- Restart Manager distinguishes applications/services and provides reason/status data for resources in use. 03.19 should bind the exact running PIDs/service state and installer observations before and after the operation.
- Tauri's NSIS output does not document an equivalent automatic Restart Manager contract. Generated NSIS behavior must be tested with the Desktop and accepted CLI/Agent running states. If stock behavior hangs, force-kills, corrupts state, or silently proceeds unsafely, the task fails closed for bounded change control.
- The certification harness must not manufacture a pass by killing VSN processes before the installer acts. Harness-initiated termination is allowed only as post-failure cleanup and must be recorded as such.
- Reboot-required and no-restart policy belong to 03.20. 03.19 may record reboot-related observations but cannot certify them.

Official references:
- https://learn.microsoft.com/en-us/windows/win32/rstmgr/about-restart-manager
- https://learn.microsoft.com/en-us/windows/win32/msi/using-windows-installer-with-restart-manager
- https://learn.microsoft.com/en-us/windows/win32/msi/msirestartmanagercontrol
- https://learn.microsoft.com/en-us/windows/win32/api/restartmanager/
- https://v2.tauri.app/distribute/windows-installer/

## Frozen handling model

1. Install exact candidate and establish one accepted installed identity.
2. Start the Desktop and a deterministic long-running CLI process using already accepted product capabilities; keep `VSN-Agent` running for per-machine cases.
3. Capture PID/image/hash/service identity and verify each running process belongs to the exact installed payload.
4. Invoke format-specific reinstall/uninstall while those resources are in use.
5. Require one of two explicit safe outcomes per tested operation:
   - coordinated completion: installer requests/achieves bounded shutdown/service quiescence, operation succeeds, state is coherent, and required services/apps are restored only when contractually appropriate; or
   - deterministic non-destructive block: installer exits/prompts with evidence-bound diagnostics before destructive mutation, leaves installed state coherent, and allows a later retry after operator close.
6. Forbid silent force termination, indefinite hang, partial package state, duplicate identity, or harness pre-kill masquerading as installer coordination.
7. MSI evidence must retain verbose logs and Restart Manager-related observations; NSIS evidence must retain UI/process/action observations.

No product/config/template/toolchain mutation is authorized by this planning conclusion.
