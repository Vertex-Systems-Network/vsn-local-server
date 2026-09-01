# PKG-03 03.19 — Evidence-Bound Installer Hook Change Control

Date: 2026-08-31
Linear: ABD-94
PR: #149
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Triggering exact head: `741a9b3a10dbc4f2dc7e9b26a28434cab9eb69f9`
Triggering Windows run: `33333493252`
Failure artifact: `9738737126`

## Triggering evidence

The per-machine NSIS lifecycle established the exact installed Desktop, an exact-path/SHA-256 CLI process and a running `VSN-Agent` service before invoking uninstall. The installer then exposed the explicit Tauri running-application block while Desktop/CLI remained alive and ARP/payload remained installed, but `Get-InstalledCoherence` proved the `VSN-Agent` service identity had already disappeared. The frozen 03.19 contract forbids that partial destructive state and the harness assertion is retained unchanged.

## Root cause

The accepted service hook `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh` performs service stop and service uninstall inside `NSIS_HOOK_PREUNINSTALL`.

The exact bundler used by Tauri CLI 2.11.4 is `tauri-bundler 2.9.4`. Its tagged `installer.nsi` executes `NSIS_HOOK_PREUNINSTALL` before its own `CheckIfAppIsRunning` call in the Uninstall section. Therefore the VSN hook can unregister the Agent before Tauri discovers a running Desktop and aborts. This explains the exact observed state: package/ARP preserved by Tauri's block, service already removed by the earlier hook.

The same tagged Tauri `utils.nsh` defines `CheckIfAppIsRunning` using `nsis_tauri_utils::FindProcess`/`FindProcessCurrentUser`, an interactive OK/Cancel block, and the corresponding Tauri process termination path only when the operator explicitly chooses OK (or in silent/passive behavior outside 03.19 scope).

## Authorized minimum mutation

Only this product installer-hook path is authorized:

- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

The hook may prepend Tauri's existing `CheckIfAppIsRunning` macro for:

1. `${MAINBINARYNAME}.exe` / `${PRODUCTNAME}`
2. `vsn.exe` / `VSN CLI`

Both checks MUST occur before any `VSN-Agent` stop/uninstall command. Existing service install/start/stop/uninstall commands, service name/account, payload, ACLs, package identity and all other installer behavior remain unchanged.

No custom process kill, harness pre-kill, service-identity relaxation, silent/passive acceptance, reboot behavior, signing behavior or updater behavior is authorized.

## Acceptance

The exact-head Windows 03.19 workflow must independently prove all frozen requirements, including:

- exact Desktop and CLI identity/SHA-256 binding;
- running Agent identity for machine lanes;
- no harness pre-kill;
- explicit installer coordination or deterministic safe block;
- on safe block, ARP + owned payload + Agent service identity remain coherent;
- operator cleanup only after the block is proven;
- successful retry uninstall after operator cleanup;
- MSI Restart Manager evidence;
- protected firewall/hosts/resolver/trust equality;
- zero tracked repository drift.

The task remains NOT DONE and PR #149 remains non-mergeable until exact-head Windows success evidence is independently inspected.
