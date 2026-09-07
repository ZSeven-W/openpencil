//! Enforce the mobile full-bleed hero contract for marked plan subtasks.

use crate::design_type::DesignForm;
use crate::plan::{OrchestratorPlan, Subtask};
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};
use std::collections::HashSet;

const BLEED_NAME_SUFFIX: &str = " (bleed)";

/// Apply the marked hero contract to one mobile root. The whole section is
/// replaced through `PatchNodeData`, which preserves every existing child id.
/// Returns one accepted edit when the section was repaired, otherwise zero.
pub(crate) fn enforce(sink: &mut dyn DocSink, plan: &OrchestratorPlan, root_id: &str) -> usize {
    if crate::geometry_validation::root_design_form(sink.state(), root_id)
        != DesignForm::MobileScreen
    {
        return 0;
    }

    let Some(section_id) = find_marked_section_id(sink, plan, root_id) else {
        return 0;
    };
    let Some(section) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(section_id.clone()),
    ) else {
        return 0;
    };
    if is_bottom_navigation(section) {
        return 0;
    }
    let Ok(mut section_value) = serde_json::to_value(section) else {
        return 0;
    };
    if section_value
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| name.ends_with(BLEED_NAME_SUFFIX))
    {
        return 0;
    }

    let Some(children) = section_value
        .get("children")
        .and_then(Value::as_array)
        .cloned()
    else {
        return 0;
    };
    let Some(first_index) = children
        .iter()
        .position(|child| !crate::cleanup::is_status_bar_from_json(child))
    else {
        return 0;
    };
    let Some(media_path) = first_media_path(&children, first_index) else {
        return 0;
    };

    let mut next_children = children;
    let Some(media) = value_at_path_mut(&mut next_children, &media_path) else {
        return 0;
    };
    media["width"] = Value::String("fill_container".into());
    if media
        .get("x")
        .is_some_and(|x| !x.is_null() && x.as_f64().is_some())
    {
        media["x"] = json!(0);
    }
    // A none-stack is the media's containing viewport. It must stretch too;
    // otherwise a fixed-width authored stack would still clip the hero.
    if media_path.len() == 2 {
        if let Some(stack) = next_children
            .get_mut(media_path[0])
            .and_then(Value::as_object_mut)
        {
            stack.insert("width".into(), Value::String("fill_container".into()));
        }
    }

    let original_name = section_value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(section_id.as_str())
        .to_string();
    let original_gap = section_value
        .get("gap")
        .cloned()
        .unwrap_or_else(|| json!(0));
    let original_padding = section_value.get("padding").cloned();
    section_value["name"] = Value::String(format!("{original_name}{BLEED_NAME_SUFFIX}"));
    section_value["padding"] = zero_horizontal_padding(original_padding.as_ref());

    let trailing = next_children.split_off(first_index + 1);
    if !trailing.is_empty() {
        let wrapper_id = unique_wrapper_id(sink.state(), &section_id);
        next_children.push(json!({
            "type": "frame",
            "id": wrapper_id,
            "name": format!("{original_name} inset"),
            "layout": "vertical",
            "gap": original_gap,
            "padding": [0, 24],
            "width": "fill_container",
            "height": "fit_content",
            "children": trailing,
        }));
    }
    section_value["children"] = Value::Array(next_children);

    let Ok(patch_json) = serde_json::to_string(&json!({
        "name": section_value.get("name").cloned().unwrap_or(Value::Null),
        "padding": section_value
            .get("padding")
            .cloned()
            .unwrap_or(Value::Null),
        "children": section_value
            .get("children")
            .cloned()
            .unwrap_or(Value::Array(Vec::new())),
    })) else {
        return 0;
    };

    usize::from(sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new(section_id),
        patch_json,
        page_id: None,
    }))
}

fn find_marked_section_id(
    sink: &dyn DocSink,
    plan: &OrchestratorPlan,
    root_id: &str,
) -> Option<String> {
    let root = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    )?;
    let children = root.children()?;
    plan.subtasks
        .iter()
        .filter(|subtask| subtask.bleed_hero)
        .find_map(|subtask| find_section_for_subtask(children, subtask))
}

fn find_section_for_subtask(children: &[PenNode], subtask: &Subtask) -> Option<String> {
    if let Some(generated_id) = subtask.generated_root_id.as_deref() {
        if let Some(child) = children.iter().find(|child| child.id_str() == generated_id) {
            return Some(child.id_str().to_string());
        }
    }
    children
        .iter()
        .find(|child| {
            let name = child.base().name.as_deref();
            name == Some(subtask.label.as_str()) || name == Some(subtask.id.as_str())
        })
        .map(|child| child.id_str().to_string())
}

fn is_bottom_navigation(node: &PenNode) -> bool {
    crate::cleanup::is_bottom_nav_section(node)
        || crate::cleanup::is_trailing_bottom_nav_section(node)
}

/// Rail passes must leave an already-bleeding section and its flush first
/// media alone. This JSON predicate is shared by passes that already inspect
/// serialized node values.
pub(crate) fn is_bleed_section_or_flush_media(value: &Value) -> bool {
    if is_bleed_section_value(value) {
        return true;
    }
    let Some(first) = value
        .get("children")
        .and_then(Value::as_array)
        .and_then(|children| {
            children
                .iter()
                .find(|child| !crate::cleanup::is_status_bar_from_json(child))
        })
    else {
        return false;
    };
    is_flush_media(first)
}

pub(crate) fn is_bleed_section_node(node: &PenNode) -> bool {
    node.base()
        .name
        .as_deref()
        .is_some_and(|name| name.ends_with(BLEED_NAME_SUFFIX))
}

fn is_bleed_section_value(value: &Value) -> bool {
    value
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| name.ends_with(BLEED_NAME_SUFFIX))
}

/// Typed-node bridge for passes that operate before their JSON conversion.
pub(crate) fn is_bleed_section_or_flush_media_node(node: &PenNode) -> bool {
    serde_json::to_value(node)
        .ok()
        .is_some_and(|value| is_bleed_section_or_flush_media(&value))
}

fn is_flush_media(value: &Value) -> bool {
    if value.get("type").and_then(Value::as_str) == Some("image") {
        return value.get("width").and_then(Value::as_str) == Some("fill_container");
    }
    if is_coloured_media(value) {
        return value.get("width").and_then(Value::as_str) == Some("fill_container");
    }
    if value.get("type").and_then(Value::as_str) == Some("frame")
        && layout_str(value) == Some("none")
    {
        return value
            .get("children")
            .and_then(Value::as_array)
            .and_then(|children| {
                children
                    .iter()
                    .find(|child| !crate::cleanup::is_status_bar_from_json(child))
            })
            .is_some_and(is_flush_media);
    }
    false
}

/// Return the path to the first media candidate. The path is relative to the
/// section's children array; a two-element path targets media inside a none
/// stack rather than the stack itself.
fn first_media_path(children: &[Value], first_index: usize) -> Option<Vec<usize>> {
    let first = children.get(first_index)?;
    if first.get("type").and_then(Value::as_str) == Some("image") {
        return Some(vec![first_index]);
    }
    if first.get("type").and_then(Value::as_str) == Some("frame")
        && layout_str(first) == Some("none")
    {
        let nested = first.get("children").and_then(Value::as_array)?;
        let nested_index = nested
            .iter()
            .position(|child| !crate::cleanup::is_status_bar_from_json(child))?;
        let nested_child = nested.get(nested_index)?;
        if nested_child.get("type").and_then(Value::as_str) == Some("image")
            || is_coloured_media(nested_child)
        {
            return Some(vec![first_index, nested_index]);
        }
    }
    if is_coloured_media(first) {
        return Some(vec![first_index]);
    }
    None
}

fn is_coloured_media(value: &Value) -> bool {
    matches!(
        value.get("type").and_then(Value::as_str),
        Some("frame" | "rectangle")
    ) && has_solid_or_gradient_fill(value)
        && !value
            .get("children")
            .and_then(Value::as_array)
            .is_some_and(|children| {
                children
                    .iter()
                    .any(|child| child.get("type").and_then(Value::as_str) == Some("text"))
            })
}

fn has_solid_or_gradient_fill(value: &Value) -> bool {
    value
        .get("fill")
        .and_then(Value::as_array)
        .is_some_and(|fills| {
            fills.iter().any(|fill| {
                matches!(
                    fill.get("type").and_then(Value::as_str),
                    Some("solid" | "linear_gradient" | "radial_gradient")
                )
            })
        })
}

fn layout_str(value: &Value) -> Option<&str> {
    value.get("layout").and_then(Value::as_str)
}

fn value_at_path_mut<'a>(value: &'a mut [Value], path: &[usize]) -> Option<&'a mut Value> {
    let (first, rest) = path.split_first()?;
    let value = value.get_mut(*first)?;
    if rest.is_empty() {
        return Some(value);
    }
    value_at_path_mut(value.get_mut("children")?.as_array_mut()?, rest)
}

fn zero_horizontal_padding(padding: Option<&Value>) -> Value {
    let number = |value: &Value| value.as_f64().unwrap_or(0.0);
    match padding {
        Some(Value::Array(values)) if values.len() == 4 => {
            json!([number(&values[0]), 0, number(&values[2]), 0])
        }
        Some(Value::Array(values)) if values.len() == 2 => json!([number(&values[0]), 0]),
        Some(Value::Number(value)) => json!([number(&Value::Number(value.clone())), 0]),
        _ => json!([0, 0]),
    }
}

fn unique_wrapper_id(state: &op_editor_core::EditorState, section_id: &str) -> String {
    let mut ids = HashSet::new();
    collect_ids(state.active_children(), &mut ids);
    let base = format!("{section_id}-bleed-inset");
    let mut candidate = base.clone();
    let mut suffix = 2;
    while ids.contains(&candidate) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    candidate
}

fn collect_ids(nodes: &[PenNode], ids: &mut HashSet<String>) {
    for node in nodes {
        ids.insert(node.id_str().to_string());
        if let Some(children) = node.children() {
            collect_ids(children, ids);
        }
    }
}

#[cfg(test)]
#[path = "hero_bleed_tests.rs"]
mod tests;
