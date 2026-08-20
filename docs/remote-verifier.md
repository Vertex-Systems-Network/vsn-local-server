# P12 remote verifier baseline

`vsn-remote` implements the cryptographic boundary that must sit in front of any future network transport.

Implemented:
- device enrollment proof signed by the Agent device key
- enrollment proof verification
- remote command canonicalization
- pinned control-plane public-key verification
- command binding to a specific device
- principal/session/permission fields included in signed payload
- maximum five-minute command TTL
- clock-skew validation
- bounded replay cache keyed by command ID

Not implemented:
- transport/gateway/relay
- cloud account authentication
- mapping remote permission strings to local authorization/approval policy
- key rotation/revocation distribution

A future gateway is transport only. Passing gateway authentication must never bypass this Agent-side verifier or the local policy layer.
