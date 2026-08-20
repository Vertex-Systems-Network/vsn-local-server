# SAML ACS security model — VSN 0.15

VSN 0.15 enables an SP-initiated SAML 2.0 ACS baseline only when a configured `xmlsec1` verifier and the configured IdP X.509 certificate are available. The flow fails closed if XMLDSig verification cannot run or fails.

The provider policy separates `entity_id` (the VSN/SP AuthnRequest issuer) from `idp_entity_id` (the expected identity-provider issuer). Existing policies must add `idp_entity_id` before SAML login can validate.

After XML signature verification, VSN validates the one-time RelayState transaction, response and assertion IDs, duplicate XML IDs, Destination, InResponseTo, Success status, IdP issuer, Audience, assertion Conditions, SubjectConfirmation recipient/time window, NameID and bounded attributes. DTD/entity declarations are rejected. Unknown `(provider_id, subject)` identities are never linked to a local account by email alone; an administrator must explicitly link the verified external subject.

The IdP certificate is referenced by environment-variable name (`x509_certificate_pem_env`) and is not embedded in the policy document. Pending SAML login transactions use the existing shared auth-transaction store when PostgreSQL HA mode is enabled.
