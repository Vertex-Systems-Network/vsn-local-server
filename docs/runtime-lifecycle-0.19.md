# Runtime lifecycle hardening — 0.19

Runtime installation now extracts into a sibling staging directory, verifies the expected executable, preserves any existing version as a temporary backup, atomically swaps the staged directory into place, and restores the backup on replace failure.

Uninstall first renames the runtime directory to a tombstone, commits registry/project-activation mutation, and restores the runtime if registry persistence fails. Registry and shim writes use temporary files plus rename. `runtime repair` drops missing installs, repairs simple executable-path drift and removes invalid activation references.
