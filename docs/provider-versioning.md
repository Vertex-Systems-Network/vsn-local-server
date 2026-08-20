# Provider Versioning Decision v1

VSN will not expose a stable Rust/C binary ABI to third-party providers in v1.

Reason: a native in-process ABI would couple providers to compiler/runtime details and would make an "unlimited extension" architecture brittle.

## Contract

Provider compatibility is defined by versioned data/protocol contracts:

- manifest schema version
- provider kind
- declared capabilities
- declared permissions
- protocol version

In-tree providers may initially be compiled with VSN for development speed. External providers must ultimately run behind a brokered out-of-process protocol so the Agent can enforce permissions and crash isolation.

Unknown databases may contribute a deterministic driver/provider that exposes introspection capabilities. VSN may generate UI from those capabilities, but must not invent or guess a database wire protocol.
