# Out-of-process updater helper — 0.16

`vsn-updater-helper` accepts one bounded JSON request on stdin and returns one JSON response. Supported operations are apply, rollback, status and stale-lock recovery.

All apply/rollback entry points now use `.vsn-update/apply.lock` created with exclusive create semantics. The lock contains PID, creation timestamp and helper version. It is removed only by the lock guard after the operation. A competing updater fails closed.

Stale-lock recovery is explicit: `confirm_recover=true` is required and a lock younger than ten minutes is never removed. A malformed lock is not automatically deleted. The helper performs no download; staged artifacts must already be SHA-256 pinned and the existing update apply transaction retains backup/rollback behavior.
