# EvenBetterDiscord

An independent, Rust-first customization platform designed around a replaceable client adapter. EBD can be developed and tested without Discord. Discord integration is intentionally disabled until a public client and a documented, authorised extension interface are available.

## Project status

The current source is a working foundation, not a claim of Discord compatibility. What is implemented today:

- Rust workspace with a stable client contract and a capability-gated adapter manager
- Deterministic offline mock client and a CLI with demo, self-check, and diagnostics
- Persistent JSON configuration and structured logging (text or JSON)
- Plugin manifests, explicit permission grants, and a sandboxed WASM runtime with fuel and memory limits
- Theme schema, validation, and CSS preview generation
- Local marketplace packages with SHA-256 integrity checks and Ed25519 publisher signatures
- Formatting, lint, MSRV, and test CI on Linux, Windows, and macOS

Not yet implemented: a graphical GPUI application, a hosted marketplace, update/release packaging, package signing tooling for publishers, and Discord integration. See [HANDOVER.md](HANDOVER.md) for the full state and next steps.

## Requirements

- Rust stable (see `.rust-toolchain.toml`)
- Minimum supported Rust version (MSRV): **1.88**, enforced by a CI job and `.cargo/config.toml` resolver fallback
- Cargo

On Windows the default `stable` toolchain expects the MSVC build tools. Without them, install a GNU toolchain and a MinGW linker, then run cargo with `+stable-x86_64-pc-windows-gnu`:

```sh
rustup toolchain install stable-x86_64-pc-windows-gnu -c rustfmt -c clippy
winget install --id BrechtSanders.WinLibs.POSIX.UCRT
```

## Build and test

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Usage

On Windows with the GNU toolchain setup described above, run any command through the helper from the repository root:

```cmd
run.cmd                       adapter demo (default)
run.cmd capabilities          list contract capabilities and adapter support
run.cmd self-check            fail unless every capability is declared
run.cmd plugin demo           run an embedded guest in the WASM sandbox
run.cmd theme validate examples/theme/midnight.json
```

The equivalent Cargo invocations (on any platform) are:

```sh
cargo run -p ebd-cli                            # adapter demo (default command)
cargo run -p ebd-cli -- capabilities            # list contract capabilities and adapter support
cargo run -p ebd-cli -- self-check              # fail unless every capability is declared
cargo run -p ebd-cli -- config                  # show effective configuration and its path
cargo run -p ebd-cli -- config --init           # write defaults (refuses to overwrite)
cargo run -p ebd-cli -- plugin demo             # run an embedded WASM guest in the sandbox
cargo run -p ebd-cli -- plugin validate examples/plugin/plugin.json
cargo run -p ebd-cli -- theme validate examples/theme/midnight.json
cargo run -p ebd-cli -- key list
cargo run -p ebd-cli -- package list
```

Full command list is in `--help`. Every adapter operation goes through `AdapterManager::adapter_for`, so an adapter that never declared a capability cannot be used as if it had.

## Configuration and logging

Configuration lives in `config.json` under the platform configuration root:

- Windows: `%APPDATA%\ebd\config.json`
- macOS/Linux: `$XDG_CONFIG_HOME/ebd/config.json`, falling back to `~/.config/ebd/config.json`

Set `EBD_CONFIG` to use a different path. A missing file means "use defaults", so a fresh checkout runs without setup. Invalid files and unknown keys are errors rather than being silently ignored. The same directory holds `trust.json` (publisher keys) and `packages/` (installed packages).

```json
{
  "logging": {
    "level": "info",
    "format": "text"
  }
}
```

`level` is one of `error`, `warn`, `info`, `debug`, `trace`. `format` is `text` or `json`; JSON records go to stderr and suit log shipping. Configuration never stores credentials or client session data.

## Plugins and the sandbox

A plugin declares its requested permissions in `plugin.json`. The user-approved grant can only be a subset of those permissions. The host links only the host functions present in that grant and rejects unknown imports before execution. CPU is capped with fuel, and linear memory, tables, instances, and memory growth are capped per invocation. See [examples/plugin](examples/plugin) for a manifest and WAT source.

```json
{
  "manifest_version": 1,
  "id": "examples.hello",
  "name": "Hello Plugin",
  "version": "1.0.0",
  "entry": "hello.wasm",
  "requested": ["log"]
}
```

The sandbox has no filesystem, network, clock, or native-code access. Native plugin libraries are never loaded into the host process.

## Themes and packages

Themes are data-only JSON documents. `ebd-cli theme validate` checks colours, text fields, and schema version, and prints the CSS custom properties a UI would apply.

Packages are directories containing `package.json` (file hashes), the content files, and `signature.sig` (a hex Ed25519 signature over the manifest bytes). Verification checks the hashes, checks the publisher key against `trust.json`, and only then allows installation. Hashes detect corruption; signatures establish which trusted publisher produced the package.

## Workspace

- `crates/ebd-contract`: shared types, capabilities, and the `ClientAdapter` trait
- `crates/ebd-core`: adapter registry, compatibility report, capability gate
- `crates/ebd-mock-client`: deterministic offline adapter
- `crates/ebd-config`: persistent configuration and log settings
- `crates/ebd-plugin`: plugin manifest schema, validation, and permission grants
- `crates/ebd-runtime`: sandboxed WASM execution (wasmi) with resource limits
- `crates/ebd-theme`: theme schema, validation, and preview generation
- `crates/ebd-registry`: local packages, integrity hashes, signatures, and trust store
- `crates/ebd-cli`: runnable diagnostics for all of the above

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
3. Plugins receive explicit permissions and run sandboxed; untrusted native code is never executed.
4. Package hashes detect corruption; signatures bind a package to a publisher in an explicit trust store.
5. No undocumented injection, credential extraction, or bypass of client security controls.

See [HANDOVER.md](HANDOVER.md), [ARCHITECTURE.md](docs/ARCHITECTURE.md), [SECURITY.md](docs/SECURITY.md), [ROADMAP.md](docs/ROADMAP.md), and [PLUGIN_PLAN.md](docs/PLUGIN_PLAN.md).
