# P0/P2 Threat Model — STRIDE Baseline

This is the first concrete threat-model pass. It must be extended whenever P3 adds mutating machine operations and again before remote access ships.

| Threat | Example | Current mitigation | Remaining work |
|---|---|---|---|
| Spoofing | Fake process impersonates Agent | Authenticated response HMAC; request nonce binding | Native pipe peer identity; signed binaries |
| Spoofing | Fake machine identity | Ed25519 device key; device ID derived from public key | Cloud pairing/revocation |
| Tampering | Audit file edited | SHA-256 hash chain + Ed25519 signature per event | Remote append-only audit sink |
| Tampering | IPC request modified | HMAC-SHA256 over canonical request | Policy signatures for remote commands |
| Repudiation | User denies sensitive action | Signed Agent audit events | User/session identity from control plane |
| Information disclosure | IPC key readable by another Windows account | Explicit ProgramData ACL; fail closed on ACL failure | ACL tests in Windows CI |
| Information disclosure | Secrets appear in logs | Security rule forbids secret audit metadata | Structured redaction layer |
| Denial of service | Oversized local IPC frame | 1 MiB bounded frame reader | Rate limits / connection quotas |
| Denial of service | Nonce replay flood | Replay cache + expiry | Per-peer throttling |
| Elevation of privilege | UI asks Agent for admin action | Agent authorization boundary + default-deny policy; high-risk permissions absent from baseline principal | Approval flow / narrow privileged broker |
| Elevation of privilege | Malicious provider executes arbitrary OS command | Provider permissions declared; baseline Agent uses LocalService on Windows | Out-of-process sandbox/provider broker |

## Security decisions

1. No custom cryptographic algorithms.
2. Device private keys are not derived from hardware identifiers.
3. The Cloud will not need inbound Agent machine ports.
4. Localhost is not automatically trusted.
5. Provider code does not receive implicit full Agent privileges.
6. AI will consume structured Agent tools and policy checks rather than an unrestricted shell.

## 0.5 additions

### Lost result acknowledgement / duplicate execution
Mitigation: leased delivery keeps the command until completion; Agent caches the semantic result before upload. A redelivered command ID is not executed again. The cached result is re-signed with a fresh result nonce/timestamp and the Control Plane idempotently accepts a matching completed command/session.

### Scoped IAM privilege escalation
Mitigation: a non-bootstrap IAM principal with `control.iam.manage` may only create roles and tokens whose permission set is a subset of its own. The Agent independently rejects baseline high-risk delegated permissions.

### Oversized remote results / state growth
Mitigation: per-surface output caps, a 1.5 MiB Agent remote-payload cap, 2 MiB Control Plane JSON body cap, and bounded result retention.

### Database query masquerading as read-only
Mitigation: stacked statements are rejected, only conservative query forms are accepted, risky clauses/functions are denied where known, DB session read-only hints are applied, and remote DB query is locally disabled by default. Residual risk remains for vendor/user-defined functions; use server-enforced read-only DB roles.
