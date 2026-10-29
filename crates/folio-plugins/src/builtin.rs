//! Plugins de referencia del MVP, con el mismo contrato que un plugin real.

use std::collections::BTreeMap;

use folio_plugin_sdk::{
    ChartKind, DiagramEdge, DiagramNode, HostContext, HostError, LabelStyle, Manifest, Permission,
    Request, Response, Series, UiNode,
};
use serde_json::{Value, json};

use crate::{PluginError, manifest};

const PROGRESS: &str = r#"
id = "io.github.folio.progress"
name = "Progreso"
version = "1.0.0"
description = "Barra de progreso con objetivo."
api = 1
entry = "plugin.wasm"
permissions = ["write_own_block", "storage"]
[[blocks]]
id = "progress"
name = "Barra de progreso"
default_height = 120.0
[[commands]]
id = "set"
name = "Establecer valor"
"#;

const CHART: &str = r#"
id = "io.github.folio.chart"
name = "Gráficas"
version = "1.0.0"
description = "Gráfica de barras, línea o tarta que puede leer una tabla."
api = 1
entry = "plugin.wasm"
permissions = ["read_notes", "write_own_block"]
[[blocks]]
id = "chart"
name = "Gráfica"
default_height = 280.0
[[commands]]
id = "render"
name = "Renderizar"
"#;

const DIAGRAM: &str = r#"
id = "io.github.folio.diagram"
name = "Diagramas"
version = "1.0.0"
description = "Diagrama a partir de un DSL de líneas y flechas."
api = 1
entry = "plugin.wasm"
permissions = ["write_own_block"]
[[blocks]]
id = "diagram"
name = "Diagrama"
default_height = 320.0
[[commands]]
id = "render"
name = "Renderizar"
"#;

const STATS: &str = r#"
id = "io.github.folio.stats"
name = "Estadísticas"
version = "1.0.0"
description = "Resumen diario de notas, autores y palabras."
api = 1
entry = "plugin.wasm"
permissions = ["read_notes", "write_own_block"]
[[blocks]]
id = "stats"
name = "Estadísticas"
default_height = 220.0
[[commands]]
id = "render"
name = "Renderizar"
"#;

const QUICKBAR: &str = r#"
id = "io.github.folio.quickbar"
name = "Barra rápida"
version = "1.0.0"
description = "Botones de acción configurables."
api = 1
entry = "plugin.wasm"
permissions = ["write_own_block"]
[[blocks]]
id = "quickbar"
name = "Barra rápida"
default_height = 80.0
[[commands]]
id = "render"
name = "Renderizar"
"#;

pub fn manifests() -> Vec<Manifest> {
    [PROGRESS, CHART, DIAGRAM, STATS, QUICKBAR]
        .iter()
        .filter_map(|text| manifest::parse(text).ok())
        .collect()
}

pub fn manifest_for(id: &str) -> Option<Manifest> {
    manifests().into_iter().find(|manifest| manifest.id == id)
}

pub fn handle(
    plugin_id: &str,
    request: &Request,
    context: &HostContext,
) -> Result<Response, PluginError> {
    match plugin_id {
        "io.github.folio.progress" => progress(request),
        "io.github.folio.chart" => chart(request, context),
        "io.github.folio.diagram" => diagram(request),
        "io.github.folio.stats" => stats(request, context),
        "io.github.folio.quickbar" => quickbar(request),
        other => Err(PluginError::NotFound(other.to_string())),
    }
}

fn state_object(request: &Request) -> serde_json::Map<String, Value> {
    match &request.block_state {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    }
}

fn progress(request: &Request) -> Result<Response, PluginError> {
    let mut state = state_object(request);
    let mut value = state.get("value").and_then(Value::as_f64).unwrap_or(0.0);
    let target = state.get("target").and_then(Value::as_f64).unwrap_or(1.0);
    if request.command == "set" {
        if let Some(new_value) = request.payload.get("value").and_then(Value::as_f64) {
            value = new_value;
        }
        if let Some(new_target) = request.payload.get("target").and_then(Value::as_f64) {
            state.insert("target".into(), json!(new_target));
        }
    }
    let target = state
        .get("target")
        .and_then(Value::as_f64)
        .unwrap_or(target);
    value = value.clamp(0.0, target.max(0.0001));
    state.insert("value".into(), json!(value));
    state.insert("target".into(), json!(target));
    let label = state
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("Progreso")
        .to_string();
    let percent = if target > 0.0 {
        value / target * 100.0
    } else {
        0.0
    };
    let ui = UiNode::Column {
        children: vec![
            UiNode::Label {
                text: format!("{label} · {percent:.0} %"),
                style: LabelStyle::Title,
            },
            UiNode::Progress {
                label: label.clone(),
                value,
                max: target,
            },
            UiNode::Slider {
                id: "value".into(),
                min: 0.0,
                max: target,
                value,
            },
        ],
    };
    Ok(Response {
        state: Value::Object(state),
        ui,
    })
}

fn chart(request: &Request, context: &HostContext) -> Result<Response, PluginError> {
    let mut state = state_object(request);
    if let Some(kind) = request.payload.get("kind").and_then(Value::as_str) {
        state.insert("kind".into(), json!(kind));
    }
    if let Some(table) = request.payload.get("table") {
        // La lectura de una tabla de la nota exige permiso de lectura.
        context
            .read_note()
            .map_err(|error| PluginError::Permission(format!("{error:?}")))?;
        let labels = table
            .get("labels")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_else(Vec::new);
        let values = table
            .get("values")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_f64)
                    .collect::<Vec<f64>>()
            })
            .unwrap_or_default();
        state.insert("labels".into(), json!(labels));
        state.insert("values".into(), json!(values));
    }
    let kind = match state.get("kind").and_then(Value::as_str).unwrap_or("bar") {
        "line" => ChartKind::Line,
        "pie" => ChartKind::Pie,
        _ => ChartKind::Bar,
    };
    let title = state
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Gráfica")
        .to_string();
    let labels: Vec<String> = state
        .get("labels")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let values: Vec<f64> = state
        .get("values")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    let series = if labels.is_empty() {
        vec![Series {
            label: title.clone(),
            values,
        }]
    } else {
        labels
            .iter()
            .zip(values.iter())
            .map(|(label, value)| Series {
                label: label.clone(),
                values: vec![*value],
            })
            .collect()
    };
    let ui = UiNode::Column {
        children: vec![
            UiNode::Heading {
                text: title.clone(),
            },
            UiNode::Chart {
                kind,
                title: title.clone(),
                series,
            },
        ],
    };
    Ok(Response {
        state: Value::Object(state),
        ui,
    })
}

fn diagram(request: &Request) -> Result<Response, PluginError> {
    let mut state = state_object(request);
    if let Some(dsl) = request.payload.get("dsl").and_then(Value::as_str) {
        state.insert("dsl".into(), json!(dsl));
    }
    let dsl = state
        .get("dsl")
        .and_then(Value::as_str)
        .unwrap_or("Inicio -> Proceso\nProceso -> Fin : listo")
        .to_string();
    let (nodes, edges) = parse_dsl(&dsl);
    let ui = UiNode::Diagram {
        nodes: nodes.clone(),
        edges: edges.clone(),
    };
    state.insert(
        "nodes".into(),
        json!(
            nodes
                .iter()
                .map(|node| node.label.clone())
                .collect::<Vec<_>>()
        ),
    );
    Ok(Response {
        state: Value::Object(state),
        ui,
    })
}

/// DSL: una arista por línea `A -> B : etiqueta`; los nodos se deducen.
pub fn parse_dsl(dsl: &str) -> (Vec<DiagramNode>, Vec<DiagramEdge>) {
    let mut nodes: BTreeMap<String, DiagramNode> = BTreeMap::new();
    let mut edges = Vec::new();
    for line in dsl.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (left, right) = match line.split_once("->") {
            Some(parts) => parts,
            None => {
                nodes
                    .entry(line.to_string())
                    .or_insert_with(|| DiagramNode {
                        id: slug(line),
                        label: line.to_string(),
                    });
                continue;
            }
        };
        let (right, label) = match right.split_once(':') {
            Some((node, label)) => (node.trim(), label.trim()),
            None => (right.trim(), ""),
        };
        let left = left.trim();
        for name in [left, right] {
            nodes
                .entry(name.to_string())
                .or_insert_with(|| DiagramNode {
                    id: slug(name),
                    label: name.to_string(),
                });
        }
        edges.push(DiagramEdge {
            from: slug(left),
            to: slug(right),
            label: label.to_string(),
        });
    }
    (nodes.into_values().collect(), edges)
}

fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn stats(request: &Request, context: &HostContext) -> Result<Response, PluginError> {
    context
        .read_note()
        .map_err(|error| PluginError::Permission(format!("{error:?}")))?;
    let state = state_object(request);
    let notes = state.get("notes").and_then(Value::as_u64).unwrap_or(0);
    let books = state.get("books").and_then(Value::as_u64).unwrap_or(0);
    let words = state.get("words").and_then(Value::as_u64).unwrap_or(0);
    let ui = UiNode::Column {
        children: vec![
            UiNode::Heading {
                text: "Estadísticas".into(),
            },
            UiNode::Table {
                headers: vec!["Métrica".into(), "Valor".into()],
                rows: vec![
                    vec!["Notas".into(), notes.to_string()],
                    vec!["Libros".into(), books.to_string()],
                    vec!["Palabras".into(), words.to_string()],
                ],
            },
        ],
    };
    Ok(Response {
        state: Value::Object(state),
        ui,
    })
}

fn quickbar(request: &Request) -> Result<Response, PluginError> {
    let state = state_object(request);
    let actions: Vec<String> = state
        .get("actions")
        .and_then(Value::as_array)
        .map(|actions| {
            actions
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(|| vec!["Nueva nota".into(), "Buscar".into(), "Hoy".into()]);
    let ui = UiNode::Row {
        children: actions
            .iter()
            .enumerate()
            .map(|(index, action)| UiNode::Button {
                id: format!("action-{index}"),
                label: action.clone(),
                disabled: false,
            })
            .collect(),
    };
    Ok(Response {
        state: Value::Object(state),
        ui,
    })
}

/// Comprueba que un plugin no puede acceder a una capacidad no concedida.
pub fn require(context: &HostContext, permission: Permission) -> Result<(), HostError> {
    if context.allows(permission) {
        Ok(())
    } else {
        Err(HostError::PermissionDenied(match permission {
            Permission::Network => "network",
            Permission::ReadNotes => "read_notes",
            Permission::WriteOwnBlock => "write_own_block",
            Permission::Storage => "storage",
            Permission::Clipboard => "clipboard",
            Permission::Notifications => "notifications",
            Permission::None => "none",
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> HostContext {
        HostContext {
            permissions: vec![
                Permission::ReadNotes,
                Permission::WriteOwnBlock,
                Permission::Storage,
            ],
            ..Default::default()
        }
    }

    #[test]
    fn five_reference_plugins_and_frozen_api() {
        let manifests = manifests();
        assert_eq!(manifests.len(), 5);
        assert!(manifests.iter().all(|manifest| manifest.api == 1));
        for id in [
            "io.github.folio.progress",
            "io.github.folio.chart",
            "io.github.folio.diagram",
            "io.github.folio.stats",
            "io.github.folio.quickbar",
        ] {
            assert!(manifests.iter().any(|manifest| manifest.id == id));
        }
    }

    #[test]
    fn all_reference_plugins_render() {
        for manifest in manifests() {
            let request = Request {
                command: "render".into(),
                block_state: json!({}),
                payload: Value::Null,
            };
            let response = handle(&manifest.id, &request, &context()).unwrap();
            response.ui.validate().unwrap();
            assert!(response.ui.count() > 0, "{} sin UI", manifest.id);
        }
    }

    #[test]
    fn chart_reads_table_with_permission() {
        let request = Request {
            command: "render".into(),
            block_state: json!({"kind": "line", "title": "Ventas"}),
            payload: json!({"table": {"labels": ["Ene", "Feb"], "values": [3.0, 7.0]}}),
        };
        let response = handle("io.github.folio.chart", &request, &context()).unwrap();
        assert_eq!(response.state["values"], json!([3.0, 7.0]));
        let denied = handle("io.github.folio.chart", &request, &HostContext::default());
        assert!(matches!(denied, Err(PluginError::Permission(_))));
    }

    #[test]
    fn diagram_dsl_parses_edges_and_dedupes_nodes() {
        let (nodes, edges) = parse_dsl("A -> B : uno\nB -> C\nA -> C");
        assert_eq!(nodes.len(), 3);
        assert_eq!(edges.len(), 3);
        assert_eq!(edges[0].label, "uno");
    }

    #[test]
    fn progress_command_updates_and_clamps() {
        let request = Request {
            command: "set".into(),
            block_state: json!({"value": 0.2, "target": 2.0, "label": "Leer"}),
            payload: json!({"value": 5.0}),
        };
        let response = handle("io.github.folio.progress", &request, &context()).unwrap();
        assert_eq!(response.state["value"], json!(2.0));
        assert_eq!(response.state["target"], json!(2.0));
    }

    #[test]
    fn quickbar_actions_are_buttons() {
        let request = Request {
            command: "render".into(),
            block_state: json!({"actions": ["A", "B"]}),
            payload: Value::Null,
        };
        let response = handle("io.github.folio.quickbar", &request, &context()).unwrap();
        match response.ui {
            UiNode::Row { children } => assert_eq!(children.len(), 2),
            _ => panic!("se esperaba fila"),
        }
    }
}
