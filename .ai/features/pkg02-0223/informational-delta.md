# 02.23 Informational Evidence Delta — Windows Line Endings

Feature: `pkg02-0223-test-dns` v1.0.0  
Date: `2026-08-24`  
Class: `informational`

The first exact-head 02.23 workflow run failed before product certification because Windows checkout converted the frozen Markdown plan from repository LF bytes to CRLF bytes. The canonical LF SHA-256 recorded in the manifest is correct: `cc9b7b503c87d4ede7fb625e080500049fd0d3c4f0d8cdd956f2d7747c3db9ed`. The Windows CRLF checkout of the same text produced `5c8864cbb8d94a5bbcec0087a37dab732a06c86ffcb3e414dfd7237967808a33`.

No frozen plan bytes, scope, behavior, interface, permission, acceptance criterion, dependency, or product source are changed. The remediation adds path-scoped `.gitattributes` entries forcing LF checkout for the 02.23 lifecycle artifacts so their recorded SHA-256 values remain platform-stable.

Independent local recomputation of the canonical LF source text also confirmed the recorded research, lifecycle-review, and development-preflight SHA-256 values.

The failed run is not acceptance evidence. All acceptance must run again on the new exact source head.
