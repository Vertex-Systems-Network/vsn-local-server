# Operational Readiness and Release Governance

Status: normative governance for new materially planned work after `GOV-V3.2`.

## Operational readiness

Production-relevant work must declare operational readiness before final acceptance. Each surface is either `REQUIRED` or `NOT_APPLICABLE_WITH_RATIONALE`; applicability may not disappear silently.

Consider and document, as applicable:

- structured application/runtime logs;
- audit/security logs;
- request/correlation IDs;
- health/readiness checks;
- metrics and capacity/error indicators;
- background-job status/monitoring;
- alerts and actionable thresholds;
- traces where distributed or latency diagnosis requires them;
- support diagnostics and troubleshooting commands;
- operator runbooks and escalation paths;
- retention/redaction rules for logs and diagnostics.

Operational evidence must not persist passwords, API keys, tokens, private keys, authorization headers, unnecessary personal data, raw secret material, or other sensitive values. Persist references, hashes, redacted excerpts, IDs and safe summaries instead.

## Release state machine

These states are distinct and may never be conflated:

1. `BUILT` — a revision/artifact was produced successfully.
2. `DEPLOYED` — the artifact/revision was placed into the target environment.
3. `RELEASED` — the intended audience can actually receive/use the deployed revision under the approved release mechanism.
4. `PRODUCTION_VERIFIED` — post-release checks prove the production outcome on the exact released revision/environment.

A later state requires evidence for every earlier state. A successful build is not deployment; a successful deployment command is not release; a released revision is not production-verified without post-release evidence.

## Recovery classification

Every release-capable change declares one recovery class:

- `SIMPLE_ROLLBACK` — previous revision can be restored without compatibility/data repair.
- `ROLLBACK_WITH_COMPATIBILITY` — rollback is possible only while declared compatibility conditions hold.
- `FORWARD_FIX_PREFERRED` — rollback could cause greater data/compatibility risk than a bounded forward fix.
- `IRREVERSIBLE` — no safe rollback exists for the affected state.

`IRREVERSIBLE` actions require explicit approval for the exact action, affected environment/data and recovery limitation before mutation. Generic feature/milestone approval is insufficient.

## Release preflight

Before a high-risk release, know and record as applicable:

- exact revision/artifact digest;
- target environment;
- migrations/data changes and deployment ordering;
- dependency/configuration/feature-flag changes;
- required FAST/FULL gate results;
- security status;
- recovery class and recovery procedure;
- backup/restore implications;
- post-deployment verification and production verification checks;
- approvals required for privileged or irreversible actions.

## Acceptance

A task is not `PRODUCTION_VERIFIED` merely because CI is green or a deploy command returned success. Release-state claims must be evidence-bound under `.ai/governance/EVIDENCE.md` and recorded in the feature/work-package or durable end-task report.
