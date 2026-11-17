# Roadmap and integration boundary

## Independent work
- [x] Rust workspace and adapter contract
- [x] Contract coverage for every declared capability
- [x] Mock client and offline CLI demonstration
- [x] Adapter capability reporting and capability-gated dispatch
- [x] Workspace tests, lint, and CI on Linux, Windows, and macOS
- [ ] Persistent configuration and structured logging
- [ ] Plugin manifest validation and permission grants
- [ ] Sandboxed WASM runtime with resource limits
- [ ] Plugin SDK and example plugins
- [ ] Theme schema, validation, editor, and preview
- [ ] Local marketplace package install/update/uninstall
- [ ] Publisher signatures and trust management
- [ ] Native desktop UI (evaluate upstream GPUI)
- [ ] Update channel and release packaging

## Discord-dependent work
Blocked until Discord publishes a client build and a documented, authorised integration mechanism. At that point inspect public interfaces, build a separate adapter, and test against the released client. No undocumented injection or compatibility claims.
