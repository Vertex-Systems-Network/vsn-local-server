# PKG-03 03.11 — VSN Agent Windows Service Install Lifecycle Plan v1

## Goal

Integrate the already-accepted `VSN-Agent` Windows service runtime into elevated/per-machine NSIS and MSI/WiX installers, prove start and authenticated health, prove deterministic removal before payload deletion, and prove current-user NSIS does not mutate the machine service.

## Acceptance sequence

1. Validate canonical 03.11 authority, parent-plan digest, exact canonical base, and dependencies 03.07 + 03.10.
2. Validate accepted service invariants in the current Agent source: `VSN-Agent`, `VSN Agent`, `--service-run`, auto-start, `NT AUTHORITY\LocalService`, STOP handling and existing service-management commands.
3. Validate accepted 03.10 payload mapping still places exactly one Agent at `bin/vsn-agent.exe`.
4. Add only supported stock-template extension wiring:
   - NSIS `installerHooks` to a task-owned `.nsh`;
   - WiX `fragmentPaths` plus the minimum explicit fragment reference required for the task-owned `.wxs`.
5. NSIS hook contract:
   - compile-time no-op for `currentUser`;
   - for `perMachine`, post-install invokes the installed Agent's existing service install then start interface and fails the installer step on non-zero result;
   - pre-uninstall performs bounded stop then service removal while the installed Agent executable still exists;
   - no full NSIS template fork.
6. WiX contract:
   - use a task-owned fragment that operates on the already-installed Agent path;
   - do not declare another `File` for `bin/vsn-agent.exe`;
   - prefer standard WiX service tables if they can reference the existing Tauri-owned file without duplicate ownership; otherwise use an elevated, synchronous task-owned custom action that invokes the installed Agent's existing service interface;
   - if the stock Tauri template cannot link this fragment without unrelated registry/file/component mutations, stop and require change control rather than forking the full WiX template.
7. Build exact-head current-user NSIS, per-machine NSIS and MSI/WiX with locked Node/Rust/Tauri inputs.
8. Current-user NSIS negative lifecycle:
   - install;
   - prove Agent payload exists;
   - prove `VSN-Agent` remains absent;
   - uninstall;
   - prove service still absent.
9. Per-machine NSIS lifecycle:
   - visible install;
   - verify SCM name/display name/start/account/image path exactly;
   - verify service reaches `RUNNING`;
   - run installed `vsn.exe ping` and require exit 0;
   - stop/start with bounded state waits and require health again;
   - uninstall and verify service removal plus 03.10 Agent payload cleanup.
10. MSI/WiX lifecycle: repeat the same service configuration, health, stop/start and uninstall-removal assertions.
11. Verify no second Agent file ownership and no tracked repository drift.
12. Bind evidence to exact source SHA/run/job/artifact and capture installer hashes, service configuration observations and lifecycle results.
13. Only after genuine acceptance, reconcile live canonical state. 03.11 completion must update tracker/master/README together; never assume concurrent 03.12–03.15 results.

## Implementation decision gate

The NSIS mechanism is frozen to Tauri's supported installer-hook extension point.

The MSI mechanism is frozen by behavior and ownership, not by an unverified XML trick. A task-owned WiX fragment must first prove on Tauri CLI 2.11.4 that it can control the existing installed Agent without duplicating the 03.10 file Component. A successful compile alone is insufficient; exact install/start/health/uninstall evidence is required.

## Expected changed product surfaces after planning approval

- `apps/desktop/src-tauri/tauri.windows.conf.json`
- task-owned NSIS hook under `apps/desktop/src-tauri/windows/`
- task-owned WiX fragment under `apps/desktop/src-tauri/windows/fragments/`
- task-owned `scripts/ci/pkg03-0311-*`
- task-owned `.github/workflows/pkg03-0311-*`
- state files and README only after exact-head acceptance

No Agent/core runtime change and no full custom installer template is planned.
