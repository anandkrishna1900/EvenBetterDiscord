# EvenBetterDiscord

An independent, Rust-first customization platform designed around a replaceable client adapter. EBD can be developed and tested without Discord. Discord integration is intentionally disabled until a public client and a documented, authorised extension interface are available.

## Project status

This repository is the canonical development home. The current source is a foundation, not a claim of Discord compatibility.

- Rust workspace with a stable client contract
- Mock client for offline development
- Core adapter manager, capability reporting, and capability gating
- CLI demo, capability listing, self-check, and config commands
- Persistent JSON configuration and structured logging (text or JSON)
- Security and integration-boundary documentation
- Formatting, lint, and test CI on Linux, Windows, and macOS

Not yet implemented: production WASM sandbox execution, a graphical GPUI application, publisher identity/signature trust, hosted marketplace services, auto-updates, and Discord integration.

## Requirements

- Rust stable (see `.rust-toolchain.toml`)
- Cargo

On Windows the default `stable` toolchain expects the MSVC build tools. Without them, use the self-contained GNU toolchain: `rustup toolchain install stable-x86_64-pc-windows-gnu` and run cargo with `+stable-x86_64-pc-windows-gnu`.

## Build and test

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Usage

```sh
cargo run -p ebd-cli                  # demo (same as `demo`)
cargo run -p ebd-cli -- capabilities  # list every contract capability and whether the adapter declares it
cargo run -p ebd-cli -- self-check    # fail unless the adapter declares every capability
cargo run -p ebd-cli -- config        # show the effective configuration and its path
cargo run -p ebd-cli -- config --init # write the defaults (refuses to overwrite)
```

`demo` walks the mock adapter through the whole contract: current user, guilds, channels, recent messages, a sent message, and a capability-gated dispatch path. Every operation goes through `AdapterManager::adapter_for`, so an adapter that never declared a capability cannot be used as if it had.

## Configuration and logging

Configuration lives in `config.json` under the platform configuration root:

- Windows: `%APPDATA%\ebd\config.json`
- macOS/Linux: `$XDG_CONFIG_HOME/ebd/config.json`, falling back to `~/.config/ebd/config.json`

Set `EBD_CONFIG` to use a different path. A missing file means "use defaults", so a fresh checkout runs without setup. Invalid files and unknown keys are errors rather than being silently ignored.

```json
{
  "logging": {
    "level": "info",
    "format": "text"
  }
}
```

`level` is one of `error`, `warn`, `info`, `debug`, `trace`. `format` is `text` or `json`; JSON records go to stderr and suit log shipping. Configuration never stores credentials or client session data.

## Workspace

- `crates/ebd-contract`: shared types, capabilities, and the `ClientAdapter` trait
- `crates/ebd-core`: adapter registry, compatibility report, capability gate
- `crates/ebd-mock-client`: deterministic offline adapter
- `crates/ebd-config`: persistent configuration (`config.json`) and log settings
- `crates/ebd-cli`: runnable demonstration, self-check, and config commands

## Writing an adapter

Implement `ClientAdapter` for your client and attach it:

```rust
let mut manager = AdapterManager::new();
manager.attach(Box::new(MyAdapter::new()));
```

Adapters declare their capabilities in `capabilities()` and must return `AdapterError::Unsupported` for anything they do not declare. Core never depends on a specific client.

## Design principles

1. EBD Core depends on a stable contract, never on Discord internals.
2. Adapters must explicitly declare capabilities, and callers must gate on them.
3. Plugins must receive explicit permissions and run sandboxed; do not execute untrusted native code.
4. Marketplace hashes detect corruption but do not authenticate publishers.
5. No undocumented injection, credential extraction, or bypass of client security controls.

See [ARCHITECTURE.md](docs/ARCHITECTURE.md), [SECURITY.md](docs/SECURITY.md), [ROADMAP.md](docs/ROADMAP.md), and [PLUGIN_PLAN.md](docs/PLUGIN_PLAN.md).
