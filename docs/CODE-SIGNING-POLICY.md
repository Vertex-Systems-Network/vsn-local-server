# VSN Local Server Code Signing Policy

## Scope

This policy applies to Windows release artifacts distributed by the VSN Local Server project from the official repository:

- https://github.com/Vertex-Systems-Network/vsn-local-server

VSN Local Server is open-source software licensed under GNU GPL v3. Third-party dependencies remain subject to their respective licenses.

## Signing objective

Code signing is used to let users verify that an official Windows release artifact:

1. originates from the VSN Local Server release process;
2. has not been modified after signing; and
3. can be traced to a specific source revision and release record.

Signing is not used to bypass operating-system security controls or to imply that unsigned development builds are production releases.

## Eligible artifacts

Only artifacts produced by the governed release pipeline may be submitted for production signing. Eligible Windows artifacts may include:

- VSN Desktop installer packages;
- VSN MSI packages;
- VSN CLI executables;
- VSN Agent executables; and
- other explicitly documented Windows release binaries added to the signed-artifact policy in the future.

Development, pull-request, local, test, fuzzing and ad-hoc artifacts are not production-signing candidates.

## Build provenance

Production signing requests must originate from an approved GitHub Actions release workflow using GitHub-hosted runners and must be bound to:

- the exact Git commit SHA;
- the exact release/tag or governed release request;
- the immutable unsigned artifact digest;
- the build workflow/run identity; and
- the declared artifact configuration.

A signed artifact must correspond byte-for-byte to the approved unsigned artifact except for the expected signing transformation.

## Signing service and private-key custody

For the SignPath Foundation route, production private signing keys are controlled by the signing provider and are never stored in this repository, in pull requests, in GitHub Actions artifacts, in issue comments or in ordinary repository variables.

The project must not export, copy, log or persist signing private-key material.

## Approval policy

Production signing is fail-closed. A signing request is accepted only when all required policy checks pass.

A designated project maintainer must manually approve each production signing request where the configured signing policy requires manual approval. The person approving a release must verify the source revision, release intent and artifact identity before approval.

GitHub and SignPath accounts used for release/signing administration must use multi-factor authentication.

## Separation of responsibilities

The release process separates these responsibilities where supported by the service and repository controls:

- **Build authority** — produces immutable unsigned artifacts from an exact source revision.
- **Signing requester** — submits only governed release artifacts to the signing service.
- **Signing approver** — reviews the release intent and artifact identity before signing.
- **Release publisher** — publishes only artifacts that passed post-signing verification.

No single workflow change on an untrusted pull-request head is sufficient to obtain production signing authority.

## Verification requirements

Before a signed artifact is treated as an official release artifact, the release pipeline must verify at least:

- Windows-native Authenticode/signature validity where applicable;
- expected signer/publisher identity as supplied by the approved signing service;
- cryptographic digest of the signed artifact;
- source/release/run binding;
- package/application identity preservation;
- deterministic rejection of a modified/tampered copy; and
- absence of private signing-key material in repository output, logs and artifacts.

Failure of any required verification keeps the release unsigned/unaccepted.

## Release transparency

Official releases must identify the relevant source revision or tag and provide sufficient release information for users to associate distributed binaries with the source repository.

Where the release process publishes checksums, those checksums must be computed from the final signed artifacts that users download.

## Incident response

If signing credentials, signing-service access, release authority or a signed artifact is suspected to be compromised:

1. stop new signing and release publication;
2. revoke or disable affected signing/service credentials where applicable;
3. investigate the exact affected source, build, signing and release records;
4. remove or clearly mark affected downloads when necessary;
5. rotate/re-establish signing authority before resuming production signing; and
6. document the remediation in the project security/release records without disclosing secret material.

## Changes to this policy

Material changes to signing authority, artifact scope, signing providers or approval rules must be reviewed as repository changes before becoming release authority. A provider migration does not by itself mark a PKG-03 acceptance task complete; the applicable certification evidence must still pass.
