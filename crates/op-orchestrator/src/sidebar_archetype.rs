use std::collections::HashMap;

use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, EditorState, LayoutPropValue, NodeId};
use serde_json::Value;

use crate::types::DocSink;

const MAX_RAIL_WIDTH: f64 = 320.0;
const LINK_SLOT_WIDTH: f64 = 72.0;
const LARGE_TEXT_SIZE: f64 = 40.0;
const CLAMPED_TEXT_SIZE: f32 = 28.0;
const MAX_PADDING_RATIO: f64 = 0.40;
const SIDEBAR_PADDING_X: f64 = 20.0;
const DIAGNOSTIC_TEXT: &str = "sidebar contains a horizontal navbar archetype — restack vertically";

pub(crate) fn repair_sidebar_navbar_archetype(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let Some(root) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    ) else {
        return false;
    };
    let Ok(v) = serde_json::to_value(root) else {
        return false;
    };
    let widths = resolved_widths(sink.state());
    let mut repairs = Repairs::default();
    collect_repairs_for_root(&v, |id| widths.get(id).copied(), &mut repairs);
    if repairs.is_empty() {
        return false;
    }
    apply_repairs(sink, repairs)
}

pub(crate) fn horizontal_navbar_archetype_diagnostics<F>(
    v: &Value,
    resolved_width: F,
) -> Vec<String>
where
    F: Fn(&str) -> Option<f64> + Copy,
{
    let mut out = Vec::new();
    if layout_str(v) != Some("horizontal") {
        return out;
    }
    let root_children = children(v);
    if root_children.len() < 2 {
        return out;
    }
    for rail in root_children {
        let Some(rail_width) = numeric(rail, "width") else {
            continue;
        };
        if rail_width > MAX_RAIL_WIDTH || !looks_like_sidebar_rail(rail) {
            continue;
        }
        if has_squeezed_navbar(rail, rail_width, resolved_width) {
            out.push(format!("{}: {DIAGNOSTIC_TEXT}", diag_label(rail)));
        }
    }
    out
}

#[derive(Debug, Default)]
struct Repairs {
    restack_rows: Vec<String>,
    outer_rows: Vec<String>,
    fill_width: Vec<String>,
    fit_width: Vec<String>,
    start_justify: Vec<String>,
    clamp_text: Vec<String>,
    fit_height: Vec<String>,
    padding: Vec<(String, [f64; 4])>,
}

impl Repairs {
    fn is_empty(&self) -> bool {
        self.restack_rows.is_empty()
            && self.fill_width.is_empty()
            && self.fit_width.is_empty()
            && self.start_justify.is_empty()
            && self.clamp_text.is_empty()
            && self.fit_height.is_empty()
            && self.padding.is_empty()
    }
}

#[derive(Debug)]
struct SqueezedHit {
    squeezed_id: String,
    outer_id: String,
}

fn collect_repairs_for_root<F>(root: &Value, resolved_width: F, repairs: &mut Repairs)
where
    F: Fn(&str) -> Option<f64> + Copy,
{
    if layout_str(root) != Some("horizontal") {
        return;
    }
    let root_children = children(root);
    if root_children.len() < 2 {
        return;
    }
    for rail in root_children {
        let Some(rail_width) = numeric(rail, "width") else {
            continue;
        };
        if rail_width > MAX_RAIL_WIDTH || !looks_like_sidebar_rail(rail) {
            continue;
        }
        let mut hits = Vec::new();
        collect_squeezed_hits(
            rail,
            Some(rail_width),
            resolved_width,
            &mut Vec::new(),
            &mut hits,
        );
        if hits.is_empty() {
            continue;
        }
        for hit in hits {
            push_unique(&mut repairs.restack_rows, hit.squeezed_id.clone());
            push_unique(&mut repairs.restack_rows, hit.outer_id.clone());
            push_unique(&mut repairs.outer_rows, hit.outer_id);
        }
        collect_restack_dependent_repairs(rail, repairs);
        collect_large_text_repairs(rail, &mut Vec::new(), repairs);
        collect_padding_repairs(rail, rail_width, repairs);
    }
}

fn has_squeezed_navbar<F>(rail: &Value, rail_width: f64, resolved_width: F) -> bool
where
    F: Fn(&str) -> Option<f64> + Copy,
{
    let mut hits = Vec::new();
    collect_squeezed_hits(
        rail,
        Some(rail_width),
        resolved_width,
        &mut Vec::new(),
        &mut hits,
    );
    !hits.is_empty()
}

fn collect_squeezed_hits<F>(
    v: &Value,
    parent_available: Option<f64>,
    resolved_width: F,
    horizontal_ancestors: &mut Vec<String>,
    hits: &mut Vec<SqueezedHit>,
) where
    F: Fn(&str) -> Option<f64> + Copy,
{
    let available = available_width(v, parent_available, resolved_width);
    let is_horizontal = layout_str(v) == Some("horizontal");
    if is_horizontal {
        let direct_text_count = direct_text_child_count(v);
        let link_count = direct_nav_item_count(v).max(direct_text_count);
        if direct_text_count >= 2
            && available.is_some_and(|w| w < link_count as f64 * LINK_SLOT_WIDTH)
        {
            if let Some(squeezed_id) = id(v) {
                let outer_id = horizontal_ancestors
                    .last()
                    .cloned()
                    .unwrap_or_else(|| squeezed_id.to_string());
                hits.push(SqueezedHit {
                    squeezed_id: squeezed_id.to_string(),
                    outer_id,
                });
            }
        }
        if let Some(current_id) = id(v) {
            horizontal_ancestors.push(current_id.to_string());
        }
    }

    for child in children(v) {
        collect_squeezed_hits(child, available, resolved_width, horizontal_ancestors, hits);
    }

    if is_horizontal && id(v).is_some() {
        horizontal_ancestors.pop();
    }
}

fn collect_restack_dependent_repairs(v: &Value, repairs: &mut Repairs) {
    let current_id = id(v);
    let restacked = current_id.is_some_and(|vid| contains_id(&repairs.restack_rows, vid));
    let outer = current_id.is_some_and(|vid| contains_id(&repairs.outer_rows, vid));

    if restacked {
        if v.get("justifyContent").and_then(Value::as_str) == Some("space_between") {
            if let Some(vid) = current_id {
                push_unique(&mut repairs.start_justify, vid.to_string());
            }
        }
        for child in children(v) {
            if child.get("type").and_then(Value::as_str) == Some("text") {
                if let Some(child_id) = id(child) {
                    push_unique(&mut repairs.fill_width, child_id.to_string());
                }
            }
        }
    }
    if outer {
        for child in children(v) {
            let Some(child_id) = id(child) else {
                continue;
            };
            if contains_id(&repairs.restack_rows, child_id) {
                continue;
            }
            if is_container(child) && layout_str(child) == Some("horizontal") {
                push_unique(&mut repairs.fit_width, child_id.to_string());
            }
        }
    }

    for child in children(v) {
        collect_restack_dependent_repairs(child, repairs);
    }
}

fn collect_large_text_repairs(
    v: &Value,
    fixed_height_stack: &mut Vec<String>,
    repairs: &mut Repairs,
) {
    let pushed = if is_container(v) && numeric(v, "height").is_some() {
        if let Some(vid) = id(v) {
            fixed_height_stack.push(vid.to_string());
            true
        } else {
            false
        }
    } else {
        false
    };

    if v.get("type").and_then(Value::as_str) == Some("text")
        && numeric(v, "fontSize").is_some_and(|size| size >= LARGE_TEXT_SIZE)
    {
        if let Some(vid) = id(v) {
            push_unique(&mut repairs.clamp_text, vid.to_string());
        }
        if let Some(ancestor_id) = fixed_height_stack.last() {
            push_unique(&mut repairs.fit_height, ancestor_id.clone());
        }
    }

    for child in children(v) {
        collect_large_text_repairs(child, fixed_height_stack, repairs);
    }

    if pushed {
        fixed_height_stack.pop();
    }
}

fn collect_padding_repairs(v: &Value, rail_width: f64, repairs: &mut Repairs) {
    if let Some(edges) = padding_edges(v) {
        let horizontal = edges[1] + edges[3];
        if horizontal > rail_width * MAX_PADDING_RATIO {
            if let Some(vid) = id(v) {
                push_padding_unique(
                    &mut repairs.padding,
                    vid.to_string(),
                    [edges[0], SIDEBAR_PADDING_X, edges[2], SIDEBAR_PADDING_X],
                );
            }
        }
    }
    for child in children(v) {
        collect_padding_repairs(child, rail_width, repairs);
    }
}

fn apply_repairs(sink: &mut dyn DocSink, repairs: Repairs) -> bool {
    let mut changed = false;
    for id in repairs.restack_rows {
        changed |= sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id.clone()),
            property: "layout".to_string(),
            value: LayoutPropValue::Keyword("vertical".to_string()),
        });
        changed |= sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id),
            property: "gap".to_string(),
            value: LayoutPropValue::Number(8.0),
        });
    }
    for id in repairs.start_justify {
        changed |= sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id),
            property: "justifyContent".to_string(),
            value: LayoutPropValue::Keyword("start".to_string()),
        });
    }
    for id in repairs.fill_width {
        changed |= sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id),
            property: "width".to_string(),
            value: LayoutPropValue::Keyword("fill_container".to_string()),
        });
    }
    for id in repairs.fit_width {
        changed |= sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id),
            property: "width".to_string(),
            value: LayoutPropValue::Keyword("fit_content".to_string()),
        });
    }
    for id in repairs.clamp_text {
        changed |= sink.apply(EditorCommand::SetNodeFontSize {
            node_id: NodeId::new(id),
            font_size: CLAMPED_TEXT_SIZE,
        });
    }
    for id in repairs.fit_height {
        changed |= sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id),
            property: "height".to_string(),
            value: LayoutPropValue::Keyword("fit_content".to_string()),
        });
    }
    for (id, padding) in repairs.padding {
        changed |= sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id),
            property: "padding".to_string(),
            value: LayoutPropValue::NumberArray(padding.to_vec()),
        });
    }
    changed
}

fn resolved_widths(state: &EditorState) -> HashMap<String, f64> {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let mut out = HashMap::new();
    for page in &scene.pages {
        collect_widths(&page.children, &mut out);
    }
    out
}

fn collect_widths(nodes: &[SceneNode], out: &mut HashMap<String, f64>) {
    for node in nodes {
        let bounds = node.aggregate_bounds();
        out.insert(node.id.clone(), f64::from(bounds.size.x));
        collect_widths(&node.children, out);
    }
}

fn available_width<F>(v: &Value, parent_available: Option<f64>, resolved_width: F) -> Option<f64>
where
    F: Fn(&str) -> Option<f64> + Copy,
{
    let base = numeric(v, "width")
        .or_else(|| id(v).and_then(resolved_width))
        .or(parent_available)?;
    Some((base - horizontal_padding(v)).max(0.0))
}

fn direct_text_child_count(v: &Value) -> usize {
    children(v)
        .iter()
        .filter(|child| child.get("type").and_then(Value::as_str) == Some("text"))
        .count()
}

fn direct_nav_item_count(v: &Value) -> usize {
    children(v)
        .iter()
        .filter(|child| {
            child.get("type").and_then(Value::as_str) == Some("text")
                || (looks_like_nav_item(child) && contains_text(child))
        })
        .count()
}

fn contains_text(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("text") || children(v).iter().any(contains_text)
}

fn looks_like_sidebar_rail(v: &Value) -> bool {
    let text = ident_text(v);
    text.contains("sidebar")
        || text.contains("side nav")
        || text.contains("side-nav")
        || text.contains("sidenav")
        || text.contains("side rail")
        || text.contains("nav rail")
        || text.contains("navigation rail")
        || text.contains("menu rail")
}

fn looks_like_nav_item(v: &Value) -> bool {
    let text = ident_text(v);
    text.contains("link") || text.contains("nav") || text.contains("menu")
}

fn ident_text(v: &Value) -> String {
    let mut out = String::new();
    for key in ["id", "name", "role"] {
        if let Some(s) = v.get(key).and_then(Value::as_str) {
            out.push(' ');
            out.push_str(&s.to_lowercase());
        }
    }
    out
}

fn padding_edges(v: &Value) -> Option<[f64; 4]> {
    match v.get("padding") {
        Some(Value::Number(n)) => {
            let p = n.as_f64()?;
            Some([p, p, p, p])
        }
        Some(Value::Array(a)) => match a.as_slice() {
            [all] => {
                let p = all.as_f64()?;
                Some([p, p, p, p])
            }
            [vertical, horizontal] => {
                let v = vertical.as_f64()?;
                let h = horizontal.as_f64()?;
                Some([v, h, v, h])
            }
            [top, horizontal, bottom] => {
                let t = top.as_f64()?;
                let h = horizontal.as_f64()?;
                let b = bottom.as_f64()?;
                Some([t, h, b, h])
            }
            [top, right, bottom, left] => Some([
                top.as_f64()?,
                right.as_f64()?,
                bottom.as_f64()?,
                left.as_f64()?,
            ]),
            _ => None,
        },
        _ => None,
    }
}

fn horizontal_padding(v: &Value) -> f64 {
    padding_edges(v).map(|p| p[1] + p[3]).unwrap_or(0.0)
}

fn is_container(v: &Value) -> bool {
    matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "group" | "rectangle" | "ellipse")
    )
}

fn layout_str(v: &Value) -> Option<&str> {
    v.get("layout").and_then(Value::as_str)
}

fn numeric(v: &Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn id(v: &Value) -> Option<&str> {
    v.get("id").and_then(Value::as_str)
}

fn diag_label(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("frame");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
    format!("{name} ({id})")
}

fn push_unique(target: &mut Vec<String>, id: String) {
    if !target.iter().any(|existing| existing == &id) {
        target.push(id);
    }
}

fn push_padding_unique(target: &mut Vec<(String, [f64; 4])>, id: String, padding: [f64; 4]) {
    if !target.iter().any(|(existing, _)| existing == &id) {
        target.push((id, padding));
    }
}

fn contains_id(ids: &[String], id: &str) -> bool {
    ids.iter().any(|existing| existing == id)
}
