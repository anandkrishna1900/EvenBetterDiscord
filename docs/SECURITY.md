# Security model

- Treat plugins, themes, and marketplace metadata as untrusted input.
- Do not load third-party native dynamic libraries into the host process.
- Planned plugin execution must use a maintained WASM runtime with fuel/time limits, memory limits, restricted imports, and explicit host capabilities.
- Permission grants must be user-visible, revocable, and deny-by-default.
- Package hashes detect accidental corruption only. They do not prove publisher identity.
- Publisher trust requires signature verification against a separately managed trust store; this is not implemented yet.
- Never store Discord credentials or session tokens in EBD.
- Do not bypass client authentication, anti-abuse, or security controls.
