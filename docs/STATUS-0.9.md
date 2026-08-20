# VSN roadmap status — 0.9.0

Machine-readable source of truth: `docs/roadmap-status.json`.

- Done / usable baseline: 8 phases
- Meaningful partial: 22 phases
- Pending: 1 phase (`P30` stable production release)

The count is not a completion percentage. Most partial phases still require native builds, end-to-end integration, production security testing, operational recovery and platform packaging before a stable release.

### Main 0.9 maturity gains

- P8/P9: structured native PostgreSQL/MySQL writes and metadata plus Redis writes.
- P10: corresponding Desktop controls, transfer verification and wait-read terminal UX.
- P12/P29: stale Control Plane SQLite writers now fail through generation CAS.
- P15/P16: bounded terminal wait-read and resumable upload status/digest.
- P18: advanced bounded localhost HTTP request baseline, still local-only for mutation-capable requests.
- P23: artifact upload, CURRENT/PREVIOUS release selection and rollback on existing VPS workspaces.
- P28: single-use recovery codes and OIDC PKCE authorization-start primitives.
