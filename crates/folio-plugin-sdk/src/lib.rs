//! Contrato de UI declarativa del SDK de plugins de Folio.
//!
//! Los plugins devuelven un árbol `UiNode`; el host lo valida y lo mapea a
//! widgets nativos. Nada del árbol puede ejecutar código ni HTML.

use serde::{Deserialize, Serialize};

pub const MAX_UI_DEPTH: usize = 32;
pub const MAX_UI_NODES: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiNode {
    Column {
        #[serde(default)]
        children: Vec<UiNode>,
    },
    Row {
        #[serde(default)]
        children: Vec<UiNode>,
    },
    Label {
        text: String,
        #[serde(default)]
        style: LabelStyle,
    },
    Heading {
        text: String,
    },
    Button {
        id: String,
        label: String,
        #[serde(default)]
        disabled: bool,
    },
    Checkbox {
        id: String,
        label: String,
        checked: bool,
    },
    Slider {
        id: String,
        #[serde(default)]
        min: f64,
        #[serde(default = "default_max")]
        max: f64,
        value: f64,
    },
    Progress {
        #[serde(default)]
        label: String,
        value: f64,
        #[serde(default)]
        max: f64,
    },
    Chart {
        kind: ChartKind,
        title: String,
        #[serde(default)]
        series: Vec<Series>,
    },
    Diagram {
        #[serde(default)]
        nodes: Vec<DiagramNode>,
        #[serde(default)]
        edges: Vec<DiagramEdge>,
    },
    Table {
        #[serde(default)]
        headers: Vec<String>,
        #[serde(default)]
        rows: Vec<Vec<String>>,
    },
    Separator,
    Spacer {
        #[serde(default)]
        height: f64,
    },
}

fn default_max() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelStyle {
    #[default]
    Body,
    Caption,
    Title,
    Monospace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    Bar,
    Line,
    Pie,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Series {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiagramNode {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiagramEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UiError {
    #[error("profundidad de UI mayor que {MAX_UI_DEPTH}")]
    TooDeep,
    #[error("demasiados nodos de UI (máximo {MAX_UI_NODES})")]
    TooManyNodes,
    #[error("texto de UI demasiado largo")]
    TextTooLong,
    #[error("valor no finito en la UI")]
    NonFinite,
}

impl UiNode {
    /// Valida estructura, tamaño y límites de texto antes de renderizar.
    pub fn validate(&self) -> Result<(), UiError> {
        let mut nodes = 0;
        self.validate_at(0, &mut nodes)
    }

    fn validate_at(&self, depth: usize, nodes: &mut usize) -> Result<(), UiError> {
        if depth > MAX_UI_DEPTH {
            return Err(UiError::TooDeep);
        }
        *nodes += 1;
        if *nodes > MAX_UI_NODES {
            return Err(UiError::TooManyNodes);
        }
        fn text_ok(text: &str) -> Result<(), UiError> {
            if text.chars().count() > 10_000 {
                return Err(UiError::TextTooLong);
            }
            Ok(())
        }
        match self {
            UiNode::Column { children } | UiNode::Row { children } => {
                for child in children {
                    child.validate_at(depth + 1, nodes)?;
                }
            }
            UiNode::Label { text, .. } | UiNode::Heading { text } => text_ok(text)?,
            UiNode::Button { id, label, .. } | UiNode::Checkbox { id, label, .. } => {
                text_ok(id)?;
                text_ok(label)?;
            }
            UiNode::Progress { label, value, max } => {
                text_ok(label)?;
                if !value.is_finite() || !max.is_finite() {
                    return Err(UiError::NonFinite);
                }
            }
            UiNode::Slider {
                value, min, max, ..
            } => {
                if !value.is_finite() || !min.is_finite() || !max.is_finite() {
                    return Err(UiError::NonFinite);
                }
            }
            UiNode::Chart { title, series, .. } => {
                text_ok(title)?;
                for series in series {
                    text_ok(&series.label)?;
                    if series.values.iter().any(|value| !value.is_finite()) {
                        return Err(UiError::NonFinite);
                    }
                }
            }
            UiNode::Diagram { nodes, edges } => {
                for node in nodes {
                    text_ok(&node.label)?;
                }
                for edge in edges {
                    text_ok(&edge.label)?;
                }
            }
            UiNode::Table { headers, rows } => {
                for header in headers {
                    text_ok(header)?;
                }
                for row in rows {
                    for cell in row {
                        text_ok(cell)?;
                    }
                }
            }
            UiNode::Separator | UiNode::Spacer { .. } => {}
        }
        Ok(())
    }

    pub fn count(&self) -> usize {
        let mut nodes = 0;
        let _ = self.validate_at(0, &mut nodes);
        nodes
    }
}

/// Permisos que un plugin declara y el usuario concede.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ReadNotes,
    WriteOwnBlock,
    Storage,
    Network,
    Clipboard,
    Notifications,
    #[default]
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_api")]
    pub api: u32,
    #[serde(default)]
    pub entry: String,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub blocks: Vec<BlockDeclaration>,
    #[serde(default)]
    pub commands: Vec<CommandDeclaration>,
    #[serde(default)]
    pub signature: String,
}

fn default_api() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockDeclaration {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub default_height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandDeclaration {
    pub id: String,
    pub name: String,
}

/// Petición del host a un plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub command: String,
    #[serde(default)]
    pub block_state: serde_json::Value,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub state: serde_json::Value,
    pub ui: UiNode,
}

/// Resultado de una función de host con permisos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    PermissionDenied(&'static str),
    LimitExceeded(&'static str),
    Invalid(&'static str),
}

/// Contexto mínimo de host: reglas que también aplica el host real.
#[derive(Debug, Clone, Default)]
pub struct HostContext {
    pub permissions: Vec<Permission>,
    pub memory_limit_mb: u32,
    pub timeout_ms: u32,
}

impl HostContext {
    pub fn allows(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }

    pub fn http_get(&self, url: &str) -> Result<String, HostError> {
        if !self.allows(Permission::Network) {
            return Err(HostError::PermissionDenied("network"));
        }
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(HostError::Invalid("url"));
        }
        Err(HostError::LimitExceeded(
            "red deshabilitada en el host de pruebas",
        ))
    }

    pub fn read_note(&self) -> Result<&'static str, HostError> {
        if !self.allows(Permission::ReadNotes) {
            return Err(HostError::PermissionDenied("read_notes"));
        }
        Ok("")
    }

    pub fn storage_key(&self, plugin_id: &str, key: &str) -> Result<String, HostError> {
        if !self.allows(Permission::Storage) {
            return Err(HostError::PermissionDenied("storage"));
        }
        if key.contains('/') || key.contains("..") {
            return Err(HostError::Invalid("storage_key"));
        }
        Ok(format!("plugin:{plugin_id}:{key}"))
    }
}
