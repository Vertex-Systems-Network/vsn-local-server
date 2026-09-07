# VSN Local Server Privacy Notice

## Status and scope

VSN Local Server is in active development. VSN 1.0 is not yet certified or stable.

This notice describes the privacy rules that apply to official VSN Local Server builds and to features that may communicate with networked systems. It does not convert an unsigned development build into a production release and does not mark PKG-03 `03.22` complete.

## User-directed network communication

VSN Local Server is a local development/server platform, but some capabilities are intentionally network-capable. Examples include remote SSH/workspace operations, control-plane enrollment, update/download operations, and provider integrations when those features are configured or invoked.

Network communication that is necessary to a user-selected feature must be limited to the systems, providers, endpoints or resources that the user or operator selected or configured for that feature.

This project does **not** claim that the program never communicates with networked systems. A blanket “no network transfer” statement would be inaccurate for a platform that includes remote-management and provider capabilities.

## Undirected data collection

A signed public release must not silently collect user data and transfer it to systems that were not specified or requested by the user or operator.

If a future release introduces telemetry, analytics, crash reporting, diagnostics upload, account synchronization, cloud storage, or any other transfer to a service that is not directly selected by the user, that behavior must be documented before release. Where applicable, the installer/application must present the relevant privacy information and provide the controls required by the applicable signing policy and law.

Until those disclosures and controls are reviewed, such a feature is not eligible for the governed production-signing path.

## Information handled by user-selected features

Depending on the feature the user chooses, VSN Local Server may process technical information such as:

- host names, addresses, ports and SSH connection settings for remote targets;
- workspace, provider, region, machine or runtime configuration chosen by the operator;
- project/runtime/database configuration required to operate the selected local or remote environment;
- release/update metadata required to identify and retrieve an explicitly requested build or update; and
- authentication material or credentials supplied for a configured provider or remote system.

Secrets and credentials must be handled by the relevant security/configuration boundary and must not be intentionally published in repository logs, release notes, public artifacts, issue comments or code-signing evidence.

## Code signing and SignPath

The SignPath Foundation route described in the project’s Code signing policy is a build/release-signing service. It is not an application telemetry service. Production signing requests must contain only the governed build/release information and artifacts required for signing and verification, never end-user secrets or private signing-key material.

## Third-party services and components

A feature that uses an external provider or service is also subject to that provider’s own privacy terms. Users and operators are responsible for choosing and configuring those services. Release documentation must identify third-party services that materially affect end-user privacy when they are part of an official supported workflow.

## Installation and uninstallation

Official installable releases must document or provide an uninstall path. System-affecting behavior and any privacy-relevant network behavior must not be hidden merely because a build is code signed.

## Security and incident handling

Suspected disclosure of secrets, credentials, release authority, signing-service access or private user information must be treated as a security incident. Affected publication/signing activity must stop until the relevant authority is contained and the release can again satisfy the project’s security and signing policies.

## Changes to this notice

Material privacy changes must be reviewed as repository changes before they become part of a signed public release. The current public source repository is the authority for this notice.