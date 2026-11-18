# Architecture

EBD is split into a stable contract, core orchestration, client adapters, and user-facing applications.

```text
Desktop UI (egui) / CLI
      |
   EBD Core ---- Plugin Host (sandboxed WASM)
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
- `ebd-plugin` defines manifest validation and non-escalating permission grants.
- `ebd-runtime` executes WASM guests in a wasmi sandbox with allow-listed host imports and hard resource limits.
- `ebd-theme` validates theme data and produces CSS previews.
- `ebd-registry` verifies package hashes and publisher signatures and manages local installs.
- `ebd-cli` is the only user-facing binary today; a production UI is a separate future component.

## Configuration and logging

Process configuration is a single JSON document, loaded before any command runs. It is deny-by-default: unknown fields are rejected, so a typo cannot silently fall back to a permissive default. A missing file means defaults; an invalid file is a hard error.

Logging uses `tracing` with a subscriber built from the config. Records go to stderr so command output on stdout stays scriptable. `format = "json"` emits one structured record per line; `format = "text"` is for humans. The subscriber is installed with `try_init`, so an embedding host or test harness can supply its own.

Config is process-level only. Plugin permissions, account-scoped data, and UI state layer on top later and must never store credentials or session tokens.

## Plugin sandbox

A manifest declares requested permissions; a grant is the user-approved subset. The runtime checks a module's imports before instantiation and refuses any host function that is not in the grant. Only `env.ebd_log`, `env.ebd_send_message`, and `env.ebd_open_url` are known today; all other imports are rejected. Each invocation gets a fresh store with a fuel budget, capped memory/tables/instances, and a trap on failed memory growth. Guests have no filesystem, network, clock, or WASI access.

The message and URL host functions currently return a placeholder failure code. Wiring them to `AdapterManager` through `adapter_for` is the next host task; the gate must still apply, so a grant never bypasses a capability the adapter did not declare.

## Themes and packages

Themes are inert data: fields are restricted, colours are hex-only, and free-form text is screened before it becomes CSS. Packages carry a manifest of file hashes plus an Ed25519 signature over the exact manifest bytes. Install is allowed only when the publisher key is in the local trust store. Hashes detect corruption; signatures establish provenance. Neither mechanism executes package content.
