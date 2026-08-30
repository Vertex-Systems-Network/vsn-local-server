# PKG-03 03.17 — Windows Installer Uninstall Cleanup & Preservation Contract v1

Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.17`
Linear: `ABD-92`

## Contract

A Windows uninstall is accepted only when it satisfies both sides of the boundary:

**Cleanup:** every artifact proven to be owned by the installed VSN package is removed through the genuine installer lifecycle.

**Preservation:** user-created and mutable runtime data outside the install root remains byte-identical unless an explicit, evidence-backed ownership rule authorizes removal.

Success exit code alone is insufficient.

## Required owned cleanup

For the applicable package format:
- `VSN Dev Platform.exe`, `bin\vsn.exe`, `bin\vsn-agent.exe` and other accepted package-owned payload under the selected install root;
- owned Start Menu/Desktop shortcuts and application registration;
- current-user or machine ARP entry / MSI ProductCode registration;
- `VSN-Agent` SCM registration for per-machine packages;
- installer-owned directories only when empty and proven not to contain preserved data.

## Required preservation

Before uninstall the certification harness seeds deterministic markers in:
- resolved mutable data root;
- resolved mutable config root;
- a dedicated user workspace/project location outside the install root;
- one unrelated adjacent outside-boundary location.

After uninstall each marker must exist at the same canonical path with identical bytes and SHA-256. Where ACL/security descriptors are meaningful, the descriptor must not be broadened or unexpectedly rewritten.

## Security-state rule

`%PROGRAMDATA%\VSN\security\ipc.key` is security state, not an ordinary user-data marker. It must be separately inventoried. Removal is allowed only if exact-head evidence establishes exclusive product ownership and safe cleanup. Otherwise the state is preserved and the task fails closed if the frozen product contract requires a decision that cannot be proven without change control.

## Containment rule

Uninstall must not recursively traverse a junction/reparse point from an owned cleanup path into an outside preserved fixture. Any such outside deletion is a task failure.

## Protected-state rule

Firewall, hosts file, resolver configuration and trust-store state must remain byte/security equivalent to the 03.13 protected boundary.

## Nonclaims

This contract does not cover interrupted rollback, live-running process coordination, Restart Manager, reboot semantics, unattended deployment, signing, updater/recovery, or cross-platform release.
