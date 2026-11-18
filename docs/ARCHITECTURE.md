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

## Capability gate

Every capability in the contract has exactly one trait method, and every trait method names the capability it needs. Callers do not call an adapter directly; they ask the manager for an adapter that declares the capability:

```rust
let adapter = manager.adapter_for(Capability::SendMessages)?;
adapter.send_message(channel_id, content)?;
```

`adapter_for` fails with `AdapterError::Operation` when nothing is attached and `AdapterError::Unsupported` when the capability was not declared. Capability declarations therefore drive behaviour instead of being decorative.

## Current shape

- `ebd-contract` defines `Capability`, the data types, `AdapterError`, and `ClientAdapter`.
- `ebd-core` owns `AdapterManager` (attach, detach, report, gate) and `CompatibilityReport` (contract version, declared capabilities, missing capabilities).
- `ebd-mock-client` implements every capability deterministically and offline.
- `ebd-config` owns the persisted `Config` document and log settings, and resolves the platform config path.
- `ebd-cli` is the only user-facing binary today; a production UI and plugin host are separate future components.

## Configuration and logging

Process configuration is a single JSON document, loaded before any command runs. It is deny-by-default: unknown fields are rejected, so a typo cannot silently fall back to a permissive default. A missing file means defaults; an invalid file is a hard error.

Logging uses `tracing` with a subscriber built from the config. Records go to stderr so command output on stdout stays scriptable. `format = "json"` emits one structured record per line; `format = "text"` is for humans. The subscriber is installed with `try_init`, so an embedding host or test harness can supply its own.

Config is process-level only. Plugin permissions, adapter selection, and account-scoped data will layer on top later and must never store credentials or session tokens.
