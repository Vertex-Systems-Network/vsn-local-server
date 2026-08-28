# Engineering Governance V3 — Post-Closeout Resume Policy Correction

Date: 2026-08-28
Classification: `CORRECTION`
Approval scope: `PROJECT` inherited from the approved Engineering Governance V3 application and the user's explicit project resume instruction.
Product/runtime scope: none.

## Defect

After Engineering Governance V3 was canonically finalized (`22/22 APPLIED`, final audit v2 persisted and closeout merged), PKG-03 03.11 completed fresh planning/preflight and attempted the required explicit product-development pause lift.

Operational Governance run `33168279670` failed on exact head `05b1fd0bdb59eb58937c12420d3cdab123ee0e9f` because `scripts/operational-governance.py` unconditionally requires:

`checkpoint["pause"]["product_development_paused"] is True`

The associated error contract says this prevents resume "before governance finalization", but the validator never checks whether governance finalization has actually completed. Therefore it also blocks every legitimate post-finalization resume.

## Correct behavior

Operational Governance must remain fail-closed before governance finalization, while allowing a checkpoint to set `product_development_paused=false` only when canonical state proves all of the following:

- Engineering Governance V3 addendum status is `APPLIED`;
- final audit status is `APPLIED`;
- final audit reports `22 APPLIED`, `0 PARTIALLY_APPLIED`, `0 BLOCKED`;
- no second read-only audit remains required;
- governance target has advanced to `PRODUCT_RESUME_RECONCILIATION` or a later post-finalization resume state.

This correction does not itself authorize any product mutation. Task-specific planning/preflight, approval, blockers, exact-head gates and scope contracts still govern product work.

## Scope firewall

Allowed change:
- `scripts/operational-governance.py` resume/finalization validation only;
- this append-only correction record.

Forbidden:
- product/runtime/config/installer changes;
- weakening stop-line, incident, operational-readiness, release-state, recovery, secret-redaction, tech-debt or handoff rules;
- PKG-03 tracker/denominator/dependency/evidence changes;
- rewriting final audit evidence;
- treating a task checkpoint as canonical product acceptance.

## Acceptance

The correction is accepted only if the exact correction head passes:
- AI Planning Governance;
- Repository Governance;
- PKG-03 Acceptance Sequence;
- Engineering Contract Governance;
- Operational Governance.

Operational Governance must continue to pass with the current canonical paused checkpoint and must, by code inspection, reject an unpaused checkpoint unless the canonical Governance V3 finalization conditions above are satisfied.
