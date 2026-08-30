# PKG-03 03.18 Development Preflight

Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.18`
Linear: `ABD-93`

## Dependency/state check

- 03.11 Agent Windows service lifecycle: DONE
- 03.12 installer ACL/state separation: DONE
- 03.14 payload integrity detection: DONE
- 03.15 installer diagnostics/exit semantics: DONE
- canonical tracker: 15/25 = 60%
- 03.18 status: READY at branch activation
- lane: `recovery`
- active/READY sibling results may not be consumed until canonically integrated on `main`
- frozen max parallel implementation tasks: 5

## Initial mutation authority

Planning may change only the 03.18 planning/contract bundle.

After exact planning gates pass, implementation may initially add only:
- `scripts/ci/pkg03-0318-*` validator/failure/recovery/evidence helpers;
- `.github/workflows/pkg03-0318-*` exact-head Windows certification;
- canonical projection surfaces only after genuine accepted evidence.

Not initially authorized:
- Tauri config or package identity changes;
- NSIS/WiX template/hook mutation;
- product runtime/updater/recovery daemon changes;
- service identity/account or ACL policy changes;
- live-running VSN process coordination;
- reboot/silent/signing/updater/later-package behavior.

## Fail-closed rule

If stock exact-head packages cannot recover from the frozen forced-failure/interruption probes without partial or duplicate state, retain evidence and open minimum-scope change control. Do not weaken the failure/recovery contract.
