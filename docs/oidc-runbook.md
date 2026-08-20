# OIDC runbook — 0.12

Each provider policy defines HTTPS issuer/client/redirect/scopes, mandatory PKCE, optional explicit authorization endpoint, optional `client_secret_env`, and whether the provider is considered MFA-assured for administrative sessions.

## Login

1. `/v1/auth/oidc/begin` creates server-side state, nonce and PKCE verifier/challenge.
2. Browser goes to the provider authorization URL.
3. Provider redirects to `/v1/auth/oidc/callback`.
4. Control Plane consumes the one-time state, discovers provider metadata/JWKS with HTTP redirects disabled, exchanges the code using the stored PKCE verifier, and verifies the ID token authenticity plus issuer/audience/nonce.
5. Only an exact `(provider_id, subject)` mapping may create a VSN session. Email is informational and never auto-links an account.
6. Unknown identities return `mapping_required`; an administrator uses `/v1/admin/auth/oidc/link` to create the explicit mapping, then the user repeats login.

Confidential client secrets are read from the environment variable named by `client_secret_env`; they are not stored in the auth policy JSON.
