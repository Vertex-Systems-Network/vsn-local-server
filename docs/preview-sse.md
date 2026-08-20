# Local preview SSE relay — 0.16

The Agent can relay a bounded localhost Server-Sent Events endpoint through the authenticated stream channel. The upstream target remains `127.0.0.1:<port>` and the request is GET-only.

Controls: redirects disabled, `Accept: text/event-stream`, optional `Last-Event-ID`, 5–300 second duration, 16 MiB total stream cap, 64 KiB read chunks, maximum 32 streams and a bounded queue that fails closed on backpressure rather than blocking an Agent worker indefinitely.

This is not a generic open proxy. Full WebSocket proxying, arbitrary cookies, asset rewriting and hot-reload URL rewriting remain future tunnel work.
