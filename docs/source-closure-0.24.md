# Source closure — 0.24

Source-closed phases now include P0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11, P12, P13, P14, P15, P16, P17, P18, P19, P20, P21, P22, P24, P26, P27, P28, and P29.

New closures in this batch:
- P5: loopback `.test` DNS, elevated OS resolver apply/status/remove, hosts fallback, local CA/certificates, Caddy reverse-proxy config and validated hot reload.
- P16: workspace containment, resumable chunk upload/download, Agent-side final SHA-256, browser direct-to-disk large downloads, >256 MiB streaming uploads without whole-file browser buffering.
- P18: bounded localhost snapshot HTTP, interactive HTTP methods with header/body bounds, SSE, and WebSocket relay; interactive paths require project.edit plus device-local opt-in.
- P26: ToolPlan policy gate, model-adapter boundary, deterministic evaluation harness, bounded persistent telemetry, mutation confirmation, recursion and unrestricted-shell denial.
- P27: signed/trusted registry, SHA-pinned packages, revocations, channels, publisher governance, submission/review lifecycle, and safe update resolution.

P23, P25, and P30 remain non-closed. P3, P6, P12, P19, P21 and P29 remain closed from earlier batches.
