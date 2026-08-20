# VSN Database Driver SDK — P7 baseline

`vsn-database::DatabaseProvider` is the deterministic connection boundary. VSN never guesses an unknown database wire protocol.

A provider declares:
- data model: relational/document/key-value/graph/search/time-series/column/vector/custom
- capability set
- connection implementation
- namespaces/entities
- entity metadata
- query implementation

`EntityMeta` is normalized into `EntityUiSchema`. This permits Database Studio to generate controls for unknown-but-described engines without database-specific frontend code.

Field mapping baseline:
- boolean -> toggle
- integer/decimal -> numeric editor
- enum -> select
- JSON -> JSON editor
- relation -> relation selector
- geo -> map
- vector -> vector viewer
- unknown -> raw editor

The provider must explicitly report unsupported capabilities. The UI must not invent them.
