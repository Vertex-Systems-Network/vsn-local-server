# PERSONAL-USE-UNSIGNED-DEFERRAL-V1

Date: 2026-10-17
Disposition: APPROVED
Engineering change classification: CONTRACT CHANGE (execution sequencing only)
Approval scope: PROJECT
Approval reference: `conversation:user-2026-10-17-personal-use-without-signing-approval`
Source canonical main: `b8d82b9d5dbf2cf60a8d7876bade5e4d0c8c6b3c`

## Problem

03.22 (Authenticode signing) depends on an external code-signing approval (Microsoft / SignPath Foundation) that has not been granted. Draft PR #231 and the SignPath preparation PRs (#209, #217–#221) have been idle since mid-September. Because 03.23–03.25 depend on 03.22, all PKG-03 progress, and everything after it, is stalled on a third party.

The owner's immediate goal is **personal use** of the Windows build. Public distribution comes later. Personal use does not need a trusted publisher signature.

## Approved change

1. **Unsigned personal builds are an approved use.** The owner may build, install and run the unsigned NSIS/MSI bundles produced by `.github/workflows/pkg03-0302-windows-bundle-build.yml` (`workflow_dispatch`) on machines they control. See `docs/PERSONAL-INSTALL-WINDOWS.md`.
2. **03.22 is a public-release gate, not a development gate.** Signing is deferred until the owner decides to distribute to others. No further work on SignPath/Microsoft approval is required to continue development.
3. **Downstream work continues in the PREIMPLEMENTATION lane** (`.ai/governance/FAST-DEVELOPMENT.md`). 03.23 and 03.24, and PKG-04+ work whose interfaces do not depend on a trusted signature, may be implemented against unsigned artifacts with the `NON_ACCEPTANCE_PREIMPLEMENTATION` marker, naming 03.22 as the canonical blocker.

## What does not change

- The canonical PKG-03 tracker, dependency DAG, denominator and statuses are unchanged: PKG-03 stays 21/25 with 03.22 READY. `scripts/pkg03-acceptance-sequence.py` continues to validate the same graph.
- No task becomes DONE from unsigned, test-signed or self-signed evidence.
- No signature, certificate, provider approval or Store review is fabricated.
- No product/runtime/installer behavior changes.
- Before any public release, 03.22 → 03.25 must still be accepted with genuine evidence.

## Follow-up housekeeping (separate changes)

- Consolidate the overlapping 03.22 draft PRs (#209, #217–#221, #231) into one canonical PR or mark them superseded.
- Refresh the stale `.ai/current-work.json` and `VSN_AI_PROJECT_STATE.md` projections (they still describe 03.12 / PKG-03 0/25).
