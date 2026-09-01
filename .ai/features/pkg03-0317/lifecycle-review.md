# PKG-03 03.17 Lifecycle Review — Uninstall cleanup and preservation

Reviewed: 2026-08-30
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.17`
Linear: `ABD-92`

## Lifecycle matrix

| Format | Owned cleanup | Preserved dirty data | Machine-specific rule |
| --- | --- | --- | --- |
| NSIS current-user | remove accepted install-root payload, user ARP entry and owned shortcuts | preserve resolved user data/config + workspace/project fixtures byte-for-byte | must not create/remove `VSN-Agent` or `%PROGRAMDATA%\VSN\security` |
| NSIS per-machine | remove accepted install-root payload, machine ARP entry, owned shortcuts and `VSN-Agent` service | preserve resolved mutable data/config + workspace/project fixtures byte-for-byte | classify machine IPC security state separately; cleanup only if exact ownership proof exists |
| MSI/WiX per-machine | remove component-owned payload, ARP/ProductCode registration, owned shortcuts and `VSN-Agent` service | preserve resolved mutable data/config + workspace/project fixtures byte-for-byte | preserve verbose uninstall log; classify security-state cleanup separately |

## Required phase order per lifecycle

1. establish a clean preflight and install the exact candidate;
2. capture package-owned file/registration/shortcut/service inventory;
3. resolve mutable data/config roots in the relevant execution context;
4. seed deterministic dirty user-data and workspace/project markers outside install root;
5. capture SHA-256 and security metadata for every preserved marker;
6. snapshot protected network/trust state from the accepted 03.13 boundary;
7. run genuine ordinary uninstall;
8. prove all required owned artifacts/registrations/services are absent;
9. prove every preserved marker remains byte-identical and path-stable;
10. prove protected network/trust state is unchanged;
11. prove no tracked repository drift and retain exact logs/UI/action evidence.

## Ownership classes

### MUST REMOVE
- accepted package-owned executable payload under the selected install root;
- package-owned shortcuts/application registration;
- the applicable ARP/ProductCode registration;
- `VSN-Agent` SCM registration for per-machine lifecycles;
- task-proven installer-owned empty directories that contain no preserved data.

### MUST PRESERVE
- user-created workspace/project fixture outside install root;
- resolved mutable data/config marker files representing persistent settings/state;
- any pre-existing unrelated file adjacent to, but outside, owned package boundaries.

### CLASSIFY BEFORE ACTION
- `%PROGRAMDATA%\VSN\security` and `ipc.key`.
These are security state rather than ordinary user data. 03.17 may remove them only if exact-head evidence proves exclusive product ownership and safe removal; otherwise they remain untouched and the task records the unresolved ownership as a fail-closed change-control trigger.

## Reparse-point boundary

The harness may create one deterministic junction/reparse-point escape fixture from an owned cleanup area to a preserved outside directory only if it can do so without changing product behavior. Uninstall must not follow that escape and delete the outside fixture. If the generated installer recursively follows reparse points, stop and classify as a product cleanup defect.

## Nonclaims

03.17 does not certify:
- rollback after failed/interrupted installation;
- uninstall while Desktop/CLI/Agent are actively running;
- Restart Manager behavior;
- reboot-required/no-restart semantics;
- silent/passive deployment;
- signing, updater, PKG-04 recovery, or cross-platform release behavior.
