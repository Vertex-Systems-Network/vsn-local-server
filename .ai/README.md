# VSN AI Workspace

This directory is the repository-local operating workspace for AI-assisted planning and delivery. It is governance and context, not an alternative execution engine. Runtime tool execution remains bounded by `crates/vsn-ai`, policy, permission and Agent mutation boundaries.

## Mandatory lifecycle

Every planned capability follows this order:

`Research -> Plan -> Architecture -> Data Flow -> Security -> Design -> QA -> Performance -> Development`

Development may start only after the preceding artifacts are present or explicitly marked not-applicable with rationale.

## Resume rule

An AI agent must not restart planning from zero when implementation begins. It must:

1. read `.ai/state.json` and the approved plan;
2. read relevant architecture, data-flow, security, design, QA and performance artifacts;
3. perform a time-bounded market-delta research pass for changes since `research_reviewed_at`;
4. record genuinely new findings as a change proposal/addendum;
5. continue the frozen plan unless an approved change alters it.

Silent scope drift is prohibited.

## Directory contract

- `governance/` — lifecycle and change-control rules.
- `agents/` — role boundaries and handoff contract.
- `catalog/` — versioned development-platform/provider catalog.
- `research/` — dated market and technical research artifacts.
- `plans/` — approved product/feature plans and addenda.
- `architecture/` — system/component decisions and ADRs.
- `data-flow/` — trust-boundary and data-flow models.
- `security/` — threat models and security requirements.
- `design/` — UX/UI/system design specifications.
- `qa/` — acceptance matrices and test strategy.
- `performance/` — budgets, benchmarks and regression criteria.
- `evidence/` — links/digests for accepted work; generated bulky evidence stays outside Git when repository policy requires.
- `changes/` — proposed/approved/rejected plan deltas.

Directories may be materialized when their first artifact is created; Git does not track empty directories.

## Canonical-state boundary

The current frozen PKG-02 acceptance sequence remains authoritative for active product delivery. This workspace must never silently change its denominator, task wording, dependency order, acceptance evidence or machine-readable completion state.
