# Interactive localhost WebSocket preview — VSN 0.17

VSN 0.17 adds a bounded bidirectional WebSocket preview bridge for development servers reachable only at `ws://127.0.0.1:<port><absolute-path>` from the attached Agent.

Security boundary:
- browser relay requires `project.edit`, not only `project.view`;
- device config `allow_remote_preview_interactive` defaults to false and must be explicitly enabled locally;
- DNS/external URLs are not accepted;
- each message is capped at 256 KiB, each session at 16 MiB total transfer and 300 seconds;
- channel queues are bounded and slow peers fail closed rather than accumulating unbounded memory;
- snapshot and SSE preview remain the lower-privilege read-only paths.

The bridge transports text/binary/ping/pong/close messages. It is not a generic TCP tunnel. Cookie/HTTP asset rewriting and arbitrary cross-origin browser credential forwarding are not provided by this path.
