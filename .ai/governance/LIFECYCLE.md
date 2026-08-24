# AI Development Lifecycle

Status: planning/governance contract. It does not mark any product capability implemented.

## Stage state machine

Each feature/workstream moves only forward through these gates unless an approved change proposal explicitly reopens an earlier stage.

1. **Research** — user problem, existing VSN capability, official platform/tooling, market delta, constraints and evidence date.
2. **Plan** — outcome, scope, non-goals, dependencies, acceptance criteria, rollout and rollback.
3. **Architecture** — components, ownership, interfaces, provider boundaries, ADRs and failure modes.
4. **Data Flow** — inputs/outputs, persistence, secrets, trust boundaries, network paths and deletion/retention.
5. **Security** — threat model, permissions, sandboxing, supply-chain rules, secret handling and fail-closed behavior.
6. **Design** — UX flows, API/CLI contracts, accessibility, empty/error/loading states and design tokens where applicable.
7. **QA** — unit/integration/E2E/negative tests, platform matrix, evidence format and deterministic acceptance gate.
8. **Performance** — budgets for startup, latency, memory, CPU, disk, network, build/bootstrap and regression thresholds.
9. **Development** — implement the approved artifacts; do not redesign scope opportunistically.

## Required implementation preflight

Before coding, the implementation agent must emit/check a compact preflight containing:

- approved plan ID/version;
- current canonical repository HEAD and active package/task;
- completed prerequisite stages;
- last research review date;
- current market-delta result (`none`, `informational`, or `change_required`);
- exact files/modules expected to change;
- acceptance commands/gates.

If `change_required`, development is blocked until change control resolves it.

## Research freshness

Research is refreshed at implementation start, but only as a delta from the approved baseline. Prefer official primary sources. Record dates and source URLs. A new framework release, deprecation, CLI replacement, security advisory, platform policy change or new official development mode is a delta; it is not automatic permission to expand scope.

## Traceability

Every implementation PR should be traceable to a plan item and acceptance criterion. Every acceptance criterion should have a test/evidence path. Unmapped code is drift and must be removed, mapped through an approved change, or explicitly justified as prerequisite remediation.

## Parallel work

Parallel agents may research independent areas, but mutation is serialized at shared architecture/state boundaries. No agent may advance a dependent task because another agent merely claims completion; canonical evidence/integration controls progression.
