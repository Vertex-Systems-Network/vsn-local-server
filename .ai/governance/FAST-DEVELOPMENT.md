# Fast Development Execution Profile v1

Status: approved project-level execution addendum. This profile accelerates implementation throughput without changing canonical acceptance semantics, product permissions, target-system behavior, or the frozen package dependency DAG.

Approval reference: `conversation:user-2026-09-16-fast-development-implementation`.

## Non-negotiable invariants

1. `docs/MASTER-EXECUTION-STATUS.json` plus the unique active package tracker remain canonical acceptance authority.
2. A task marked `BLOCKED` by the canonical DAG remains blocked for acceptance. Fast development never changes `READY`, `DONE`, package progress, dependency edges, denominators, or release/certification claims.
3. Fixture, synthetic, test-signed, self-signed, mock-provider, or pre-certification evidence can never satisfy a production/external acceptance criterion.
4. Security, QA, containment, secret-handling, privilege boundaries, target-system safety, and fail-closed behavior are not relaxed.
5. External/privileged/irreversible operations still require their normal authority. No fake provider IDs, Store identities, credentials, signatures, approvals, or production evidence may be invented.
6. Existing accepted evidence remains immutable. Fast development does not rewrite historical plans/evidence to match new code.

## PREIMPLEMENTATION lane

`PREIMPLEMENTATION` is a development state outside the canonical task tracker. It permits implementation and test-harness work for a canonically blocked future task when all of the following are true:

- the behavior is already inside approved product scope; no `NEW_PRODUCT_SCOPE` is introduced;
- the future task's interface/acceptance contract is sufficiently frozen to build against deterministic fixtures;
- missing prerequisite outputs can be represented by explicit non-production fixtures without weakening the final contract;
- the work package declares exact mutable surfaces, collision keys, scope budget and `PARALLEL_SAFE` or `COORDINATED_PARALLEL` execution;
- no external production account, signing key, provider identity, privileged target mutation, or irreversible action is required;
- Security and QA define negative tests proving fixture evidence cannot be mistaken for final acceptance.

A PREIMPLEMENTATION branch/PR must carry the literal marker `NON_ACCEPTANCE_PREIMPLEMENTATION` and identify the canonical blocker it is waiting on.

### Promotion rule

When the real prerequisite becomes canonically DONE:

1. refresh live `main` and the active tracker;
2. reconcile the preimplementation branch onto the current integration base;
3. replace fixture bindings with the genuine predecessor outputs/evidence;
4. rerun all task-specific FULL GATE, security, integration, negative and platform acceptance required by the original frozen contract;
5. only genuine integrated evidence may project the task to DONE or unblock dependent work.

A fixture-pass is never grandfathered into final acceptance.

## Parallel execution model

Use the existing project/package concurrency ceiling as the upper bound for acceptance work. PREIMPLEMENTATION may use additional isolated agents only when mutable surfaces do not collide with active acceptance work.

The orchestrator must maintain an execution map containing:

- canonical HEAD and active package/task;
- acceptance lane(s);
- preimplementation lane(s);
- exact mutable paths/modules;
- shared surfaces/collision keys;
- integration owner;
- FAST GATE command/evidence;
- final prerequisite and FULL GATE required for promotion.

Shared files such as canonical trackers, package plans, root lockfiles, common manifests, release state and shared workflow routers are single-writer integration surfaces unless an explicit coordination contract says otherwise.

## Integration epochs

To reduce exact-head churn, parallel agents should work from one refreshed canonical base during a short integration epoch. Default operational target: 4-6 hours, shortened whenever `main` materially moves or a security blocker appears.

During an epoch:

- each lane owns disjoint declared surfaces;
- shared-surface edits serialize through the integration owner;
- agents run targeted FAST GATE feedback on their own slice;
- an epoch does not freeze or block emergency/security fixes on canonical `main`;
- before integration, refresh `main`; stale branches reconcile rather than silently overwrite newer work.

The epoch ends with one coordinated integration candidate and the required FULL GATE set, reducing repeated whole-repository exact-head rebuild cycles.

## CI tiers

### Tier 1 — FAST GATE

Run on each meaningful mutation slice. It contains only the targeted compile/type/unit/contract/security checks needed by the changed surfaces plus mandatory governance checks. FAST GATE is feedback only.

### Tier 2 — INTEGRATION FULL GATE

Run on the coordinated integration candidate before merge. It runs package/task integration, negative/fail-closed tests, required platform lanes, scope/zero-drift checks and any impacted completed-package regression.

### Tier 3 — RELEASE/CERTIFICATION GATE

Run candidate-bound full regression, reproducibility, clean-machine/VM, signing/provenance, security/resilience/pentest or other release evidence only when its original contract requires it. Tier 3 evidence is never replaced by Tier 1/2.

Completed-package historical workflows may be path-routed away from PRs that change only unrelated governance/docs/task-local certification surfaces, but they must remain runnable on relevant shared/product/toolchain changes, trusted integration candidates, manual dispatch where already supported, and release/certification boundaries. Routing must be proven before broad adoption; tests are not deleted merely for speed.

## Live state, not stale checkpoints

`.ai/current-work.json` remains a historical/non-authoritative checkpoint. Agents should use:

```bash
python3 scripts/ci/live-work-state.py
```

immediately before planning/mutation and record the emitted canonical HEAD/package/task/ready-state in the handoff. The script fails closed when the active package tracker is missing or ambiguous.

## External-gate decoupling

An external gate may block final acceptance without blocking safe internal implementation. Examples include Microsoft Store verification, production signing-provider onboarding, external review, notarization, or certificate issuance.

While such a gate is pending, agents may prepare deterministic package builders, manifest renderers, validators, VM harnesses, provenance/SBOM pipelines, Store-safe user-mode behavior, tests and documentation using explicit non-production fixtures. They may not impersonate the external authority or claim final acceptance.

## Dormant/stale branch policy

Dormant research and historical PRs are evidence inputs, not execution authority. Reuse their verified findings through a current work package/addendum; do not merge stale branches merely to recover research. Current canonical source wins.

## Safety / rollback

This profile changes development orchestration only. It does not change runtime protocols, installer behavior, target-system permissions, data paths, updater authority or release semantics.

If the profile causes ambiguous ownership, evidence confusion, security regression or repeated integration conflicts, stop new PREIMPLEMENTATION work, keep canonical acceptance state unchanged, and fall back to the normal serialized lifecycle until the coordination issue is resolved.
