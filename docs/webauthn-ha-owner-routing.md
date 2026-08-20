# WebAuthn HA owner routing — VSN 0.17

VSN does not serialize opaque in-flight WebAuthn registration/authentication cryptographic state into shared PostgreSQL unless the credential library explicitly supports a reviewed serialization model.

Instead, begin requests pin each ceremony to the owning Control Plane instance and store only bounded shared owner metadata. Begin responses include `owner_instance_id` and `owner_endpoint`; wrong-node finish requests return owner-routing guidance.

0.17 additionally exposes:

`GET /v1/auth/passkey/owner/{transaction_id}?kind=registration|authentication`

The lookup is rate-limited and returns the live shared owner record while the transaction is unconsumed and unexpired. Clients can use it before the finish ceremony after a load-balancer/reconnect event. If the owning instance itself restarted and lost the opaque ceremony state, VSN fails closed and the user starts a new ceremony.
