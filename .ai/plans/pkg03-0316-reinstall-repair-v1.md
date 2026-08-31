# PKG-03 03.16 — Idempotent reinstall and repair execution plan v1

Status: frozen task plan
Task: `03.16`
Linear: `ABD-91`
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Certify that the accepted Windows installer candidates can be safely rerun on an already installed product and can genuinely restore bounded missing/tampered owned files to the exact candidate bytes, without duplicate installation identity or unauthorized mutation of service, ACL, state, network, updater, recovery, or later-task behavior.

## Acceptance

The exact-head Windows certification must:

1. build current-user NSIS, per-machine NSIS and MSI/WiX from the exact source head and bind package hashes/toolchain metadata;
2. perform a clean install for each lifecycle and capture accepted payload hashes plus install-root/product identity;
3. run a healthy same-version reinstall/repair and require success with unchanged expected payload hashes;
4. prove no duplicate install root, shortcut, ARP entry, product identity, or service registration is introduced by the healthy rerun;
5. create a bounded missing-file probe, prove `MISSING`, execute genuine format-specific reinstall/repair, and prove exact SHA-256 restoration to `MATCH`;
6. create a bounded tampered-file probe, prove `HASH_MISMATCH`, execute genuine reinstall/repair, and prove exact SHA-256 restoration to `MATCH`;
7. run a second healthy reinstall/repair after restoration and prove idempotence again;
8. for current-user NSIS, prove no machine Agent service/security state is introduced;
9. for per-machine NSIS/MSI, keep the Agent service quiescent during destructive repair probes and verify the accepted service identity/health contract after repair;
10. verify accepted ACL/security-state locations are not permission-widened or relocated by repair;
11. preserve MSI `/L*V` repair diagnostics, exact exit codes, pre/post hash observations, and UI/action evidence required to reproduce each lifecycle;
12. clean up certification installs using the already accepted normal uninstall path and verify zero tracked repository drift.

## Destructive probe set

- NSIS current-user: Desktop, CLI, Agent may be selected for missing/tamper probes.
- NSIS per-machine: destructive probes limited to Desktop + CLI; Agent remains read-only.
- MSI/WiX per-machine: destructive probes limited to Desktop + CLI; Agent remains read-only.

The implementation harness may choose one missing and one tampered target per lifecycle as long as the target set above is respected and the exact selected path/hash is evidence-bound.

## Format-specific repair path

### NSIS

Rerun the exact generated candidate setup executable in its accepted install scope. No custom repair binary/template/hook is permitted. Same-version rerun must actually restore the damaged file; otherwise the task fails closed.

### MSI/WiX

Use documented Windows Installer repair against the exact installed ProductCode. Force-file reinstall semantics are permitted for the bounded repair proof so that exact restoration does not depend on unproven MSI checksum metadata. Retain verbose repair logs.

## Boundaries

- No live-running Desktop/CLI/Agent repair or Restart Manager claim; 03.19 owns that scope.
- No comprehensive dirty-user-data uninstall preservation; 03.17 owns it.
- No forced transactional failure/interrupted recovery; 03.18 owns it.
- No reboot-required/no-restart claim; 03.20 owns it.
- No silent/passive deployment; 03.21 owns it.
- No signing-secret access, updater mutation, PKG-04 recovery, or PKG-05 release work.
- No Tauri config, installer template/hook, package identity, service identity, ACL policy, firewall/hosts/resolver/trust, or product runtime mutation is authorized by this v1 plan.
- A genuine stock-installer failure must trigger bounded change control rather than acceptance weakening.

## Governance sequence

1. freeze this planning/contract bundle on fresh canonical main;
2. require exact planning-head AI Planning Governance, Repository Governance, PKG-03 Acceptance Sequence, Engineering Contract Governance, and Operational Governance;
3. only after all required planning gates are green, add task-owned harness/validator/workflow surfaces;
4. run all governance gates again on exact implementation head plus `PKG-03 03.16 Reinstall Repair`;
5. independently verify artifact bytes and evidence binding;
6. only then project 03.16 DONE and canonical package progress; never optimistically mutate tracker/master state.

## Evidence

Artifact name: `pkg03-0316-reinstall-repair`

Required:
- `evidence.json`
- `evidence.json.sha256`
- exact source/run/job/artifact binding
- package hashes and ProductCode where applicable
- expected/pre-repair/post-repair SHA-256 observations
- MSI verbose repair logs
- NSIS/UI lifecycle observations/actions
- installation identity/cardinality observations
- service/ACL invariant observations where applicable
- cleanup and zero tracked repository drift proof
