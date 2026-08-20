# P2 Secure Agent Status

P2 remains the security foundation; later milestones now build on it.

## Implemented
- Ed25519 Agent identity and public-key-derived ID
- OS credential-store private-key persistence
- Windows shared IPC credential with explicit ACL
- HMAC-SHA256 authenticated local request/response envelopes
- timestamp + nonce replay rejection
- bounded IPC frames
- Windows Service integration using `LocalService` rather than `LocalSystem`
- Linux systemd user-service deployment template
- macOS LaunchAgent deployment template
- signed SHA-256 hash-chained JSONL audit log with file locking
- secure-update architecture design

## Transport hardening still pending
Native Windows named pipes and Unix-domain sockets remain future work. Loopback TCP is still the local transport, protected by request/response authentication.

## Important update from P3
The command surface is no longer read-only. Mutating service operations now pass the policy engine and are additionally restricted to `VSN-*` managed service names. High-risk permissions remain absent from the baseline local principal.
