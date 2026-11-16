# Architecture

EBD is split into a stable contract, core orchestration, client adapters, and user-facing applications.

```text
Desktop UI / CLI
      |
   EBD Core ---- Plugin Host (planned)
      |
 EBD Contract
      |
 Client Adapter
      |
 Mock Client today / authorised client adapter in future
```

The core must not import Discord-specific crates or depend on private client implementation details. Adapters advertise capabilities and return typed errors when an operation is unsupported.

The current CLI and mock adapter demonstrate the boundary. A production UI and plugin host are separate future components.
