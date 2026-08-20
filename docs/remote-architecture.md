# Remote architecture baseline — P12 design

## Connection direction
Agent -> Gateway/Relay is outbound. No inbound machine port is required by default.

## Enrollment
A device presents its public key, device ID, pairing nonce and a proof signature. The control plane stores only the public identity and account/device association.

## Commands
Remote commands are bound to device, principal, session, permission, issue/expiry times and a unique command ID. Cloud authorization is necessary but not sufficient: the Agent performs local policy enforcement again.

## Relay
Relay transports encrypted session traffic and should not be treated as an authorization authority. Direct peer connectivity can be introduced later, with relay fallback.

## Not implemented in this milestone
Persistent cloud connection, account service, gateway, relay, browser terminal, remote file transfer, remote DB proxy, preview tunnels and remote desktop remain future implementation work.
