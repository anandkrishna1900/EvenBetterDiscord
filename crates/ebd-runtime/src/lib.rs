//! Sandboxed WASM plugin execution.
//!
//! Guests get no filesystem, network, clock, or native code. Only host
//! functions named in an explicit [`PluginGrant`] are linked, CPU time is
//! capped with fuel, and memory/table/instance counts are capped by a store
//! resource limiter. Modules are validated before instantiation.

use ebd_contract::Capability;
use ebd_plugin::{PluginGrant, PluginPermission};
use wasmi::{Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, Val};

/// Hard limits applied to every plugin invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxLimits {
    /// Maximum number of executed WASM instructions (approximate).
    pub fuel: u64,
    /// Maximum linear memory per module, in 64 KiB pages.
    pub max_memory_pages: u32,
    /// Maximum number of linear memories per module.
    pub max_memories: usize,
    /// Maximum number of tables per module.
    pub max_tables: usize,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            fuel: 1_000_000,
            max_memory_pages: 16,
            max_memories: 1,
            max_tables: 1,
        }
    }
}

/// Result of a successful plugin invocation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Execution {
    /// Lines the guest emitted through the granted `log` host function.
    pub log: Vec<String>,
}

/// Errors from validating or running a plugin.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("invalid plugin module: {0}")]
    Module(String),
    #[error("plugin requires host function `{0}` that was not granted")]
    Denied(String),
    #[error("plugin requires unknown host function `{0}`")]
    UnknownImport(String),
    #[error("plugin exceeded its fuel budget")]
    FuelExhausted,
    #[error("plugin exceeded its resource limits: {0}")]
    Limit(String),
    #[error("plugin trapped: {0}")]
    Trap(String),
    #[error("plugin does not export a `run` function")]
    MissingEntry,
    #[error("plugin returned a non-zero status: {0}")]
    Failed(i32),
}

#[derive(Debug)]
struct HostState {
    log: Vec<String>,
    limits: StoreLimits,
}

/// The sandbox. Cheap to construct; each `run` gets a fresh store.
#[derive(Debug, Clone, Copy, Default)]
pub struct Runtime {
    limits: SandboxLimits,
}

impl Runtime {
    pub fn new(limits: SandboxLimits) -> Self {
        Self { limits }
    }

    /// Runs the `run() -> i32` export of `wasm` with only `grant` available.
    pub fn run(&self, wasm: &[u8], grant: &PluginGrant) -> Result<Execution, RuntimeError> {
        let mut config = wasmi::Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module =
            Module::new(&engine, wasm).map_err(|error| RuntimeError::Module(error.to_string()))?;

        check_imports(&module, grant)?;

        let mut store = Store::new(
            &engine,
            HostState {
                log: Vec::new(),
                limits: self.resource_limits(),
            },
        );
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(self.limits.fuel)
            .map_err(|error| RuntimeError::Module(error.to_string()))?;

        let mut linker = Linker::new(&engine);
        link_host_functions(&mut linker, grant)?;

        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(map_wasmi_error)?;

        let run = instance
            .get_export(&store, "run")
            .and_then(Extern::into_func)
            .ok_or(RuntimeError::MissingEntry)?;

        let mut results = [Val::I32(0)];
        run.call(&mut store, &[], &mut results)
            .map_err(map_wasmi_error)?;

        let status = match results[0] {
            Val::I32(value) => value,
            _ => return Err(RuntimeError::Module("`run` must return i32".into())),
        };
        if status != 0 {
            return Err(RuntimeError::Failed(status));
        }

        Ok(Execution {
            log: store.into_data().log,
        })
    }

    fn resource_limits(&self) -> StoreLimits {
        StoreLimitsBuilder::new()
            .memory_size(self.limits.max_memory_pages as usize * 65_536)
            .memories(self.limits.max_memories)
            .tables(self.limits.max_tables)
            .instances(1)
            .trap_on_grow_failure(true)
            .build()
    }
}

/// Rejects anything the guest asks for that is not on the allow-list.
fn check_imports(module: &Module, grant: &PluginGrant) -> Result<(), RuntimeError> {
    for import in module.imports() {
        if import.module() != "env" {
            return Err(RuntimeError::UnknownImport(format!(
                "{}.{}",
                import.module(),
                import.name()
            )));
        }

        let permission = match import.name() {
            "ebd_log" => PluginPermission::Log,
            "ebd_send_message" => PluginPermission::SendMessages,
            "ebd_open_url" => PluginPermission::OpenExternalUrl,
            other => return Err(RuntimeError::UnknownImport(other.to_owned())),
        };

        if !grant.allows(permission) {
            return Err(RuntimeError::Denied(permission.as_str().to_owned()));
        }
    }
    Ok(())
}

fn link_host_functions(
    linker: &mut Linker<HostState>,
    grant: &PluginGrant,
) -> Result<(), RuntimeError> {
    if grant.allows(PluginPermission::Log) {
        linker
            .func_wrap(
                "env",
                "ebd_log",
                |mut caller: wasmi::Caller<'_, HostState>, pointer: i32, length: i32| {
                    let bytes = read_memory(&mut caller, pointer, length)?;
                    let message = String::from_utf8_lossy(&bytes).into_owned();
                    caller.data_mut().log.push(message);
                    Ok::<(), wasmi::Error>(())
                },
            )
            .map_err(|error| RuntimeError::Module(error.to_string()))?;
    }
    if grant.allows(PluginPermission::SendMessages) {
        // Return 1 ("no adapter available") until a real client is attached.
        linker
            .func_wrap(
                "env",
                "ebd_send_message",
                |_caller: wasmi::Caller<'_, HostState>, _pointer: i32, _length: i32| {
                    Ok::<i32, wasmi::Error>(1)
                },
            )
            .map_err(|error| RuntimeError::Module(error.to_string()))?;
    }
    if grant.allows(PluginPermission::OpenExternalUrl) {
        linker
            .func_wrap(
                "env",
                "ebd_open_url",
                |_caller: wasmi::Caller<'_, HostState>, _pointer: i32, _length: i32| {
                    Ok::<i32, wasmi::Error>(1)
                },
            )
            .map_err(|error| RuntimeError::Module(error.to_string()))?;
    }
    Ok(())
}

fn read_memory(
    caller: &mut wasmi::Caller<'_, HostState>,
    pointer: i32,
    length: i32,
) -> Result<Vec<u8>, wasmi::Error> {
    let pointer =
        usize::try_from(pointer).map_err(|_| wasmi::Error::new("negative memory pointer"))?;
    let length =
        usize::try_from(length).map_err(|_| wasmi::Error::new("negative memory length"))?;
    let memory = caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or_else(|| wasmi::Error::new("guest has no exported `memory`"))?;
    let mut buffer = vec![0u8; length];
    memory
        .read(&*caller, pointer, &mut buffer)
        .map_err(|error| wasmi::Error::new(error.to_string()))?;
    Ok(buffer)
}

fn map_wasmi_error(error: wasmi::Error) -> RuntimeError {
    if error.as_trap_code() == Some(wasmi::TrapCode::OutOfFuel) {
        RuntimeError::FuelExhausted
    } else {
        let message = error.to_string();
        if message.contains("resource limit")
            || message.contains("memory")
            || message.contains("table")
        {
            RuntimeError::Limit(message)
        } else {
            RuntimeError::Trap(message)
        }
    }
}

/// Convenience helper for callers that have a capability to check first.
pub fn require_capability(grant: &PluginGrant, capability: Capability) -> Result<(), RuntimeError> {
    if grant.allows_capability(capability) {
        Ok(())
    } else {
        Err(RuntimeError::Denied(
            ebd_plugin::PluginPermission::for_capability(capability)
                .as_str()
                .to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(permissions: &[PluginPermission]) -> PluginGrant {
        PluginGrant::new("test.plugin", permissions.iter().copied())
    }

    #[test]
    fn runs_a_pure_guest_and_returns_its_status() {
        let wasm = wat::parse_str(r#"(module (func (export "run") (result i32) i32.const 0))"#)
            .expect("wat");

        let result = Runtime::default().run(&wasm, &grant(&[])).expect("run");
        assert!(result.log.is_empty());
    }

    #[test]
    fn non_zero_status_is_a_failure() {
        let wasm = wat::parse_str(r#"(module (func (export "run") (result i32) i32.const 7))"#)
            .expect("wat");

        assert!(matches!(
            Runtime::default().run(&wasm, &grant(&[])),
            Err(RuntimeError::Failed(7))
        ));
    }

    #[test]
    fn missing_entry_point_is_rejected() {
        let wasm = wat::parse_str("(module)").expect("wat");

        assert!(matches!(
            Runtime::default().run(&wasm, &grant(&[])),
            Err(RuntimeError::MissingEntry)
        ));
    }

    #[test]
    fn ungranted_host_function_is_rejected_before_execution() {
        let wasm = wat::parse_str(
            r#"(module
                (import "env" "ebd_log" (func $log (param i32 i32)))
                (func (export "run") (result i32) i32.const 0 i32.const 0 call $log i32.const 0))"#,
        )
        .expect("wat");

        assert!(matches!(
            Runtime::default().run(&wasm, &grant(&[])),
            Err(RuntimeError::Denied(name)) if name == "log"
        ));
    }

    #[test]
    fn granted_log_function_collects_guest_output() {
        let wasm = wat::parse_str(
            r#"(module
                (import "env" "ebd_log" (func $log (param i32 i32)))
                (memory (export "memory") 1)
                (data (i32.const 0) "hello from wasm")
                (func (export "run") (result i32)
                    i32.const 0 i32.const 15 call $log
                    i32.const 0))"#,
        )
        .expect("wat");

        let execution = Runtime::default()
            .run(&wasm, &grant(&[PluginPermission::Log]))
            .expect("run");
        assert_eq!(execution.log, vec!["hello from wasm".to_owned()]);
    }

    #[test]
    fn unknown_imports_are_rejected() {
        let wasm = wat::parse_str(
            r#"(module
                (import "env" "wasi_snapshot_preview1" (func $f))
                (func (export "run") (result i32) i32.const 0))"#,
        )
        .expect("wat");

        assert!(matches!(
            Runtime::default().run(&wasm, &grant(&[PluginPermission::Log])),
            Err(RuntimeError::UnknownImport(_))
        ));
    }

    #[test]
    fn fuel_budget_stops_infinite_loops() {
        let wasm = wat::parse_str(
            r#"(module
                (func (export "run") (result i32)
                    (loop br 0)
                    i32.const 0))"#,
        )
        .expect("wat");

        let runtime = Runtime::new(SandboxLimits {
            fuel: 10_000,
            ..SandboxLimits::default()
        });
        assert!(matches!(
            runtime.run(&wasm, &grant(&[])),
            Err(RuntimeError::FuelExhausted)
        ));
    }

    #[test]
    fn memory_growth_is_capped() {
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1 1000)
                (func (export "run") (result i32)
                    i32.const 100 memory.grow drop
                    i32.const 0))"#,
        )
        .expect("wat");

        let runtime = Runtime::new(SandboxLimits {
            max_memory_pages: 2,
            ..SandboxLimits::default()
        });
        let result = runtime.run(&wasm, &grant(&[]));
        assert!(
            matches!(
                &result,
                Err(RuntimeError::Limit(_)) | Err(RuntimeError::Trap(_))
            ),
            "unexpected result: {result:?}"
        );
    }

    #[test]
    fn capability_gate_helper_matches_the_grant() {
        let grant = grant(&[PluginPermission::ReadCurrentUser]);
        assert!(require_capability(&grant, Capability::ReadCurrentUser).is_ok());
        assert!(require_capability(&grant, Capability::SendMessages).is_err());
    }
}
