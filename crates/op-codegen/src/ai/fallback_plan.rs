//! Deterministic planning fallback for AI codegen. Used only when the
//! planner responds with text that cannot be parsed as a `CodePlan`.

use serde_json::{Map, Value};

use crate::ai::parse::sanitize_name;
use crate::ai::types::{CodePlan, PlannedChunk, RootLayout};

pub(crate) fn fallback_plan_from_nodes_json(nodes_json: &str) -> Option<CodePlan> {
    let value: Value = serde_json::from_str(nodes_json).ok()?;
    let roots: Vec<&Value> = match &value {
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => vec![&value],
        _ => Vec::new(),
    };

    let mut chunks = Vec::new();
    for node in &roots {
        if let Some(chunk) = chunk_from_node(node, chunks.len() + 1) {
            chunks.push(chunk);
        }
    }
    if chunks.is_empty() {
        return None;
    }

    Some(CodePlan {
        chunks,
        shared_styles: Vec::new(),
        root_layout: root_layout_from_node(roots.first().copied()),
    })
}

fn chunk_from_node(node: &Value, index: usize) -> Option<PlannedChunk> {
    let map = node.as_object()?;
    let id = map
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())?;
    let label = node_label(map, index);
    let suggested = {
        let sanitized = sanitize_name(&label);
        if sanitized.is_empty() {
            format!("Chunk{index}")
        } else {
            sanitized
        }
    };

    Some(PlannedChunk {
        id: format!("chunk-{index}"),
        name: label_to_kebab(&label, index),
        node_ids: vec![id.to_string()],
        role: map
            .get("role")
            .and_then(Value::as_str)
            .or_else(|| map.get("type").and_then(Value::as_str))
            .unwrap_or("component")
            .to_string(),
        suggested_component_name: suggested,
        dependencies: Vec::new(),
    })
}

fn node_label(map: &Map<String, Value>, index: usize) -> String {
    let has_name = map
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .is_some();
    let mut label = map
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| map.get("role").and_then(Value::as_str))
        .or_else(|| map.get("type").and_then(Value::as_str))
        .filter(|s| !s.trim().is_empty())
        .map(str::trim)
        .unwrap_or("chunk")
        .to_string();
    if !has_name {
        label.push_str(&format!(" {index}"));
    }
    label
}

fn label_to_kebab(label: &str, index: usize) -> String {
    let mut out = String::new();
    let mut last_was_sep = true;
    for c in label.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('-');
            last_was_sep = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        format!("chunk-{index}")
    } else {
        out
    }
}

fn root_layout_from_node(node: Option<&Value>) -> RootLayout {
    let Some(map) = node.and_then(Value::as_object) else {
        return default_root_layout();
    };
    RootLayout {
        direction: root_direction(map),
        gap: first_number(
            map,
            &["gap", "itemSpacing", "spacing", "rowGap", "columnGap"],
        )
        .unwrap_or(0.0),
        responsive: true,
    }
}

fn default_root_layout() -> RootLayout {
    RootLayout {
        direction: "vertical".to_string(),
        gap: 0.0,
        responsive: true,
    }
}

fn root_direction(map: &Map<String, Value>) -> String {
    let raw = map
        .get("layoutMode")
        .or_else(|| map.get("layoutDirection"))
        .or_else(|| map.get("flexDirection"))
        .or_else(|| map.get("direction"))
        .and_then(Value::as_str)
        .unwrap_or("");
    match raw.to_ascii_lowercase().as_str() {
        "horizontal" | "row" => "horizontal".to_string(),
        _ => "vertical".to_string(),
    }
}

fn first_number(map: &Map<String, Value>, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| match map.get(*key)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_plan_uses_selected_node_names_and_ids() {
        let plan = fallback_plan_from_nodes_json(
            r#"[{"type":"frame","id":"hero","name":"Hero","layoutMode":"horizontal","gap":12}]"#,
        )
        .expect("fallback plan");

        assert_eq!(plan.chunks.len(), 1);
        assert_eq!(plan.chunks[0].id, "chunk-1");
        assert_eq!(plan.chunks[0].node_ids, vec!["hero"]);
        assert_eq!(plan.chunks[0].suggested_component_name, "Hero");
        assert_eq!(plan.root_layout.direction, "horizontal");
        assert_eq!(plan.root_layout.gap, 12.0);
    }
}
