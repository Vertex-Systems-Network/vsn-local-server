# VSN Extension SDK baseline

Extensions are manifests plus one or more provider manifests. Core code must interact with declared capabilities, not vendor names.

Security baseline:
- permissions are explicit and default-denied
- extension signatures are a reserved field; unsigned third-party code must not become trusted merely because a manifest parses
- an extension cannot silently escalate from database/network access to shell/admin access
- provider API versions are independently versioned from extension package versions

Runtime loading/sandboxing and signature trust roots remain future implementation work.
