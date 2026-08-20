# SCIM 2.0 provisioning runbook — VSN 0.15

VSN exposes a bounded SCIM 2.0 provisioning baseline behind the scoped `control.scim.manage` permission.

## Resources

- Users: list/filter/create/read/replace/PATCH/disable/delete
- Groups: list/create/read/replace/PATCH/delete
- Bulk: up to 100 bounded POST/PUT/PATCH/DELETE operations with a 1 MiB request cap
- Reconciliation: detects dangling group members and duplicate external IDs; optional repair removes only dangling memberships

User/group GET responses carry deterministic `meta.version` values and `ETag`. PUT/PATCH/DELETE honor `If-Match`; Bulk operations can carry their resource `version`, which is applied as per-operation `If-Match`.

Security-sensitive User role/disable/password/TOTP changes revoke sessions. A scoped provisioning principal cannot assign a role containing permissions outside its own delegation. SCIM-created users are not given a usable plaintext local password through the API.

Current limitations: full RFC conformance testing, Bulk `bulkId` cross-operation references, SCIM PATCH path grammar depth, persistent monotonic resource versions, sorting, Groups policy semantics and automatic authoritative-directory reconciliation remain partial.
