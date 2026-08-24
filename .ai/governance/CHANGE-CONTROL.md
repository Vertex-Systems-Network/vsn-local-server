# AI Plan Change Control

## Purpose

Approved plans are stable execution contracts. Fresh research may extend them, but an AI agent must never silently rewrite scope, acceptance criteria, security posture or task order while coding.

## Change classes

- **Informational** — documentation/source refresh with no behavior or acceptance impact. Record it; implementation may continue.
- **Compatible extension** — new optional provider, starter, test or integration that does not invalidate accepted behavior. Requires an approved addendum before implementation.
- **Contract change** — modifies interfaces, data flow, permissions, security assumptions, acceptance criteria, dependencies or roadmap ordering. Requires explicit approval and re-review of impacted downstream stages.
- **Emergency security change** — remediation for an actively unsafe behavior. May interrupt sequencing only when the repository's security/governance rules permit it; document the reason, blast radius and regression evidence.

## Change proposal minimum fields

Each proposal under `.ai/changes/` must contain:

- unique ID and date;
- source plan/version;
- discovery source and research date;
- problem/new capability;
- change class;
- affected stages/files/providers;
- compatibility and migration impact;
- security/data-flow impact;
- acceptance additions/changes;
- disposition: proposed / approved / rejected / deferred;
- approver or canonical decision reference.

## Drift policy

Implementation is drift when it adds behavior not mapped to the approved plan/addenda, bypasses a prerequisite stage, broadens permissions/network reach, changes an acceptance denominator, or substitutes a different platform/provider without a recorded decision.

Drift must fail the planning preflight. Do not normalize it after the fact by editing the plan to match already-written code.

## Platform catalog changes

The platform catalog is intentionally extensible. Adding a new provider/profile is normally a compatible extension when it:

- uses the provider SDK/declared capability model;
- does not weaken existing certified templates;
- declares local-vs-SaaS execution accurately;
- pins or discovers official tooling safely;
- has explicit secrets/account/network requirements;
- gets its own acceptance evidence before being advertised as supported.

`proposed` does not mean implemented. `existing_certified` is reserved for behavior with repository acceptance evidence.
