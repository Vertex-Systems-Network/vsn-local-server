# PKG-03 03.24 Research — Fresh/dirty Windows VM acceptance matrix

Status: **PLANNING-ONLY / BLOCKED**
Reviewed: 2026-09-04
Canonical preflight base: `e3fb61581646a475c117cc893566286e397c2108`
Task: `03.24`
Linear: `ABD-99`

## Frozen dependency contract

`03.16`, `03.17`, `03.18`, `03.19`, `03.20`, `03.21`, `03.22`, `03.23`.

At this checkpoint 03.22 is not canonically DONE and 03.23 is therefore blocked. 03.24 has no implementation authority.

## Preflight finding

03.24 is an end-to-end acceptance task over already accepted installer behavior. Its default posture is **certification-first / no product mutation**. It should consume exact release candidates and provenance accepted by 03.23, then exercise them across deterministic clean and intentionally dirty Windows states.

The activation-time matrix must cover the accepted installer families and scopes without inventing new behavior:
- current-user NSIS;
- per-machine NSIS;
- MSI/WiX enterprise package;
- install, reinstall/repair and uninstall paths;
- preservation/removal boundaries already frozen by 03.12/03.17;
- running-resource/reboot semantics already frozen by 03.19/03.20;
- unattended semantics from 03.21;
- production signature verification from 03.22;
- exact hashes/SBOM/provenance from 03.23.

## Fresh vs dirty state model

A fresh case must start from a Windows image with no VSN install roots, service, registration, shortcuts or task-owned test residue.

A dirty case must be deliberately seeded from a documented prior state, such as:
- an accepted existing installation requiring repair/reinstall;
- missing/tampered owned payload consistent with 03.14 scenarios;
- preserved user data/config that must survive uninstall per 03.17;
- service/runtime state relevant to accepted 03.19 behavior;
- pending-reboot/no-restart state relevant to 03.20 where deterministic reproduction is supported.

Dirty-state setup must be explicit and reproducible; accidental runner residue cannot count as acceptance evidence.

## Infrastructure decision deferred to activation

The task title requires Windows VM acceptance, but this preflight does not freeze a specific VM provider. At activation, the plan must prove that each case starts from a known image/snapshot and that dirty cases are seeded deterministically. GitHub-hosted `windows-2025` may be used for suitable isolated cases, but any scenario needing reboot persistence/snapshot semantics must use an infrastructure path that can actually prove that lifecycle rather than simulating it falsely.

### GitHub-hosted runner lifecycle constraint — verified 2026-09-04

GitHub's current hosted-runner documentation states that a new VM is automatically provisioned for each hosted job, all steps in that job share that VM/filesystem, and the VM is automatically decommissioned when the job finishes. GitHub also documents `windows-2025` as a standard hosted Windows runner label.

Consequences for 03.24:
- standard `windows-2025` is suitable for fresh isolated cases and for dirty-state seeding/execution that completes within one job;
- a second hosted job cannot be treated as continuation of the first job's VM state;
- reboot-persistence acceptance must not be simulated by splitting pre/post checks across ordinary hosted jobs;
- any row that requires state to survive a real reboot must use infrastructure that preserves the same VM across the boot boundary and can resume evidence collection afterward (for example a governed persistent/self-hosted or managed VM path selected at activation time);
- the final evidence must identify the VM/image/snapshot and prove that the post-reboot observation belongs to the same governed candidate and persisted machine state.

Official references checked at this preflight:
- https://docs.github.com/en/actions/how-tos/manage-runners/github-hosted-runners/use-github-hosted-runners
- https://docs.github.com/en/actions/reference/runners/github-hosted-runners

## Evidence requirements

Every matrix row should bind:
- exact source SHA;
- exact 03.23 handoff manifest and package SHA-256;
- Windows image/build identity;
- case seed description/digest;
- installer family/scope and command line;
- native exit codes and bounded timing;
- signature verification result before execution;
- expected/actual install roots, registration, shortcuts and service state;
- user-data preservation result where applicable;
- non-mutation checks for firewall/hosts/resolver/trust-store;
- cleanup result and post-case residue inventory;
- logs/diagnostics artifact identity.

## Scope boundary

No PKG-04 updater/recovery or PKG-05 cross-platform implementation. Failures must first be classified as product defect, test-harness defect, runner/infrastructure issue or governance/evidence mismatch. Product changes require the smallest acceptance-mapped change-control path and a fresh exact-head rerun.

`change_required = false` for the frozen PKG-03 boundary at this preflight checkpoint.
