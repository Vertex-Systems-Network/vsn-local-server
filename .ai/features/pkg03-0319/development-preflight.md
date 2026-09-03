# PKG-03 03.19 Development Preflight

Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.19`
Linear: `ABD-94`

## Dependency/state check

- 03.11 Agent Windows service lifecycle: DONE
- 03.15 installer diagnostics/exit semantics: DONE
- canonical tracker: 15/25 = 60%
- 03.19 status: READY at branch activation
- lane: `runtime`
- sibling branch-local results are non-authoritative until integrated on main
- frozen max parallel implementation tasks: 5

## Initial mutation authority

Planning may change only the 03.19 planning/contract bundle.

After exact planning gates pass, implementation may initially add only:
- `scripts/ci/pkg03-0319-*` validator/process/coordination/evidence helpers;
- `.github/workflows/pkg03-0319-*` exact-head Windows certification;
- canonical projection surfaces only after genuine evidence.

Not initially authorized:
- Tauri/NSIS/WiX template or hook changes;
- product runtime changes or new process-control daemon;
- service identity/account or ACL changes;
- reboot/silent/signing/updater/later-package behavior.

## Fail-closed rule

If exact generated installers cannot safely complete or explicitly block under the frozen running-resource matrix, retain evidence and open minimum-scope change control. Do not pre-kill product processes or weaken acceptance to manufacture success.
