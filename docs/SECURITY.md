# Security model

- Treat plugins, themes, and marketplace metadata as untrusted input.
- Never load third-party native dynamic libraries into the host process.
- Plugins run in a maintained WASM interpreter (wasmi) with fuel, memory, table, and instance limits, restricted imports, and explicit host capabilities. There is no filesystem, network, clock, or WASI access.
- Permission grants must be user-visible, revocable, and deny-by-default. A grant can only narrow what a manifest requested.
- Capabilities are deny-by-default: an adapter may only be used for a capability it declared, and adapters are responsible for rejecting input they cannot safely handle (for example, `open_external_url` accepts only absolute `https` URLs with a valid host).
- Configuration is deny-by-default: unknown keys are rejected and an invalid config file stops startup rather than silently reverting to different settings.
- Package hashes detect corruption or tampering. They do not prove publisher identity.
- Publisher identity is an Ed25519 signature over the exact package manifest, verified against a separately managed local trust store. A publisher is trusted only after the user adds its key.
- The trust store and config are ordinary files protected by OS permissions only. There is no secret storage, key rotation, or revocation list yet.
- Never store Discord credentials or session tokens in EBD.
- Do not bypass client authentication, anti-abuse, or security controls.
