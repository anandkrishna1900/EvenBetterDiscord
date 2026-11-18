# Examples

These files are data for the CLI commands. Nothing here is executed automatically.

- `plugin/` — a plugin manifest plus its WebAssembly Text source. Compile `hello.wat` to `hello.wasm` (for example with `wat2wasm`) before running `ebd-cli plugin run examples/plugin/plugin.json`.
- `theme/` — a theme manifest you can check with `ebd-cli theme validate examples/theme/midnight.json`.

Plugins can only call host functions named in their manifest and approved by the grant. The sandbox caps fuel, memory, tables, and instances; see `docs/ARCHITECTURE.md`.
