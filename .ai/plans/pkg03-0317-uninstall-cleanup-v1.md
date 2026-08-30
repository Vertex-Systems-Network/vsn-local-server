# PKG-03 03.17 — Uninstall owned-artifact cleanup with user-data preservation plan v1

Status: frozen task plan
Task: `03.17`
Linear: `ABD-92`
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Certify that uninstall removes every accepted installer-owned Windows artifact and registration while preserving user-created and process-context mutable data outside the install root byte-for-byte, without unsafe recursive deletion, scope crossover, network/trust mutation, or silent expansion into rollback/running-process/reboot behavior.

## Acceptance

The exact-head Windows certification must:

1. build current-user NSIS, per-machine NSIS and MSI/WiX from the exact source head and bind package hashes/toolchain metadata;
2. perform a clean install for each lifecycle and capture accepted owned payload, ARP/ProductCode, shortcut and service identity;
3. resolve and seed deterministic mutable data/config markers plus a workspace/project fixture outside the install root;
4. record exact marker bytes, SHA-256, paths and security metadata before uninstall;
5. execute the genuine ordinary uninstall path for each format;
6. prove all accepted package-owned executable payload and required empty install-root directories are removed;
7. prove applicable ARP/ProductCode registration and owned shortcuts are removed;
8. prove `VSN-Agent` service registration is removed for per-machine lifecycles and is never created by current-user uninstall;
9. prove all seeded persistent data/config/workspace/project markers remain byte-identical and path-stable;
10. prove no unrelated adjacent file outside owned boundaries is deleted;
11. prove uninstall does not follow a bounded reparse-point/junction escape into preserved outside data;
12. prove firewall/hosts/resolver/trust state remains unchanged from the accepted 03.13 boundary;
13. classify `%PROGRAMDATA%\VSN\security` separately and remove it only if exclusive product ownership is exact-head proven; no heuristic recursive deletion is allowed;
14. preserve MSI verbose uninstall diagnostics and NSIS UI/action evidence;
15. verify zero tracked repository drift after cleanup.

## Preservation policy

### Preserve
- user-created projects/workspaces;
- process-context mutable data/config;
- unrelated files outside installer-owned paths;
- any state whose ownership cannot be proven safe to delete.

### Remove
- exact accepted package-owned payload;
- owned shortcuts/application registration;
- ARP/ProductCode registration;
- per-machine `VSN-Agent` service registration;
- task-proven empty installer-owned directories.

Security state is not silently treated as either class. `%PROGRAMDATA%\VSN\security` requires explicit evidence-backed classification in this task.

## Boundaries

- No transactional failure/interrupted recovery; 03.18 owns it.
- No live-running Desktop/CLI/Agent uninstall or Restart Manager claim; 03.19 owns it.
- No reboot-required/no-restart claim; 03.20 owns it.
- No silent/passive deployment; 03.21 owns it.
- No signing-secret access, updater mutation, PKG-04 recovery, or PKG-05 release work.
- No Tauri config, installer template/hook, package identity, service identity, ACL policy, firewall/hosts/resolver/trust, or product runtime mutation is authorized by this v1 plan.
- A genuine stock-installer cleanup/preservation failure must trigger bounded change control rather than acceptance weakening.

## Governance sequence

1. freeze this planning/contract bundle on fresh canonical main;
2. require exact planning-head AI Planning Governance, Repository Governance, PKG-03 Acceptance Sequence, Engineering Contract Governance, and Operational Governance;
3. only after all required planning gates are green, add task-owned harness/validator/workflow surfaces;
4. run all governance gates again on exact implementation head plus `PKG-03 03.17 Uninstall Cleanup`;
5. independently verify artifact bytes and evidence binding;
6. only then project 03.17 DONE and canonical package progress; never optimistically mutate tracker/master state.

## Evidence

Artifact name: `pkg03-0317-uninstall-cleanup`

Required:
- `evidence.json`
- `evidence.json.sha256`
- exact source/run/job/artifact binding
- package hashes and ProductCode where applicable
- owned-artifact pre/post inventories
- preserved data/config/workspace marker hashes and metadata
- service/ARP/shortcut cleanup observations
- network/trust non-mutation snapshot hashes
- machine security-state classification
- MSI verbose uninstall log
- NSIS UI lifecycle observations/actions
- reparse-point containment observation
- cleanup and zero tracked repository drift proof
