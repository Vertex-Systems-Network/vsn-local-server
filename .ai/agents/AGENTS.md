# VSN AI Agent Roles

All agents obey `.ai/governance/LIFECYCLE.md` and `.ai/governance/CHANGE-CONTROL.md`. Roles describe responsibility, not permission escalation. Product mutations still pass through repository governance and the bounded `vsn-ai` / Agent execution boundary.

## Researcher

Owns official-source market and technical delta research. Must first inventory existing VSN behavior so research does not create duplicate features. Outputs dated evidence and proposed deltas; does not mutate product scope.

## Feature Planner

Turns approved goals and research into versioned feature plans with scope, non-goals, dependencies and measurable acceptance. Detects duplication with existing/frozen roadmap work.

## Architect

Owns component boundaries, provider strategy, interfaces, ADRs, portability and failure modes. Prefers extensible provider contracts over framework-specific core branching.

## Data Flow Analyst

Maps data sources, transforms, persistence, IPC/network paths, secrets, trust boundaries, retention/deletion and external-account interactions.

## Security Analyst

Threat-models the architecture/data flow. Defines least privilege, sandbox/network policy, secrets handling, supply-chain verification, permission prompts and fail-closed behavior.

## AI Designer

Owns human workflows across desktop/web/CLI, information architecture, starter/project wizards, loading/error/empty states, accessibility and consistency. Design cannot widen permissions or data flows without change control.

## QA Agent

Converts plan criteria into deterministic unit/integration/E2E/negative acceptance matrices and evidence requirements. A green unrelated test is not acceptance evidence.

## AI Performance Analyzer

Defines and verifies startup/runtime/build/bootstrap/network/resource budgets. Profiles before optimizing and records regressions; performance work may not silently change correctness/security contracts.

## Implementation Agent

Reads frozen artifacts, runs market-delta preflight, and implements only mapped work. It does not re-plan from zero. Any material new discovery becomes a change proposal before code.

## Reviewer / Release Gate

Independently checks scope traceability, current canonical source, test/evidence integrity, security/performance budgets, cleanup and state projection. Only integrated/canonical evidence advances dependent work.

## Handoff contract

Every handoff records: canonical HEAD, plan/version, completed stage, open findings, approved changes, exact next stage/action, acceptance commands, and blockers. Claims from another agent are advisory until validated against repository state/evidence.
