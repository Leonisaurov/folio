use std::time::{Duration, Instant};

use folio_plugin_sdk::{HostContext, Manifest, Request, Response, UiNode};

use crate::PluginError;
use crate::builtin;

/// Contrato de ejecución. El host real puede ser WASM o un plugin de
/// referencia; la UI y los permisos son idénticos.
pub trait PluginRuntime {
    fn call(
        &mut self,
        manifest: &Manifest,
        request: &Request,
        context: &HostContext,
    ) -> Result<Response, PluginError>;
}

#[derive(Debug, Clone)]
pub struct SandboxLimits {
    pub memory_limit_mb: u32,
    pub timeout: Duration,
    pub max_state_bytes: usize,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            memory_limit_mb: 64,
            timeout: Duration::from_millis(2000),
            max_state_bytes: 1024 * 1024,
        }
    }
}

/// Ejecuta un plugin de referencia con los mismos límites que el host.
#[derive(Default)]
pub struct BuiltinRuntime {
    pub limits: SandboxLimits,
}

impl PluginRuntime for BuiltinRuntime {
    fn call(
        &mut self,
        manifest: &Manifest,
        request: &Request,
        context: &HostContext,
    ) -> Result<Response, PluginError> {
        let payload_size = serde_json::to_vec(request)?.len();
        if payload_size > self.limits.max_state_bytes {
            return Err(PluginError::OutOfMemory);
        }
        if context.memory_limit_mb > 0 && context.memory_limit_mb < 8 {
            return Err(PluginError::OutOfMemory);
        }
        let start = Instant::now();
        let response = builtin::handle(&manifest.id, request, context)?;
        if start.elapsed() > self.limits.timeout {
            return Err(PluginError::FuelExhausted);
        }
        let state_size = serde_json::to_vec(&response.state)?.len();
        if state_size > self.limits.max_state_bytes {
            return Err(PluginError::OutOfMemory);
        }
        response
            .ui
            .validate()
            .map_err(|error| PluginError::Crash(format!("UI inválida: {error}")))?;
        Ok(response)
    }
}

/// Cargador WASM. Mantiene el contrato y falla de forma aislada hasta que el
/// runtime esté habilitado; la app muestra un marcador y conserva el estado.
pub struct WasmRuntime {
    pub wasm: Vec<u8>,
    pub limits: SandboxLimits,
}

impl PluginRuntime for WasmRuntime {
    fn call(
        &mut self,
        _manifest: &Manifest,
        _request: &Request,
        _context: &HostContext,
    ) -> Result<Response, PluginError> {
        if self.wasm.is_empty() {
            return Err(PluginError::Crash("binario wasm vacío".into()));
        }
        Err(PluginError::Crash(
            "runtime WASM no habilitado en esta compilación; usa un plugin de referencia".into(),
        ))
    }
}

/// Envoltorio seguro que convierte un panic de un plugin en error aislado.
pub fn call_isolated<R: PluginRuntime>(
    runtime: &mut R,
    manifest: &Manifest,
    request: &Request,
    context: &HostContext,
) -> Result<Response, PluginError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.call(manifest, request, context)
    }))
    .unwrap_or_else(|_| Err(PluginError::Crash("panic del plugin aislado".into())))
}

pub fn render_marker(manifest: Option<&Manifest>, error: &PluginError) -> UiNode {
    let name = manifest
        .map(|manifest| manifest.name.as_str())
        .unwrap_or("plugin");
    UiNode::Column {
        children: vec![
            UiNode::Heading {
                text: name.to_string(),
            },
            UiNode::Label {
                text: format!("No se pudo ejecutar: {error}"),
                style: folio_plugin_sdk::LabelStyle::Caption,
            },
            UiNode::Button {
                id: "disable-plugin".into(),
                label: "Desactivar plugin".into(),
                disabled: false,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use folio_plugin_sdk::Permission;

    #[test]
    fn builtin_runs_and_validates_ui() {
        let manifest = builtin::manifest_for("io.github.folio.progress").unwrap();
        let mut runtime = BuiltinRuntime::default();
        let response = runtime
            .call(
                &manifest,
                &Request {
                    command: "render".into(),
                    block_state: serde_json::json!({"value": 0.4}),
                    payload: serde_json::Value::Null,
                },
                &HostContext::default(),
            )
            .unwrap();
        assert!(response.state.get("value").is_some());
        assert!(response.ui.count() >= 2);
    }

    #[test]
    fn network_permission_is_denied_by_default() {
        let context = HostContext::default();
        assert_eq!(
            context.http_get("https://example.com"),
            Err(folio_plugin_sdk::HostError::PermissionDenied("network"))
        );
        let context = HostContext {
            permissions: vec![Permission::Network],
            ..Default::default()
        };
        assert!(context.http_get("https://example.com").is_err());
        assert_eq!(
            context.http_get("file:///etc/passwd"),
            Err(folio_plugin_sdk::HostError::Invalid("url"))
        );
    }

    #[test]
    fn panic_is_isolated() {
        struct Panics;
        impl PluginRuntime for Panics {
            fn call(
                &mut self,
                _: &Manifest,
                _: &Request,
                _: &HostContext,
            ) -> Result<Response, PluginError> {
                panic!("boom");
            }
        }
        let manifest = builtin::manifest_for("io.github.folio.progress").unwrap();
        let result = call_isolated(
            &mut Panics,
            &manifest,
            &Request {
                command: "render".into(),
                block_state: serde_json::Value::Null,
                payload: serde_json::Value::Null,
            },
            &HostContext::default(),
        );
        assert!(matches!(result, Err(PluginError::Crash(_))));
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let manifest = builtin::manifest_for("io.github.folio.progress").unwrap();
        let mut runtime = BuiltinRuntime {
            limits: SandboxLimits {
                max_state_bytes: 10,
                ..Default::default()
            },
        };
        let result = runtime.call(
            &manifest,
            &Request {
                command: "render".into(),
                block_state: serde_json::json!({"value": 0.5, "padding": "x".repeat(100)}),
                payload: serde_json::Value::Null,
            },
            &HostContext::default(),
        );
        assert!(matches!(result, Err(PluginError::OutOfMemory)));
    }
}
