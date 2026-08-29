# Incident Mode and Stop-the-Line Governance

Status: normative governance.

## Stop-the-line triggers

Immediately stop the affected normal workstream when any of these is observed or cannot be safely ruled out:

- unexpected data loss or corruption;
- cross-user, cross-workspace or cross-tenant data exposure;
- credential/private-key/token exposure;
- destructive or privileged command whose effect/target is not safely understood;
- migration/schema corruption;
- unexplained massive or cross-scope diff;
- repository/VCS state that cannot be safely understood;
- critical authentication/authorization/security bypass;
- evidence-integrity failure that makes acceptance claims untrustworthy;
- production impact whose scope is unknown and could worsen through continued feature mutation.

## Mutation boundary

When `STOP_THE_LINE` or `INCIDENT_ACTIVE` is set for a surface:

- normal feature/refactor/release mutation on that affected surface is blocked;
- unrelated parallel work may continue only if it is demonstrably isolated and cannot interfere with containment/evidence;
- permitted mutation is limited to the minimum incident-bounded action required to stabilize, contain, preserve evidence or recover;
- authority does not expand merely because an incident exists;
- privileged, destructive or irreversible recovery still requires the applicable explicit authorization.

## Incident sequence

Use this order unless an immediate safety action requires a documented exception:

`STABILIZE -> CONTAIN -> PRESERVE_EVIDENCE -> DIAGNOSE -> RECOVER -> VERIFY -> ROOT_CAUSE -> PREVENT_RECURRENCE`

Do not perform broad feature refactoring during containment/recovery.

## Evidence preservation

Preserve, where safe and relevant:

- exact revision/branch/environment identifiers;
- timestamps and affected scope;
- failing commands/checks and exit status;
- safe/redacted logs;
- hashes/digests of relevant artifacts;
- database/schema/config versions without secret values;
- screenshots or operator observations when they materially prove behavior;
- recovery actions and their results.

Never persist secret values merely to preserve evidence. Redact sensitive material and retain secure references/handles instead.

## Exit from incident/stop-line

Normal work may resume on the affected surface only after:

- immediate impact is stabilized/contained;
- evidence needed for diagnosis is preserved;
- recovery is verified or an approved bounded degraded state is documented;
- the current repository/environment state is understood well enough to mutate safely;
- any required security/production owner approval is satisfied;
- the checkpoint/end-task report records current state and exact next safe action.

A root-cause/prevention item may remain open after service recovery, but it must be durably tracked and must not be silently forgotten.
