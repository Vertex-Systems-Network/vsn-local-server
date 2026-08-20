# Native PostgreSQL cancellation and read-only transactions — VSN 0.17

The existing durable CLI database jobs remain the restart-safe external query path. VSN 0.17 adds a second, loopback-native PostgreSQL control path:

- server cancellation jobs use the PostgreSQL driver's cancellation token;
- status distinguishes `cancel_requested` from final cancelled/failed/completed state because cancellation is asynchronous/racy;
- each job begins a read-only transaction and applies a 30 second statement timeout;
- explicit read-only transaction sessions use `BEGIN READ ONLY`, a 15 second statement timeout, 60 second idle-in-transaction timeout, a 10–60 second VSN TTL, maximum 100 statements and maximum 32 concurrent sessions;
- only VSN's existing read-only SQL validator is accepted;
- transaction sessions are not reconstructed after Agent restart.

Remote execution still requires `database.query` and the device-local `allow_remote_database_query` opt-in.
