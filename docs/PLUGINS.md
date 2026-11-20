# Writing a plugin

A plugin is a WebAssembly module with a `plugin.json` manifest (see `examples/plugin`). It runs in a sandbox: no filesystem, network or clock; capped fuel and memory. It only gets the host functions its manifest requests **and** the user grants.

## Entry point
Export `run() -> i32` and (if you use host functions) `memory`. Return `0` for success; anything else is reported as failure.

## Host functions (module `env`)
All pointers are offsets into your exported `memory`. Strings are UTF-8.

| Function | Permission | Returns |
|---|---|---|
| `ebd_log(ptr, len)` | `log` | — |
| `ebd_current_user(out_ptr, out_cap) -> i32` | `read_current_user` | bytes of the username written (truncated to `out_cap`), or `-1` if unavailable |
| `ebd_send_message(chan_ptr, chan_len, text_ptr, text_len) -> i32` | `send_messages` | `0` sent, `1` no client / unsupported, `2` client rejected it |
| `ebd_open_url(ptr, len) -> i32` | `open_external_url` | `0` opened, `1` no client / unsupported, `2` rejected |

Importing a function you were not granted fails before your code runs. Client calls go through the same capability gate as everything else, so an adapter that does not support a capability yields status `1`.

## Example: say hello to a channel
```wat
(module
  (import "env" "ebd_send_message" (func $send (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0)  "mock-general")
  (data (i32.const 64) "hello from a plugin")
  (func (export "run") (result i32)
    i32.const 0 i32.const 12 i32.const 64 i32.const 19 call $send))
```
Manifest: `"requested": ["send_messages"]`. Compile with `wat2wasm`, then `ebd-cli plugin run plugin.json` (runs against the offline mock client).

## Publishing
```sh
ebd-cli key gen publisher.key                      # prints your public key
ebd-cli package pack <dir> my.plugin 1.0.0 plugin me publisher.key
ebd-cli key trust me <public-key>                  # on the installing machine
ebd-cli package install <dir>
```
`pack` hashes every file in `<dir>` and writes `package.json` + `signature.sig`. Keep `publisher.key` secret.
