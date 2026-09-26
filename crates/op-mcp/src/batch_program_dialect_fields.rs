//! Field-level dialect repairs (see `batch_program_dialect.rs`): single
//! properties written in a shape the schema rejects, each of which used to
//! fail its whole node. Safe on both node bodies and `U()` patches — every
//! rule only rewrites a key that is already present.

use serde_json::{json, Map, Value};

pub(crate) fn repair_field_dialect(obj: &mut Map<String, Value>, notes: &mut Vec<String>) {
    repair_text_align_vertical(obj, notes);
    repair_corner_radius(obj, notes);
    for key in ["leadingIcon", "trailingIcon"] {
        repair_icon_name(obj, key, notes);
    }
    repair_events(obj, notes);
    repair_animations(obj, notes);
    repair_colorless_solid_fills(obj, notes);
}

/// `textAlignVertical` takes `top|middle|bottom`; models write the CSS /
/// flex words (`center`, `start`, `end`).
fn repair_text_align_vertical(obj: &mut Map<String, Value>, notes: &mut Vec<String>) {
    let Some(raw) = obj.get("textAlignVertical").and_then(Value::as_str) else {
        return;
    };
    let mapped = match raw.trim().to_ascii_lowercase().as_str() {
        "center" | "centre" => "middle",
        "start" | "flex-start" => "top",
        "end" | "flex-end" => "bottom",
        _ => return,
    };
    notes.push(format!("textAlignVertical \"{raw}\" read as \"{mapped}\""));
    obj.insert("textAlignVertical".into(), json!(mapped));
}

/// `cornerRadius` is a number or `[tl, tr, br, bl]`. Accept the per-corner
/// OBJECT (`{"topLeft":3,"topRight":3}`, missing corners 0), numeric
/// strings (`"8"`, `"8px"`), and the CSS 1/2/3-value list shorthands.
fn repair_corner_radius(obj: &mut Map<String, Value>, notes: &mut Vec<String>) {
    let Some(raw) = obj.get("cornerRadius") else {
        return;
    };
    let repaired = match raw {
        Value::Object(corners) => {
            let corner = |names: &[&str]| {
                names
                    .iter()
                    .find_map(|name| corners.get(*name).and_then(radius_number))
                    .unwrap_or(0.0)
            };
            let tl = corner(&["topLeft", "tl", "top-left", "top_left"]);
            let tr = corner(&["topRight", "tr", "top-right", "top_right"]);
            let br = corner(&["bottomRight", "br", "bottom-right", "bottom_right"]);
            let bl = corner(&["bottomLeft", "bl", "bottom-left", "bottom_left"]);
            Some(json!([tl, tr, br, bl]))
        }
        Value::String(_) => radius_number(raw).map(|r| json!(r)),
        Value::Array(items) => {
            let numbers: Option<Vec<f64>> = items.iter().map(radius_number).collect();
            match numbers.as_deref() {
                Some([all]) => Some(json!(all)),
                Some([a, b]) => Some(json!([a, b, a, b])),
                Some([a, b, c]) => Some(json!([a, b, c, b])),
                Some([a, b, c, d]) if items.iter().any(|v| !v.is_number()) => {
                    Some(json!([a, b, c, d]))
                }
                _ => None,
            }
        }
        _ => None,
    };
    if let Some(value) = repaired {
        notes.push(format!("cornerRadius {raw} read as {value}"));
        obj.insert("cornerRadius".into(), value);
    }
}

fn radius_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().trim_end_matches("px").trim().parse().ok(),
        _ => None,
    }
}

/// `leadingIcon` / `trailingIcon` are a lucide glyph NAME. Models pass an
/// icon node (`{"iconFontName":"search","width":16,…}`); keep its name.
fn repair_icon_name(obj: &mut Map<String, Value>, key: &str, notes: &mut Vec<String>) {
    let Some(Value::Object(icon)) = obj.get(key) else {
        return;
    };
    let name = ["iconFontName", "name", "icon"]
        .iter()
        .find_map(|k| icon.get(*k).and_then(Value::as_str))
        .map(str::to_string);
    match name {
        Some(name) => {
            notes.push(format!("{key} icon object read as \"{name}\""));
            obj.insert(key.into(), json!(name));
        }
        None => {
            notes.push(format!("dropped {key}: icon object names no glyph"));
            obj.remove(key);
        }
    }
}

/// `events` is one handler map. Models write a list of single-handler maps
/// (`[{"onTap":[…]}]`) or an empty list; merge the maps in order.
fn repair_events(obj: &mut Map<String, Value>, notes: &mut Vec<String>) {
    let Some(Value::Array(list)) = obj.get("events") else {
        return;
    };
    if !list.iter().all(Value::is_object) {
        return;
    }
    let mut merged = Map::new();
    for entry in list {
        if let Value::Object(handlers) = entry {
            for (name, actions) in handlers {
                merged
                    .entry(name.clone())
                    .or_insert_with(|| actions.clone());
            }
        }
    }
    notes.push("events list merged into one handler map".into());
    if merged.is_empty() {
        obj.remove("events");
    } else {
        obj.insert("events".into(), Value::Object(merged));
    }
}

/// `animations` is a list of `mount` / `inView` animations. Wrap a single
/// object; map trigger synonyms; drop an entry whose trigger has no
/// counterpart (`press`, `hover` — interaction states, not entrance motion)
/// rather than the node that carries it.
fn repair_animations(obj: &mut Map<String, Value>, notes: &mut Vec<String>) {
    let Some(value) = obj.get_mut("animations") else {
        return;
    };
    if value.is_object() {
        let single = std::mem::take(value);
        *value = Value::Array(vec![single]);
        notes.push("single animation object wrapped in a list".into());
    }
    let Value::Array(items) = value else {
        return;
    };
    let before = items.len();
    let mut dropped: Vec<String> = Vec::new();
    items.retain_mut(|item| {
        let Some(trigger) = item.get("trigger").and_then(Value::as_str) else {
            return true;
        };
        let canonical = match trigger
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '_'], "")
            .as_str()
        {
            "mount" | "load" | "appear" | "enter" | "onmount" | "onload" => "mount",
            "inview" | "scroll" | "visible" | "viewport" | "oninview" => "inView",
            _ => {
                dropped.push(trigger.to_string());
                return false;
            }
        };
        if canonical != trigger {
            item["trigger"] = json!(canonical);
        }
        true
    });
    if !dropped.is_empty() {
        notes.push(format!(
            "dropped {} animation(s) with unsupported trigger {:?}",
            before - items.len(),
            dropped
        ));
    }
    if items.is_empty() {
        obj.remove("animations");
    }
}

/// A `{"type":"solid"}` fill with no colour cannot paint; drop the entry
/// (the node keeps its default paint) instead of the node.
fn repair_colorless_solid_fills(obj: &mut Map<String, Value>, notes: &mut Vec<String>) {
    let Some(Value::Array(fills)) = obj.get_mut("fill") else {
        return;
    };
    let before = fills.len();
    fills.retain(|fill| {
        !(fill.get("type").and_then(Value::as_str) == Some("solid")
            && fill.get("color").is_none_or(Value::is_null))
    });
    if fills.len() == before {
        return;
    }
    notes.push(format!(
        "dropped {} solid fill(s) with no color",
        before - fills.len()
    ));
    if fills.is_empty() {
        obj.remove("fill");
    }
}
