# Roadmap and integration boundary

## Independent work
- [x] Rust workspace and adapter contract
- [x] Contract coverage for every declared capability
- [x] Mock client and offline CLI demonstration
- [x] Adapter capability reporting and capability-gated dispatch
- [x] Workspace tests, lint, and CI on Linux, Windows, and macOS
- [x] Persistent configuration and structured logging
- [x] Plugin manifest validation and permission grants
- [x] Sandboxed WASM runtime with resource limits
- [ ] Plugin SDK and example plugins
- [x] Theme schema, validation, and CSS preview (editor still pending)
- [x] Local marketplace package install/update/uninstall
- [x] Publisher signatures and trust management (publisher signing CLI still pending)
- [ ] Native desktop UI (evaluate upstream GPUI)
- [ ] Update channel and release packaging

See [HANDOVER.md](../HANDOVER.md) for scope, limitations, and the recommended next steps.

## Discord-dependent work
Blocked until Discord publishes a client build and a documented, authorised integration mechanism. At that point inspect public interfaces, build a separate adapter, and test against the released client. No undocumented injection or compatibility claims.
