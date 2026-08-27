# PKG-03 03.10 — CLI and Agent Payload Placement Plan v1

## Goal

Package the already-declared CLI and Agent executables into both accepted Windows installer formats at the frozen owned paths, prove deterministic discovery and direct launch, and prove uninstall cleanup without crossing into service/PATH/security/updater scope.

## Acceptance sequence

1. Validate canonical 03.10 authority, planning digests, unchanged parent plan and ownership manifest.
2. Build `vsn` and `vsn-agent` with locked Rust/Cargo inputs.
3. Stage only the two exact Windows release executables under a task-owned bundling directory and record SHA-256 values.
4. Add a narrow Tauri `bundle.resources` mapping to:
   - `bin/vsn.exe`
   - `bin/vsn-agent.exe`
5. Build exact-head NSIS and MSI installers with locked Node/Rust/Tauri inputs.
6. NSIS current-user lifecycle:
   - visible interactive install;
   - verify both `bin` files under the current-user install root;
   - verify installed hashes equal staged hashes;
   - execute bounded direct-launch probes by absolute installed path;
   - visible uninstall and verify both files are removed.
7. MSI/WiX lifecycle:
   - visible/default install;
   - verify both `bin` files under the per-machine install root;
   - verify installed hashes equal staged hashes;
   - execute bounded direct-launch probes by absolute installed path;
   - uninstall and verify both files are removed.
8. Verify no service registration, PATH/environment mutation, ACL mutation, signing or updater/recovery behavior is claimed by this task.
9. Bind evidence to exact source SHA/run/job/artifact and verify zero tracked repository drift.
10. Only after genuine acceptance, reconcile PKG-03 to 9/25 or 10/25 according to live canonical state at merge time; never assume 03.09 completion from this branch.

## Concurrency rule

This branch starts from canonical `main` at 8/25. It is independent of 03.09. At reconciliation time it must re-read live `main`:
- if 03.09 is still not integrated, completing 03.10 yields 9/25 and cursor remains 03.09;
- if 03.09 is already integrated, completing 03.10 yields 10/25 and cursor moves to the lowest READY task.
This task never overwrites evidence/state from another lane.

## Expected changed product surfaces after planning approval

- `apps/desktop/src-tauri/tauri.conf.json` — only the resource mapping needed for the two owned payloads.
- task-owned staging/build helper(s) under `scripts/ci/`.
- task-owned validation/certification workflow.
- state files only after exact-head acceptance.

No custom NSIS/WiX template is expected.
