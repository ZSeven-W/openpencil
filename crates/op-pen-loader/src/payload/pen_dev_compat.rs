//! pen.dev (Pencil) document compatibility for the canonical loader.
//!
//! `.op` grew out of pen.dev's node schema, but a native pen.dev document
//! (`version: "2.x"`, no `formatVersion`) differs in ways the strict schema
//! rejects or misreads (issue #272):
//!
//! - **Paint order.** pen.dev paints the LAST child on top; a `.op` children
//!   array is both flow order and paint order with the FIRST child on top.
//!   Free-form frames reverse entirely. Auto-layout frames keep their flow
//!   children in flow order and move only the absolute children, which are
//!   the only ones that overlap siblings.
//! - `layoutPosition: "absolute"` → the child keeps `x`/`y` and gains
//!   `constraints`, which is how `.op` marks an out-of-flow child.
//! - `stroke` + `strokeWidth` + `strokeAlignment` + caps / joins → one
//!   `stroke` object; `effect` → `effects`; `clip` → `clipContent`;
//!   `enabled: false` → `visible: false`; `fit_content(N)` → `fit_content`;
//!   text without `content` → empty content.
//! - Image fill modes `contain` / `cover` / `fill` → `fit` / `fill` /
//!   `stretch`, and a linear gradient's `rotation` → `.op` angle
//!   `(360 - rotation) % 360`.
//!
//! Everything else pen.dev and `.op` share (icon nodes, string fills,
//! `type: "gradient"`, `$ref` radii) is handled by the general legacy pass
//! that runs right after this one. The mapping follows the converter the
//! issue reporter verified against pen.dev's own PNG exports.

use serde_json::{json, Map, Value};

/// A native pen.dev document: the legacy `2.x` product version as its only
/// version marker. TypeScript OpenPencil always wrote `1.0.0`, and every
/// later writer stamps `formatVersion`, so neither is mistaken for one.
pub(super) fn is_pen_dev_document(root: &Value) -> bool {
    if root.get("formatVersion").is_some() {
        return false;
    }
    root.get("version")
        .and_then(Value::as_str)
        .is_some_and(|version| {
            let mut parts = version.split('.');
            parts.next() == Some("2")
                && parts.all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        })
}

/// Rewrite a native pen.dev document into `.op` shape in place. Returns
/// whether anything changed.
pub(super) fn convert_pen_dev_document(root: &mut Value) -> bool {
    let mut changed = false;
    let Value::Object(root) = root else {
        return false;
    };
    let mut stack: Vec<&mut Value> = Vec::new();
    let mut pages = None;
    for (key, value) in root.iter_mut() {
        match key.as_str() {
            "children" => {
                if let Some(children) = value.as_array_mut() {
                    changed |= reverse_roots(children);
                    stack.extend(children.iter_mut());
                }
            }
            "pages" => pages = value.as_array_mut(),
            _ => {}
        }
    }
    if let Some(pages) = pages {
        for page in pages {
            if let Some(children) = page.get_mut("children").and_then(Value::as_array_mut) {
                changed |= reverse_roots(children);
                stack.extend(children.iter_mut());
            }
        }
    }
    // Parents are converted before their children, so a parent's z-order
    // pass still sees each child's `layoutPosition`.
    while let Some(node) = stack.pop() {
        let Value::Object(map) = node else {
            continue;
        };
        changed |= convert_node(map);
        if let Some(children) = map.get_mut("children").and_then(Value::as_array_mut) {
            stack.extend(children.iter_mut());
        }
    }
    changed
}

/// Top-level boards are free-form siblings: last-on-top → first-on-top.
fn reverse_roots(children: &mut [Value]) -> bool {
    if children.len() < 2 {
        return false;
    }
    children.reverse();
    true
}

fn is_absolute(node: &Value) -> bool {
    node.get("layoutPosition").and_then(Value::as_str) == Some("absolute")
}

/// pen.dev → `.op` child order for one container (see the module doc).
fn reorder_children(map: &mut Map<String, Value>) -> bool {
    // Only an explicit `layout: "none"` is free-form; pen.dev's default
    // layout is shared with `.op`, so a frame without one keeps flow rules.
    let free_form = map.get("layout").and_then(Value::as_str) == Some("none");
    let Some(Value::Array(children)) = map.get_mut("children") else {
        return false;
    };
    if children.len() < 2 {
        return false;
    }
    let original = std::mem::take(children);
    if free_form {
        *children = original.into_iter().rev().collect();
        return true;
    }
    if !original.iter().any(is_absolute) {
        // Pure flow: order is unchanged and nothing overlaps.
        *children = original;
        return false;
    }
    let absolute: Vec<bool> = original.iter().rev().map(is_absolute).collect();
    let mut flow = Vec::new();
    let mut overlays = Vec::new();
    for child in original {
        if is_absolute(&child) {
            overlays.push(child);
        } else {
            flow.push(child);
        }
    }
    // Paint order (reversed) decides each overlay's slot; flow children
    // refill the remaining slots in their original order.
    let mut overlays = overlays.into_iter().rev();
    let mut flow = flow.into_iter();
    *children = absolute
        .into_iter()
        .filter_map(|abs| if abs { overlays.next() } else { flow.next() })
        .collect();
    true
}

fn convert_node(map: &mut Map<String, Value>) -> bool {
    let mut changed = reorder_children(map);

    match map.remove("layoutPosition") {
        Some(Value::String(position)) if position == "absolute" => {
            map.entry("constraints")
                .or_insert_with(|| json!({ "h": "left", "v": "top" }));
            changed = true;
        }
        Some(_) => changed = true,
        None => {}
    }
    for key in ["width", "height"] {
        if let Some(Value::String(size)) = map.get(key) {
            if size.starts_with("fit_content(") {
                map.insert(key.to_string(), json!("fit_content"));
                changed = true;
            }
        }
    }
    if map.get("type").and_then(Value::as_str) == Some("text") && !map.contains_key("content") {
        map.insert("content".to_string(), json!(""));
        changed = true;
    }
    if let Some(clip) = map.remove("clip") {
        map.entry("clipContent").or_insert(clip);
        changed = true;
    }
    if let Some(Value::Bool(enabled)) = map.get("enabled") {
        if !enabled {
            map.insert("visible".to_string(), json!(false));
        }
        map.remove("enabled");
        changed = true;
    }
    changed |= convert_stroke(map);
    changed |= convert_effects(map);
    if let Some(fill) = map.get_mut("fill") {
        changed |= convert_paints(fill);
    }
    changed
}

/// Fold pen.dev's flat stroke fields into the `.op` stroke object.
fn convert_stroke(map: &mut Map<String, Value>) -> bool {
    let width = map.remove("strokeWidth");
    let align = map.remove("strokeAlignment");
    let cap = map.remove("strokeLinecap");
    let join = map.remove("strokeLinejoin");
    let touched = width.is_some() || align.is_some() || cap.is_some() || join.is_some();
    let Some(paint) = map.get("stroke").cloned() else {
        return touched;
    };
    if paint.get("thickness").is_some() {
        // Already an `.op` stroke object.
        return touched;
    }
    if paint.is_null() {
        map.remove("stroke");
        return true;
    }
    let mut fill = paint;
    convert_paints(&mut fill);
    let mut stroke = Map::new();
    stroke.insert("thickness".to_string(), width.unwrap_or_else(|| json!(1)));
    stroke.insert("fill".to_string(), fill);
    if let Some(Value::String(align)) = align {
        let align = match align.as_str() {
            "inner" | "inside" => "inside",
            "outer" | "outside" => "outside",
            _ => "center",
        };
        stroke.insert("align".to_string(), json!(align));
    }
    if let Some(cap) = cap {
        stroke.insert("cap".to_string(), cap);
    }
    if let Some(join) = join {
        stroke.insert("join".to_string(), join);
    }
    map.insert("stroke".to_string(), Value::Object(stroke));
    true
}

/// pen.dev `effect` (one or many) → `.op` `effects`.
fn convert_effects(map: &mut Map<String, Value>) -> bool {
    let Some(effect) = map.remove("effect") else {
        return false;
    };
    let entries = match effect {
        Value::Array(entries) => entries,
        Value::Null => Vec::new(),
        other => vec![other],
    };
    let effects: Vec<Value> = entries
        .iter()
        .filter_map(|entry| {
            let kind = entry.get("type").and_then(Value::as_str)?;
            let visible = entry.get("enabled").and_then(Value::as_bool);
            let mut out = match kind {
                "shadow" => {
                    let offset = entry.get("offset");
                    let axis = |key: &str| {
                        offset
                            .and_then(|o| o.get(key))
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0)
                    };
                    json!({
                        "type": "shadow",
                        "inner": entry.get("shadowType").and_then(Value::as_str) == Some("inner"),
                        "offsetX": axis("x"),
                        "offsetY": axis("y"),
                        "blur": entry.get("blur").and_then(Value::as_f64).unwrap_or(0.0),
                        "spread": entry.get("spread").and_then(Value::as_f64).unwrap_or(0.0),
                        "color": entry.get("color").and_then(Value::as_str).unwrap_or("#00000040"),
                    })
                }
                "blur" | "background_blur" => json!({
                    "type": kind,
                    "radius": entry.get("radius").and_then(Value::as_f64).unwrap_or(0.0),
                }),
                _ => return None,
            };
            if visible == Some(false) {
                out["visible"] = json!(false);
            }
            Some(out)
        })
        .collect();
    if !effects.is_empty() && !map.contains_key("effects") {
        map.insert("effects".to_string(), Value::Array(effects));
    }
    true
}

/// pen.dev-only paint semantics inside a fill value (string, object or
/// array): image fit modes and the linear-gradient angle convention.
fn convert_paints(fill: &mut Value) -> bool {
    match fill {
        Value::Array(items) => items
            .iter_mut()
            .fold(false, |acc, item| convert_paint(item) | acc),
        Value::Object(_) => convert_paint(fill),
        _ => false,
    }
}

fn convert_paint(paint: &mut Value) -> bool {
    let Value::Object(obj) = paint else {
        return false;
    };
    match obj.get("type").and_then(Value::as_str) {
        Some("image") => {
            let mode = match obj.get("mode").and_then(Value::as_str) {
                Some("contain") => "fit",
                Some("cover") => "fill",
                Some("fill") => "stretch",
                _ => return false,
            };
            obj.insert("mode".to_string(), json!(mode));
            true
        }
        Some("gradient") if obj.get("gradientType").and_then(Value::as_str) != Some("radial") => {
            let rotation = obj.get("rotation").and_then(Value::as_f64).unwrap_or(0.0);
            obj.insert(
                "rotation".to_string(),
                json!((360.0 - rotation).rem_euclid(360.0)),
            );
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "pen_dev_compat_tests.rs"]
mod tests;
