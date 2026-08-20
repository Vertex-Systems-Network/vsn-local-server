# VSN Dev Platform 0.37 — Certification State Governance MAX

P0–P29 remain 100% source-closed. P30 remains evidence-driven.

## 0.37 governance additions

- Import-ready P30 result bundles with bounded safe ZIP extraction and candidate/version/hash verification.
- Evidence Journal v3 with states: `active`, `revoked`, `quarantined`, and `superseded`.
- Deterministic authoritative-ledger rebuild from neutral candidate state plus all active stored evidence fragments.
- Incident quarantine/unquarantine without deleting evidence.
- Permanent revoke/explicit restore lifecycle.
- Controlled supersession: an old evidence bundle can be retired only in favor of an already-active replacement.
- Evidence-aging policy report with `fresh`, `expiring_soon`, `expired`, and invalid-timestamp classifications.
- Disaster-recovery checkpoints containing the journal, active evidence store, and authoritative ledger; checkpoint restore is candidate/version bound and hash verified.
- Status synchronization checks keep evidence report, roadmap, scoreboard, fastest path and runner plan aligned with the authoritative ledger.
- Evidence merge semantics prevent newer `blocked`/`pending` runner attempts from erasing substantive PASS/FAIL evidence.

None of these source/governance changes count as P30 PASS evidence by themselves.

## Compatibility / closure anchors

This remains the **P30 Runner-Pack Execution** and **P30 handoff/resume sprint** lineage. The exact overall source-plus-certification score at 0/21 is **98.9032%**. Every evidence ledger and governance journal is bound by `candidate_id`; evidence from **different source candidates** is rejected rather than mixed.
