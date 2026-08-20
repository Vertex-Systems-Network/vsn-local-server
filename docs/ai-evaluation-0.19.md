# AI deterministic evaluation — 0.19

Evaluation cases contain an intent, expected VSN tool commands and an optional `forbid_mutations` assertion. The evaluator runs the deterministic planner, verifies command presence, mutation policy and the invariant that unrestricted shell remains disabled.

This provides a regression harness for the structured tool planner. It does not claim production model-quality evaluation or autonomous unrestricted execution.
