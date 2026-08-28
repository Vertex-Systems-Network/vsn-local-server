# Durable Handoff and Unrelated-Finding Governance

Status: normative governance.

## End-of-task / handoff record

After meaningful engineering work, and whenever work becomes blocked, pauses across sessions/agents, reaches a milestone boundary, or exits incident mode, persist a durable report using `.ai/templates/end-task-report.v1.json` or an equivalent repository-native record containing the same information.

The report records:

- status/state;
- changed areas and why;
- research performed;
- tests/checks and evidence references;
- security review;
- data/migration implications;
- affected areas;
- exact VCS branch/revision/PR where applicable;
- documentation/checkpoint changes;
- known issues;
- items not verified;
- release state and recovery classification where applicable;
- incident/stop-line state where applicable;
- exact next safe action.

The handoff must be sufficient for a new AI/engineer to resume after refreshing live repository state, without requiring previous chat history.

## Checkpoint relationship

`.ai/current-work.json` remains a non-authoritative resume checkpoint. The durable end-task report may feed/update it, but neither can override live repository, CI, database/schema/config, accepted evidence or canonical package state.

Before resuming or mutating, refresh live state and reconcile any conflict.

## Secrets and sensitive information

Do not persist secret values, authorization headers, private keys, tokens, passwords, or unnecessary sensitive data in handoff/checkpoint/tech-debt records. Use secure references/handles, hashes and redacted summaries.

## Unrelated findings and technical debt

During scoped feature work, do not automatically fix unrelated cleanup/refactors/dependency upgrades/architecture improvements merely because they are discovered.

Record them using `.ai/templates/tech-debt-item.v1.json` or equivalent repository-native tracking with:

- source task/evidence;
- affected area and impact;
- risk if deferred;
- owner/priority/disposition when known;
- explicit statement whether current scope authorizes mutation.

Default for an unrelated finding is `authorized_to_fix_now=false`. Changing that requires the normal scope/change-control process unless immediate stop-the-line/incident containment rules apply.

This rule prevents silent scope creep while ensuring important maintainability/security/support findings are not forgotten.
