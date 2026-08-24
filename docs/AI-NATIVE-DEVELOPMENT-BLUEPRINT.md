# VSN AI-Native Development Blueprint

Status: planning-only future architecture. This document does not change the frozen PKG-02 sequence and does not implement active task 02.23.

Reviewed against canonical `main` `d9c5aa245efb0d20957b4eb840e29a4f95a520d2` on 2026-08-24.

## Audit result

VSN is not starting from zero.

Existing accepted foundations include:

- `crates/vsn-project` project detection, dependency reporting, bounded bootstrap execution and a versioned `ProjectProvider` SDK;
- certified built-in 02.07 templates: Laravel, Node.js, Django, Rust and Go;
- `crates/vsn-ai` structured intents, candidate-plan validation, bounded tool-call count, mutation confirmation and no unrestricted shell;
- runtime, database, network, files, terminal, container, preview, audit and policy crates that future providers can compose.

The primary gaps are orchestration/governance rather than a need for another unconstrained AI engine: no root `.ai/` workspace, no mandatory multi-stage planning state machine, no frozen-plan/change-control protocol, no market-delta research contract, no explicit AI role handoffs, and no broad versioned platform/provider catalog.

## Product direction

VSN should become an **AI-native local development operating environment**, not merely a stack launcher. The AI layer plans and verifies work while the Agent remains the mutation boundary.

Core AI capabilities:

1. **AI Feature Planner** — maps user goals to existing capabilities first, then creates scoped versioned plans.
2. **AI Architect** — owns component/provider boundaries, ADRs, portability and failure modes.
3. **AI Data-Flow Analyst** — models persistence, secrets, IPC/network paths and trust boundaries.
4. **AI Security Analyst** — threat-models every mutating/external flow and defines least privilege/fail-closed behavior.
5. **AI Designer** — designs desktop/web/CLI workflows and platform-specific project wizards without bypassing architecture/security.
6. **AI QA Agent** — derives deterministic acceptance/evidence from the approved plan.
7. **AI Performance Analyzer** — defines budgets and profiles startup/runtime/bootstrap/build/network/resource behavior.
8. **AI Implementation Agent** — executes only approved/mapped work after a delta-research preflight.
9. **AI Reviewer / Release Gate** — independently verifies scope, evidence and canonical integration.

## Mandatory lifecycle

`Research -> Plan -> Architecture -> Data Flow -> Security -> Design -> QA -> Performance -> Development`

This order is a state machine, not a suggested checklist. Development is blocked when a required predecessor artifact is missing.

### Why development is last

The goal is to avoid implementation-time wandering. At development start the AI must already know:

- what is being built and what is explicitly out of scope;
- the component/provider boundary;
- data and secret paths;
- threat model and permissions;
- user/API/CLI interaction design;
- exact tests and evidence;
- performance budgets.

It then performs **market-delta research only**. A new official feature, deprecation or security requirement becomes a change proposal; it does not trigger a fresh uncontrolled redesign.

## Architecture

### 1. Planning plane: `.ai/`

Persistent, reviewable repository-local context. Stores lifecycle rules, role contracts, platform catalog, research, plans, ADRs, data flow, security, design, QA, performance, evidence references and change proposals.

### 2. Reasoning/execution planner: `vsn-ai`

Extend the existing structured-intent engine rather than replace it. Future intents can consume approved plan IDs and provider descriptors. Candidate plans remain bounded and validated.

### 3. Provider plane: `vsn-project`

Use the existing `ProjectProvider` SDK as the expansion seam. Avoid adding dozens of framework-specific branches to VSN core. Providers/profiles declare:

- detection evidence;
- runtime/tool requirements;
- starter profiles;
- DB/service options;
- bootstrap plan and dry-run;
- local/SaaS mode;
- network/account/secret requirements;
- health/start/stop/preview contracts;
- acceptance capability.

### 4. Mutation plane: Agent

AI never receives an unrestricted shell shortcut. File/process/runtime/database/network mutations remain permissioned, audited and bounded by Agent/policy/tool contracts.

## Platform modes

Every provider must declare one of:

- `local_native` — application/runtime can primarily run locally;
- `container_recommended` — local development is viable, but a reproducible isolated multi-service profile is preferred;
- `saas_connected` — local source/CLI/dev server exists, but the vendor-hosted platform/account remains part of the loop.

This prevents misleading claims such as "running Shopify/Wix/Webflow fully locally" when official workflows still depend on their hosted platforms.

## New Project experience

A new project should use a capability-driven wizard:

`Category -> Platform/Framework -> Starter -> Runtime -> Database/Services -> Domain/HTTPS -> Git -> AI Project Initialization -> Review -> Create`

### Starter layer

Common profiles are normalized to user intent rather than hard-coded commands:

- Hello World / Minimal
- Standard Web App
- API
- Full Stack
- Ecommerce / Store
- Plugin / Extension / Module
- Theme / Template
- Platform App

A provider maps these normalized profiles to its current official scaffold. The UI can therefore offer a consistent experience while tooling remains platform-specific.

### Base-app guarantee

For a newly supported language/framework, VSN should provide the smallest runnable starter first. A provider is not considered supported merely because VSN can install its runtime. Acceptance must prove scaffold -> dependency install -> start -> health/preview -> stop/cleanup.

## Existing vs future catalog

The existing five certified 02.07 templates remain unchanged. The initial future catalog contains 41 entries spanning PHP/CMS/ecommerce, JavaScript/TypeScript, Python, Ruby, JVM, .NET, Rust/Go frameworks, headless CMS/ecommerce, static/generic projects and SaaS-connected platforms. See `.ai/catalog/platforms.v1.json`.

A catalog entry marked `proposed` is **not implemented support**. It becomes supported only after its own provider plan, implementation, QA/performance/security checks and acceptance evidence.

## Current 2026 research deltas that affect design

- Shopify CLI supports local theme/app development and preview, but the dev store/platform remains authoritative; localhost app development also has feature limitations for platform callbacks. Therefore Shopify is `saas_connected`.
- Wix's current unified CLI creates apps/headless projects and runs a hot-reload local development environment, while sites/hosting remain Wix-managed. Therefore Wix is `saas_connected`.
- Webflow's current CLI brings site/API/code-component/DevLink workflows into local tooling and version control while synchronizing/deploying to Webflow. Therefore Webflow is `saas_connected`.
- WordPress `wp-env` and WooCommerce's current developer guidance explicitly support reproducible local Docker environments. These are `container_recommended`.
- Nuxt 3 reached EOL on 2026-07-31 and Nuxt 4 is current in the reviewed docs. This is exactly why provider implementation must refresh market/tooling state instead of freezing old scaffold assumptions in the core.
- Medusa's current docs support a local open-source backend/admin flow and also advertise AI/agent workflows. It fits `local_native`, with local PostgreSQL and optional service profiles.

## Planning invariants

- Do not alter a frozen acceptance denominator/order to fit this blueprint.
- Do not claim proposed providers are implemented.
- Do not store provider credentials in `.ai/` or catalog JSON.
- Do not let AI auto-approve its own material scope changes.
- Do not bypass provider/tool validation with arbitrary shell commands.
- Prefer official scaffolds and source verification over copying stale templates into VSN.
- Preserve rollback/cleanup when bootstrap fails.
- Treat external SaaS mutation and tunnels as explicit security/data-flow concerns.

## Future implementation sequence

When this architecture reaches an approved product package, implement in capability slices rather than 41 one-off branches:

1. provider/catalog schema + registry/discovery;
2. normalized starter-profile contract and Hello World acceptance;
3. runtime/tool prerequisite resolver;
4. local-native provider adapters;
5. container-recommended service profiles;
6. SaaS-connected account/secret/tunnel adapters;
7. New Project UI/CLI flow;
8. `.ai/` plan binding and drift preflight in `vsn-ai`;
9. cross-provider QA/performance/security certification.

Each slice must be separately planned and accepted when it becomes active work.
