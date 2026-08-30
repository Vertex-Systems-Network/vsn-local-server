# PKG-03 03.18 — Transactional install failure rollback and interrupted-install recovery plan v1

Status: frozen task plan
Task: `03.18`
Linear: `ABD-93`
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Certify that exact Windows installer candidates fail cleanly under deterministic mid-install faults and can recover from an interrupted installation without leaving partial package ownership, duplicate identity, unsafe service/security residue, or user/system-state damage.

## Acceptance

Exact-head Windows evidence must:
1. build and hash current-user NSIS, per-machine NSIS and MSI/WiX candidates;
2. establish clean preflight plus evidence-bound failure sentinels outside preserved user-data scope;
3. force a genuine deterministic install failure for each format after setup begins;
4. bind non-success exit/log/UI evidence and prove no unauthorized partial owned payload/ARP/shortcut/service/security state remains;
5. prove failure sentinels and protected firewall/hosts/resolver/trust state remain unchanged;
6. start a second clean install and interrupt it only after a positive transaction-start observation;
7. inventory post-interruption package residue without modifying it optimistically;
8. rerun the exact same candidate and require deterministic recovery into exactly one valid complete installed identity, or fail closed;
9. verify accepted owned payload hashes and service/ACL invariants after recovery;
10. prove no duplicate install root, registration, shortcut or service identity;
11. perform accepted ordinary uninstall cleanup and return product-owned state to absent;
12. retain MSI verbose failure/recovery logs plus NSIS UI/process/action evidence;
13. verify zero tracked repository drift.

## Boundaries

- No running Desktop/CLI/Agent coordination or Restart Manager claim (03.19).
- No reboot semantics (03.20).
- No silent deployment (03.21).
- No signing/updater/PKG-04 recovery or cross-platform work.
- Initial v1 plan authorizes certification-first only; any installer/product change requires bounded change control after exact failure evidence.

## Governance sequence

Planning head -> five governance gates -> task-owned certification implementation -> five exact implementation-head governance gates + `PKG-03 03.18 Install Rollback` -> independent artifact verification -> canonical DONE projection only after accepted evidence.

## Evidence artifact

`pkg03-0318-install-rollback`
