# PKG-03 03.14 Installed Payload Integrity Detection Contract v1

Canonical base: `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`
Task: `03.14`
Linear: `ABD-89`

## Owned set

The integrity detector is restricted to:
1. `VSN Dev Platform.exe`
2. `bin/vsn.exe`
3. `bin/vsn-agent.exe`

No directory wildcard, user data, runtime state, logs, caches, certificates, secrets, project content, or updater payload may enter the owned set.

## Classification

For each owned path, the detector must return exactly one of:
- `MATCH` — file exists and SHA-256 equals the exact-head expected value;
- `MISSING` — file does not exist as a regular file;
- `HASH_MISMATCH` — file exists but SHA-256 differs.

A result requiring repair is derived as `classification != MATCH`; 03.14 does not execute the repair.

## Evidence requirements

Every observation records:
- lifecycle;
- install root;
- owned relative path;
- expected SHA-256;
- observed existence;
- observed SHA-256 when present;
- classification;
- `repair_required`;
- source commit.

The final evidence must prove:
- healthy post-install `MATCH` for all owned files in all three installer lifecycles;
- current-user NSIS missing + tamper detection for all three owned files;
- per-machine NSIS missing + tamper detection for Desktop and CLI while Agent remains read-only;
- MSI/WiX missing + tamper detection for Desktop and CLI while Agent remains read-only;
- exact restoration to `MATCH` after each controlled probe;
- uninstall cleanup;
- zero tracked repository drift.

## Scope firewall

03.14 certification may intentionally alter an installed test fixture and restore the exact verified bytes only for bounded detection probes.

It may not:
- invoke installer repair/reinstall/self-healing;
- add or change service lifecycle behavior;
- stop/start the Agent merely to make a destructive probe possible;
- mutate Tauri configuration or installer templates/hooks;
- change ACLs, firewall, hosts, resolver, trust stores, PATH, signing, updater or recovery behavior;
- claim repair success.

Any real repair lifecycle remains 03.16.
