# Handover

Snapshot: 2026-11-18, `main`. This document is the state of the project for the next contributor, whether work continues before or after Discord ships an authorised client interface.

## What works today

| Area | Status | Entry points |
|---|---|---|
| Client contract | Stable `ClientAdapter`, capabilities, typed errors | `crates/ebd-contract` |
| Adapter core | Attach/detach, compatibility report, capability-gated dispatch | `crates/ebd-core` |
| Offline client | Deterministic mock covering every capability | `crates/ebd-mock-client` |
| Config + logging | JSON config at platform paths, `EBD_CONFIG` override, text/JSON `tracing` output | `crates/ebd-config`, `ebd-cli config` |
| Plugins | Manifest schema, validation, non-escalating permission grants | `crates/ebd-plugin`, `ebd-cli plugin validate` |
| Sandbox | wasmi interpreter, allow-listed host imports, fuel/memory/table/instance limits | `crates/ebd-runtime`, `ebd-cli plugin demo` |
| Themes | JSON schema, colour/text validation, CSS preview | `crates/ebd-theme`, `ebd-cli theme validate` |
| Packages | SHA-256 hashes, Ed25519 signatures, trust store, install/list/update/uninstall | `crates/ebd-registry`, `ebd-cli package` / `key` |
| CI | fmt, clippy `-D warnings`, tests, MSRV job, Linux/Windows/macOS matrix | `.github/workflows/ci.yml` |

All 72 tests pass locally; `fmt` and `clippy -D warnings` are clean. The MSRV is 1.88.

## Decisions worth knowing

- **MSRV raised from 1.75 to 1.88.** Current WASM and crypto crates (and their transitive deps) require edition 2024 / newer Cargo. `.cargo/config.toml` sets `incompatible-rust-versions = "fallback"` so dependency resolution respects `rust-version`. The CI MSRV job regenerates the lockfile with 1.88.
- **JSON config instead of TOML.** The maintained `toml` releases pulled `indexmap` with edition 2024, which broke the old MSRV. JSON uses `serde_json`, which was already a workspace dependency.
- **Security boundary.** The runtime never loads native libraries. It validates imports before instantiation, links only granted host functions, caps fuel (default 1,000,000) and memory (default 16 pages / 1 MiB), and enables `trap_on_grow_failure`. There is no filesystem, network, clock, or WASI access.
- **Permissions are deny-by-default.** A grant cannot add a permission the manifest did not request; `PluginGrant::checked` enforces this. Capability-to-permission mapping is total over `Capability::ALL`.
- **Trust is explicit.** Package hashes alone only detect corruption. A publisher key must be added to `trust.json` before a package can be installed, and its signature is verified over the exact bytes of `package.json`.
- **stdout stays scriptable.** User-facing command output goes to stdout; structured logs go to stderr.

## How to verify

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p ebd-cli -- self-check
cargo run -p ebd-cli -- plugin demo
```

On a Windows machine without MSVC Build Tools, the working local setup is the GNU toolchain plus a MinGW linker (see README). Use `run.cmd` from the repository root, or prefix cargo commands with `+stable-x86_64-pc-windows-gnu`.

## Known limitations

- `ebd_send_message` and `ebd_open_url` host functions currently return `1` ("no adapter available"). Wiring them to `AdapterManager` and a permission dialog is the next plugin-host task.
- Fuel is a CPU proxy, not a wall-clock deadline. A cooperative guest cannot block forever, but an extremely expensive single instruction sequence is bounded only by fuel accounting.
- The WASM guest protocol is minimal: `run() -> i32` plus exported linear memory. There is no SDK crate, no ABI version negotiation, and no compiled example `.wasm` (only WAT source and an embedded demo).
- There is no publisher-facing signing CLI. `ebd_registry::sign_manifest` exists for tests and integrators; a `cargo run -p ebd-cli -- key generate` / `package sign` flow is still needed, including secure key generation and storage.
- Trust store and config are plain JSON files with OS-level permissions only. No secret storage, no rotation, no revocation list.
- The registry is local-only: no remote index, no update feed, no release packaging, no rollback beyond version-directory removal.
- No graphical UI. Theme preview generates CSS data only; there is no GPUI application or live preview.
- The plugin plan in `docs/PLUGIN_PLAN.md` is a 159-plugin wish list, not a commitment. The `[?]` entries are explicitly review-before-build.

## Next Discord-independent work, in suggested order

1. **Finish the plugin host loop.** Give `ebd-runtime` a real adapter-backed host: pass an `AdapterManager` reference into the store, implement `ebd_send_message` and `ebd_open_url` through `adapter_for` + `PluginGrant`, and surface denials to the guest. Add a permission prompt/approval store in `ebd-config`.
2. **Publisher tooling.** `key generate`, `key export`, `package sign`, and `package build` CLI commands; deterministic canonical manifest bytes; key file permissions; revocation/trust revocation.
3. **Plugin SDK + real example.** A small `no_std` guest crate with typed host bindings, an ABI version field in the manifest, and a checked-in compiled `.wasm` example so `plugin run` works without `wat2wasm`.
4. **Theme editor/preview UI.** Wire `ThemePreview` into a UI; validate at save time; support multiple themes with activation state.
5. **Update channel and release packaging.** Version manifests, platform artefacts, hashes/signatures for releases, rollback, and an offline-first update check.
6. **Hosted marketplace (optional).** Only after the local model is stable; keep the trust model user-controlled and never auto-trust publishers.
7. **Native UI.** Evaluate upstream GPUI; keep the CLI as the reference surface and reuse the same crates.

## When Discord publishes an authorised interface

1. Confirm the interface is public, documented, and permitted by Discord's terms. Do not reverse-engineer, inject into the client, extract credentials, or bypass security controls.
2. Add a separate adapter crate that implements `ebd_contract::ClientAdapter`, declares only the capabilities it can actually honour, and returns `AdapterError::Unsupported` for the rest.
3. Keep Discord-specific code out of `ebd-core`, `ebd-plugin`, `ebd-runtime`, `ebd-theme`, and `ebd-registry`. The contract is the only boundary.
4. Extend the contract version deliberately if new capabilities are needed; update `Capability::ALL`, `PluginPermission`, and the compatibility report together, and add tests for the new mapping.
5. Test against the released client with a real integration suite before making any compatibility claim in the README.

## Rules to preserve

- Core depends only on the stable contract, never on Discord internals.
- Adapters declare capabilities; callers gate on them.
- Plugins get explicit permissions and run sandboxed; never load untrusted native code.
- Hashes detect corruption; signatures plus an explicit trust store establish publisher identity.
- No credentials, session tokens, undocumented injection, or security-control bypass.
