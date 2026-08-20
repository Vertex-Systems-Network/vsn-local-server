# SAML federation policy — VSN 0.15

VSN now has a fail-closed SP-initiated SAML ACS baseline. Provider policy separates `entity_id` (VSN/SP issuer) from `idp_entity_id` (expected IdP issuer), pins the IdP signing certificate through `x509_certificate_pem_env`, and requires HTTPS SSO plus bounded ACS/audience/attribute settings.

The ACS path is enabled only when the external `xmlsec1` verifier is available and signature verification succeeds. VSN then validates RelayState/InResponseTo, duplicate IDs, Destination, Success status, IdP issuer, Audience, assertion and SubjectConfirmation time windows, NameID and bounded attributes. Unknown verified subjects return `mapping_required`; email equality never auto-links an identity.

See `docs/saml-acs.md` for the detailed validation sequence. Broad IdP interoperability, signed AuthnRequest support, Single Logout and federation lifecycle testing remain pending.
