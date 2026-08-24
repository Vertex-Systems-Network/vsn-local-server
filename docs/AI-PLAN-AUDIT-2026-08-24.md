# AI Planning Adversarial Audit — 2026-08-24

Scope: PR #91 AI-native planning/governance layer only. Product PKG-02 implementation and frozen 27-task sequence are out of scope.

Baseline audited canonical main: `d9c5aa245efb0d20957b4eb840e29a4f95a520d2`.

## Audit method

The plan was reviewed as an adversarial state/authority system rather than only as documentation. The audit looked for paths where an AI or future contributor could obtain stale authority, skip a stage, rewrite history, widen permissions, consume prompt-injected instructions, misrepresent proposed platforms as supported, bypass normalized starter UX, or accept weak evidence.

Current external guidance was also checked for agentic-AI trust issues. The resulting controls intentionally follow least privilege, segregation of untrusted external content, human/governance approval for privileged actions, provenance and machine-readable evidence.

## Findings and disposition

### A1 — Cached `.ai` state could become stale and override live roadmap state — FIXED

Risk: the original `.ai/state.json` stored active task/progress as if it could be resumed later. After a product merge that snapshot would be stale.

Fix: schema v2 makes the snapshot a historical `audit_baseline` only. Live canonical main/state must be refreshed before every stage and immediately before mutation. Any mismatch is fail-closed: `stop_and_reconcile`.

### A2 — Approved plan could be edited retrospectively — FIXED

Risk: an agent could change the plan after code exists and then claim traceability.

Fix: feature manifests bind plan path + SHA-256 + canonical base SHA + approval reference. Development verifies the frozen digest. Material change creates a new version/addendum; digest mismatch blocks mutation.

### A3 — `not_applicable` could bypass lifecycle stages — FIXED

Risk: a future agent could mark Security/QA/Data Flow N/A without proof.

Fix: stage-skip policy requires artifact/rationale and independent decision reference. Security and QA cannot be N/A for mutating product work; Data Flow cannot be skipped when files/processes/IPC/network/persistence/secrets/accounts/external services are touched.

### A4 — AI/self/sub-agent could approve its own widened scope — FIXED

Risk: self-generated approval text or delegation could become authority.

Fix: independent approval reference is mandatory for material changes, stage skips and privileged external mutations. Delegated scope may only narrow; sub-agents cannot approve parent changes.

### A5 — External research/repository content could prompt-inject the implementation agent — FIXED

Risk: docs, issues, PR comments, logs, package metadata or web pages could contain instructions that an AI follows as commands.

Fix: `.ai/governance/TRUST-BOUNDARIES.md` establishes a data-vs-authority boundary. External/retrieved text never overrides governance or grants tool permission. Commands from sources are candidates only and must be mapped to the approved plan/provider contract before execution.

### A6 — Secret values could leak into persistent AI context/evidence — FIXED

Risk: repository-local AI files are durable and reviewable; embedding credentials there would spread them.

Fix: secrets are represented only by handles/purpose. CI scans `.ai/**` for high-confidence private-key/GitHub/AWS credential patterns and bounds AI-context file sizes. Logs/evidence must redact credentials and authorization headers.

### A7 — Platform `starter_profiles` were not actually normalized — FIXED

Risk: catalog entries used many upstream-specific labels (`appstarter`, `block`, `hydrogen`, `web-api`, etc.) while the product direction promised normalized user intent. A UI could accidentally consume catalog labels directly.

Fix: `.ai/catalog/starter-intents.v1.json` defines normalized intents and an explicit alias map. Catalog labels are non-executable planning metadata. CI requires every current catalog/policy profile to map to a known normalized intent; unknown profiles block implementation.

### A8 — Proposed `official_tooling` text could be mistaken for verified execution commands — FIXED BY AUTHORITY BOUNDARY

Risk: only representative/high-risk platforms have current primary-source research in this planning PR. Other proposed entries are useful roadmap hypotheses but are not all implementation-verified.

Fix: entire platform catalog remains `blueprint_only`; `proposed` entries cannot become execution/support claims. Provider implementation requires a fresh official-source market-delta pass and its own plan/evidence. Catalog profile/tooling text alone is not execution authority.

### A9 — Acceptance could rely on unrelated green CI or synthetic merge source — FIXED

Risk: a green job can be unrelated, and PR workflows may certify synthetic merge refs instead of exact source heads.

Fix: `.ai/governance/EVIDENCE.md` requires criterion-to-evidence mapping, exact source SHA, run/job or command transcript, negative checks, cleanup, artifact digests where applicable, and explicit binding when CI executes a synthetic merge ref.

### A10 — AI governance workflow itself had weak supply-chain/runtime assumptions — FIXED

Risk: mutable checkout tag, persisted credentials, Python `assert` semantics and unbounded AI context weakened a governance gate.

Fix: checkout is pinned to verified `actions/checkout` v4.2.2 commit `11bd71901bbe5b1630ceea73d27597364c9af683`, credentials are not persisted, job has a timeout, checks use explicit fail functions instead of `assert`, and `.ai` file size/secret rules are machine-enforced.

## Residual governance boundary

A repository-hosted workflow can be edited in the same pull request that it evaluates. No workflow can fully make itself tamper-proof from inside the same repository. Therefore changes to `.ai/**` or `ai-planning-governance.yml` still require independent human/governance review and should ultimately be protected by repository rulesets/CODEOWNER-style policy. CI remains a strong guard, not the root of authority.

This residual limitation does not authorize auto-merge or self-approval. The release/merge decision must still inspect the exact PR head and required independent gates.

## Merge acceptance for PR #91

Merge only if the final exact head satisfies all of the following:

- diff remains planning/governance only;
- canonical product machine state/frozen PKG-02 sequence is not modified;
- AI Planning Governance passes;
- Repository Governance passes;
- PKG-02 Acceptance Sequence passes;
- PKG-02 02.07 template/bootstrap regression passes;
- PR remains mergeable against current live main;
- any pre-existing unrelated CI failures are distinguished from this PR and do not replace required gates;
- PR body identifies the final exact head and audit hardening.

After merge, re-read canonical main and machine state before continuing active PKG-02 work.
