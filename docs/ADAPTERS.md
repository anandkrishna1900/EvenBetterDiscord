# Writing a client adapter

An adapter is the only code that talks to a chat client. Core, plugins and the UI see it through `ebd_contract::ClientAdapter` and never bypass it.

## Steps
1. Implement `ClientAdapter` (see `crates/ebd-contract`). `capabilities()` is the truth: declare only what you really implement.
2. For anything you do **not** declare, return `AdapterError::Unsupported(<that capability>)`. Do not fake success.
3. Return `AdapterError::Operation(msg)` for real failures. Never put secrets (tokens) in messages or `Debug` output.
4. Prove it with the shared check:

```rust
#[test]
fn conforms() {
    assert!(ebd_contract::conformance::check_gating(&MyAdapter::new()).is_empty());
}
```

`check_gating` is side-effect free: it only calls methods you did not declare.

## Reference adapters
- `ebd-mock-client` — offline, deterministic, used by tests and the UI.
- `ebd-discord-bot` — Discord's documented **bot** REST API. Needs a bot token (`EBD_DISCORD_TOKEN`) and, to read message text, the MESSAGE_CONTENT intent enabled in the developer portal. It does not modify or inject into the Discord app. Try it: `EBD_DISCORD_TOKEN=... ebd-cli bot guilds`.

## Rules
- Validate ids and inputs from callers before building requests (see `snowflake` in `ebd-discord-bot`).
- Timeouts on every network call.
- No undocumented endpoints, no client injection.
