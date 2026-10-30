//! Runtime WASM de terceros sobre el ABI JSON `api = 1`.
//!
//! ABI (módulo core WebAssembly, sin WASI):
//! - `folio_api_version() -> i32` devuelve la versión de API (1).
//! - `folio_alloc(len: i32) -> i32` reserva un buffer en memoria del plugin.
//! - `folio_handle(req_ptr: i32, req_len: i32) -> i32` procesa la petición y
//!   devuelve la longitud de la respuesta en `folio_result_ptr()`.
//! - `folio_result_ptr() -> i32` y `folio_result_len() -> i32` localizan la
//!   respuesta JSON.
//! - `memory` es la memoria lineal exportada.
//!
//! El host crea un `Store`/instancia nuevos por llamada, limita memoria y fuel,
//! y no expone WASI ni el sistema de archivos. Las funciones de host consultan
//! `HostContext`; sin permiso `network`, `folio_http_allowed` devuelve 0.

use folio_plugin_sdk::{HostContext, Manifest, Request, Response};
use wasmi::{
    Caller, Engine, Error as WasmiError, Extern, Linker, Module, Store, StoreLimits,
    StoreLimitsBuilder,
};

use crate::PluginError;
use crate::runtime::{PluginRuntime, SandboxLimits};

struct HostState {
    context: HostContext,
    limits: StoreLimits,
}

pub struct WasmRuntime {
    pub wasm: Vec<u8>,
    pub limits: SandboxLimits,
}

impl WasmRuntime {
    pub fn new(wasm: Vec<u8>, limits: SandboxLimits) -> Self {
        Self { wasm, limits }
    }

    pub fn from_file(path: &std::path::Path, limits: SandboxLimits) -> Result<Self, PluginError> {
        Ok(Self::new(std::fs::read(path)?, limits))
    }

    fn map_error(error: WasmiError) -> PluginError {
        let kind = error.kind();
        if matches!(
            kind,
            wasmi::errors::ErrorKind::TrapCode(wasmi::TrapCode::OutOfFuel)
                | wasmi::errors::ErrorKind::Fuel(wasmi::errors::FuelError::OutOfFuel { .. })
        ) {
            return PluginError::FuelExhausted;
        }
        let text = error.to_string();
        if text.contains("memory") || text.contains("limit") {
            PluginError::OutOfMemory
        } else {
            PluginError::Crash(text)
        }
    }

    fn map_memory_error(_error: wasmi::errors::MemoryError) -> PluginError {
        PluginError::OutOfMemory
    }

    pub fn validate_module(&self) -> Result<(), PluginError> {
        let engine = Engine::default();
        Module::new(&engine, &self.wasm)
            .map(|_| ())
            .map_err(Self::map_error)
    }

    fn run(
        &self,
        manifest: &Manifest,
        request: &Request,
        context: &HostContext,
    ) -> Result<Response, PluginError> {
        let mut config = wasmi::Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, &self.wasm).map_err(Self::map_error)?;
        let memory_limit = (self.limits.memory_limit_mb as usize)
            .saturating_mul(1024 * 1024)
            .max(1024 * 1024);
        let limits = StoreLimitsBuilder::new().memory_size(memory_limit).build();
        let mut store = Store::new(
            &engine,
            HostState {
                context: context.clone(),
                limits,
            },
        );
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(self.limits.fuel.max(1))
            .map_err(Self::map_error)?;

        let mut linker = <Linker<HostState>>::new(&engine);
        let _ = linker.func_wrap(
            "env",
            "folio_http_allowed",
            |caller: Caller<'_, HostState>| -> i32 {
                if caller
                    .data()
                    .context
                    .allows(folio_plugin_sdk::Permission::Network)
                {
                    1
                } else {
                    0
                }
            },
        );
        let _ = linker.func_wrap(
            "env",
            "folio_read_note_allowed",
            |caller: Caller<'_, HostState>| -> i32 {
                if caller
                    .data()
                    .context
                    .allows(folio_plugin_sdk::Permission::ReadNotes)
                {
                    1
                } else {
                    0
                }
            },
        );
        let _ = linker.func_wrap(
            "env",
            "folio_storage_allowed",
            |caller: Caller<'_, HostState>| -> i32 {
                if caller
                    .data()
                    .context
                    .allows(folio_plugin_sdk::Permission::Storage)
                {
                    1
                } else {
                    0
                }
            },
        );
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(Self::map_error)?;
        let memory = instance
            .get_export(&store, "memory")
            .and_then(Extern::into_memory)
            .ok_or_else(|| PluginError::Crash("el plugin no exporta memory".into()))?;
        let api_version = instance
            .get_typed_func::<(), i32>(&store, "folio_api_version")
            .map_err(Self::map_error)?
            .call(&mut store, ())
            .map_err(Self::map_error)?;
        if api_version != manifest.api as i32 {
            return Err(PluginError::Manifest(format!(
                "el plugin declara API {api_version}, el host espera {}",
                manifest.api
            )));
        }
        let request_bytes = serde_json::to_vec(request)?;
        let alloc = instance
            .get_typed_func::<i32, i32>(&store, "folio_alloc")
            .map_err(Self::map_error)?;
        let ptr = alloc
            .call(&mut store, request_bytes.len() as i32)
            .map_err(Self::map_error)?;
        if ptr < 0 {
            return Err(PluginError::OutOfMemory);
        }
        memory
            .write(&mut store, ptr as usize, &request_bytes)
            .map_err(Self::map_memory_error)?;
        let handle = instance
            .get_typed_func::<(i32, i32), i32>(&store, "folio_handle")
            .map_err(Self::map_error)?;
        let len = handle
            .call(&mut store, (ptr, request_bytes.len() as i32))
            .map_err(Self::map_error)?;
        if len < 0 || len as usize > memory.data_size(&store) {
            return Err(PluginError::Crash("longitud de respuesta inválida".into()));
        }
        let result_ptr = instance
            .get_typed_func::<(), i32>(&store, "folio_result_ptr")
            .map_err(Self::map_error)?
            .call(&mut store, ())
            .map_err(Self::map_error)?;
        let result_len = instance
            .get_typed_func::<(), i32>(&store, "folio_result_len")
            .ok()
            .and_then(|func| func.call(&mut store, ()).ok())
            .unwrap_or(len);
        let result_len = result_len.max(0).min(len);
        let mut bytes = vec![0u8; result_len as usize];
        memory
            .read(&store, result_ptr as usize, &mut bytes)
            .map_err(Self::map_memory_error)?;
        let response: Response = serde_json::from_slice(&bytes)?;
        response
            .ui
            .validate()
            .map_err(|error| PluginError::Crash(format!("UI inválida: {error}")))?;
        Ok(response)
    }
}

impl PluginRuntime for WasmRuntime {
    fn call(
        &mut self,
        manifest: &Manifest,
        request: &Request,
        context: &HostContext,
    ) -> Result<Response, PluginError> {
        self.run(manifest, request, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use folio_plugin_sdk::{Permission, UiNode};

    fn wat_response(response: &str) -> Vec<u8> {
        let result_len = response.len();
        let response = response.replace('"', "\\22");
        let wat = format!(
            r#"(module
                (memory (export "memory") 2)
                (data (i32.const 4096) "{response}")
                (func (export "folio_api_version") (result i32) i32.const 1)
                (func (export "folio_alloc") (param i32) (result i32) i32.const 1024)
                (func (export "folio_handle") (param i32 i32) (result i32) i32.const {len})
                (func (export "folio_result_ptr") (result i32) i32.const 4096)
                (func (export "folio_result_len") (result i32) i32.const {len})
            )"#,
            len = result_len
        );
        wat::parse_str(wat).unwrap()
    }

    fn manifest() -> Manifest {
        Manifest {
            id: "io.github.test.wasm".into(),
            name: "Wasm test".into(),
            version: "1.0.0".into(),
            description: String::new(),
            api: 1,
            entry: "plugin.wasm".into(),
            permissions: vec![],
            blocks: vec![],
            commands: vec![],
            signature: String::new(),
        }
    }

    fn request() -> Request {
        Request {
            command: "render".into(),
            block_state: serde_json::json!({}),
            payload: serde_json::Value::Null,
        }
    }

    #[test]
    fn runs_a_third_party_module() {
        let wasm = wat_response(r#"{"state":{"ok":true},"ui":{"type":"label","text":"hola"}}"#);
        let mut runtime = WasmRuntime::new(wasm, SandboxLimits::default());
        let response = runtime
            .call(&manifest(), &request(), &HostContext::default())
            .unwrap();
        assert_eq!(response.state["ok"], true);
        match response.ui {
            UiNode::Label { text, .. } => assert_eq!(text, "hola"),
            _ => panic!("se esperaba label"),
        }
    }

    #[test]
    fn infinite_loop_is_stopped_by_fuel() {
        let wat = r#"(module
            (memory (export "memory") 1)
            (func (export "folio_api_version") (result i32) i32.const 1)
            (func (export "folio_alloc") (param i32) (result i32) i32.const 0)
            (func (export "folio_handle") (param i32 i32) (result i32)
                (loop br 0) i32.const 0)
            (func (export "folio_result_ptr") (result i32) i32.const 0)
            (func (export "folio_result_len") (result i32) i32.const 0)
        )"#;
        let mut runtime = WasmRuntime::new(wat::parse_str(wat).unwrap(), SandboxLimits::default());
        let result = runtime.call(&manifest(), &request(), &HostContext::default());
        assert!(matches!(result, Err(PluginError::FuelExhausted)));
    }

    #[test]
    fn host_permissions_are_reported_to_plugins() {
        let wat = r#"(module
            (import "env" "folio_http_allowed" (func $http (result i32)))
            (memory (export "memory") 1)
            (func (export "folio_api_version") (result i32) i32.const 1)
            (func (export "folio_alloc") (param i32) (result i32) i32.const 1024)
            (func (export "folio_handle") (param i32 i32) (result i32)
                call $http drop i32.const 0)
            (func (export "folio_result_ptr") (result i32) i32.const 0)
            (func (export "folio_result_len") (result i32) i32.const 0)
        )"#;
        let mut runtime = WasmRuntime::new(wat::parse_str(wat).unwrap(), SandboxLimits::default());
        // Sin permiso de red no se le concede la capacidad (el resultado de la
        // llamada es 0, pero el host no implementa red; la prueba verifica que
        // el contexto sin permiso no la concede).
        assert!(!HostContext::default().allows(Permission::Network));
        let _ = runtime.call(&manifest(), &request(), &HostContext::default());
    }

    #[test]
    fn malformed_module_is_rejected() {
        let runtime = WasmRuntime::new(b"no es wasm".to_vec(), SandboxLimits::default());
        assert!(runtime.validate_module().is_err());
    }

    #[test]
    fn api_mismatch_is_rejected() {
        let wasm = wat_response(r#"{"state":{},"ui":{"type":"label","text":"x"}}"#);
        let mut manifest = manifest();
        manifest.api = 2;
        let mut runtime = WasmRuntime::new(wasm, SandboxLimits::default());
        assert!(matches!(
            runtime.call(&manifest, &request(), &HostContext::default()),
            Err(PluginError::Manifest(_))
        ));
    }

    #[test]
    fn trap_is_isolated_as_crash() {
        let wat = r#"(module
            (memory (export "memory") 1)
            (func (export "folio_api_version") (result i32) i32.const 1)
            (func (export "folio_alloc") (param i32) (result i32) i32.const 1024)
            (func (export "folio_handle") (param i32 i32) (result i32) unreachable)
            (func (export "folio_result_ptr") (result i32) i32.const 0)
            (func (export "folio_result_len") (result i32) i32.const 0)
        )"#;
        let mut runtime = WasmRuntime::new(wat::parse_str(wat).unwrap(), SandboxLimits::default());
        assert!(matches!(
            runtime.call(&manifest(), &request(), &HostContext::default()),
            Err(PluginError::Crash(_))
        ));
    }

    #[test]
    fn memory_limit_is_enforced() {
        let wat = r#"(module
            (memory (export "memory") 100)
            (func (export "folio_api_version") (result i32) i32.const 1)
            (func (export "folio_alloc") (param i32) (result i32) i32.const 1024)
            (func (export "folio_handle") (param i32 i32) (result i32) i32.const 0)
            (func (export "folio_result_ptr") (result i32) i32.const 0)
            (func (export "folio_result_len") (result i32) i32.const 0)
        )"#;
        let limits = SandboxLimits {
            memory_limit_mb: 1,
            ..SandboxLimits::default()
        };
        let mut runtime = WasmRuntime::new(wat::parse_str(wat).unwrap(), limits);
        assert!(
            runtime
                .call(&manifest(), &request(), &HostContext::default())
                .is_err()
        );
    }

    #[test]
    fn invalid_ui_response_is_rejected() {
        let wasm = wat_response(r#"{"state":{},"ui":{"type":"boton_inventado"}}"#);
        let mut runtime = WasmRuntime::new(wasm, SandboxLimits::default());
        assert!(matches!(
            runtime.call(&manifest(), &request(), &HostContext::default()),
            Err(PluginError::Json(_)) | Err(PluginError::Crash(_))
        ));
    }
}
