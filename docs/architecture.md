# VSN Architecture v0.2

## Product boundary

VSN is a local-first development platform whose machine execution boundary is `vsn-agent`. Desktop, CLI, Web and future mobile clients are controllers; they do not directly own runtime/database/process privileges.

```text
Client (CLI/Desktop/Web)
          |
          v
   authenticated protocol
          |
          v
       VSN Agent
          |
   policy / audit boundary
          |
   +------+------+------+------+
   |      |      |      |      |
Runtime Database Service Project Network
Providers Providers Providers Engine Providers
```

## Current implementation

P0/P1/P2 currently contain:

- provider/permission contracts
- shared types and Core crate
- Ed25519 Agent identity
- secure-store abstraction
- authenticated local IPC protocol
- signed audit chain
- Windows Service runtime host
- Windows Service/interactive CLI shared IPC credential provisioning

P3 will add the first mutating machine capabilities: process discovery, ports, service lifecycle, health checks and log streaming. No P3 mutation should be exposed before its permission check is implemented.

## Execution principle

The Core does not define a finite list of technologies. It defines extension categories:

- RuntimeProvider
- DatabaseProvider
- ServiceProvider
- ProjectProvider
- ContainerProvider
- CloudProvider
- OsProvider
- NetworkProvider

This is the mechanism that allows future languages/databases/services to be added without rewriting the Core.

## Database principle

VSN may generate a Database Studio interface from deterministic provider capabilities and introspection metadata. It must not guess an unknown database protocol. A real driver/API/CLI adapter must exist first.

## Remote-ready boundary

The future remote architecture keeps the Agent behind an outbound connection:

```text
Browser/Desktop
      |
Control Plane
      |
Secure Gateway/Relay
      |
 outbound Agent session
      |
Managed PC/VPS
```

Direct public database, terminal and development-server ports are not required by the design.
