# Updater apply / rollback transaction — VSN 0.17 (introduced in 0.16)

The update verifier has an explicit single-file apply/rollback transaction and a dedicated `vsn-updater-helper` executable for the out-of-process boundary. Apply requires `confirm_apply=true`, a safe relative target under a canonical install root, a pre-downloaded staged artifact, and the expected SHA-256 digest.

The helper copies and re-verifies the staged artifact, preserves target permissions where possible, moves the current target to `.vsn-update/previous`, replaces it with the pending file, fsyncs the result/state, and attempts to restore the previous target if replacement fails. Rollback requires explicit confirmation and an existing previous backup.

The API does not download code, bypass manifest verification, or claim that a running Windows executable can replace itself. All Core/helper apply and rollback entry points acquire the same exclusive `.vsn-update/apply.lock`. A Windows self-update must launch the apply operation from the separate updater helper after the target executable has exited; file-lock failures remain fail-closed. Status and explicit stale-lock recovery are available without manually deleting lock files.
