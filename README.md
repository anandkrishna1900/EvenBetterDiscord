# EvenBetterDiscord

An independent, Rust-first customization platform designed around a replaceable client adapter. EBD can be developed and tested without Discord. Discord integration is intentionally disabled until a public client and a documented, authorised extension interface are available.

## Project status

This repository is the canonical development home. The current source is a foundation, not a claim of Discord compatibility.

- Rust workspace with a stable client contract
- Mock client for offline development
- Core adapter manager and capability reporting
- CLI demo and self-check
- Security and integration-boundary documentation

Not yet implemented: production WASM sandbox execution, a graphical GPUI application, publisher identity/signature trust, hosted marketplace services, auto-updates, and Discord integration.

## Requirements

- Rust stable (see `.rust-toolchain.toml`)
- Cargo

## Build and test

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p ebd-cli
cargo run -p ebd-cli -- self-check
```

## Workspace

- `crates/ebd-contract`: shared types and adapter interface
- `crates/ebd-core`: adapter registry and compatibility reporting
- `crates/ebd-mock-client`: offline mock adapter
- `crates/ebd-cli`: runnable demonstration

## Design principles

1. EBD Core depends on a stable contract, never on Discord internals.
2. Adapters must explicitly declare capabilities.
3. Plugins must receive explicit permissions and run sandboxed; do not execute untrusted native code.
4. Marketplace hashes detect corruption but do not authenticate publishers.
5. No undocumented injection, credential extraction, or bypass of client security controls.

See [ARCHITECTURE.md](docs/ARCHITECTURE.md), [SECURITY.md](docs/SECURITY.md), and [ROADMAP.md](docs/ROADMAP.md).
