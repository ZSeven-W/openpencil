//! Weak-model DIALECT repairs for program-DSL node bodies, applied before
//! the payload meets the typed `PenNode` deserialiser.
//!
//! Every rule here comes from a line the executor dropped in the
//! design-arena-v1 corpus (GLM-5.3-Flash, DeepSeek V4 Pro, space-bunny;
//! `[program-gen] dropped line` in each run's stderr). A dropped container
//! line is the expensive failure: every later line that targets its binding
//! cascades into "Insert parent not found", so one `"type":"table-cell"`
//! took a whole 8-row table with it. The rules are deliberately narrow —
//! each maps a spelling the schema rejects onto the one it accepts with the
//! same intent — and every rewrite is reported as a note so the run log shows
//! where the executor second-guessed the model. Anything not listed keeps
//! failing with the unchanged diagnostic.
//!
//! Node-level rules (type aliases, missing `type`, null children) run only on
//! node bodies (`I()`/`R()`); the field-level rules also run on `U()` patches.

use serde_json::{json, Map, Value};

// Field-level repairs live in a sibling to keep both files small.
pub(crate) use super::batch_program_dialect_fields::repair_field_dialect;

/// Repair one node body (recursively through inline object children).
pub(crate) fn repair_node_dialect(value: &mut Value, notes: &mut Vec<String>) {
    let Value::Object(obj) = value else {
        return;
    };
    infer_missing_type(obj, notes);
    rewrite_structural_type(obj, notes);
    repair_field_dialect(obj, notes);
    if let Some(Value::Array(children)) = obj.get_mut("children") {
        let before = children.len();
        children.retain(|child| !child.is_null());
        if children.len() != before {
            notes.push(format!(
                "dropped {} null children entr{}",
                before - children.len(),
                if before - children.len() == 1 {
                    "y"
                } else {
                    "ies"
                }
            ));
        }
        for child in children.iter_mut() {
            repair_node_dialect(child, notes);
        }
    }
}

/// A body with no `type` is typed by the one field only that type carries.
/// A body with none of them (`{}`) stays untyped and keeps failing.
fn infer_missing_type(obj: &mut Map<String, Value>, notes: &mut Vec<String>) {
    if obj.contains_key("type") {
        return;
    }
    let has = |key: &str| obj.get(key).is_some_and(|v| !v.is_null());
    let inferred = if has("content") {
        "text"
    } else if has("iconFontName") {
        "icon_font"
    } else if has("src") || has("imagePrompt") || has("imageSearchQuery") {
        "image"
    } else if has("children") || has("layout") || has("fill") {
        "frame"
    } else {
        return;
    };
    obj.insert("type".into(), json!(inferred));
    notes.push(format!("inferred missing type as {inferred}"));
}

/// What a non-schema structural type becomes.
enum StructuralAlias {
    /// A plain frame with a default `layout` direction when none is given.
    Frame { layout: &'static str },
    /// A table cell: a horizontal frame whose inline `content` becomes a
    /// text child.
    Cell,
    /// An empty flex spacer: a frame with its paint stripped.
    Spacer,
    /// A hairline rule: a `rectangle`.
    Divider,
    /// A control chip (`button`, `icon-button`, `badge`): a centred
    /// horizontal frame; `iconFontName` / `content` become children.
    Control,
}

fn structural_alias(raw: &str) -> Option<StructuralAlias> {
    let key = raw.trim().to_ascii_lowercase().replace('_', "-");
    Some(match key.as_str() {
        "table" | "table-body" | "tbody" => StructuralAlias::Frame { layout: "vertical" },
        "table-row" | "tr" | "table-header" | "table-head" | "thead" => StructuralAlias::Frame {
            layout: "horizontal",
        },
        "table-cell" | "td" | "th" => StructuralAlias::Cell,
        "spacer" => StructuralAlias::Spacer,
        "divider" | "separator" | "hr" => StructuralAlias::Divider,
        "button" | "icon-button" | "badge" | "chip" | "pill" | "tag" => StructuralAlias::Control,
        _ => return None,
    })
}

fn rewrite_structural_type(obj: &mut Map<String, Value>, notes: &mut Vec<String>) {
    let Some(raw) = obj.get("type").and_then(Value::as_str).map(str::to_string) else {
        return;
    };
    let Some(alias) = structural_alias(&raw) else {
        return;
    };
    let target = match alias {
        StructuralAlias::Frame { layout } => {
            default_key(obj, "layout", json!(layout));
            "frame"
        }
        StructuralAlias::Cell => {
            default_key(obj, "layout", json!("horizontal"));
            default_key(obj, "alignItems", json!("center"));
            lift_label_child(obj, &["content"]);
            "frame"
        }
        StructuralAlias::Spacer => {
            for paint in ["fill", "stroke", "effects"] {
                obj.remove(paint);
            }
            "frame"
        }
        StructuralAlias::Divider => {
            shape_divider(obj);
            "rectangle"
        }
        StructuralAlias::Control => {
            default_key(obj, "layout", json!("horizontal"));
            default_key(obj, "alignItems", json!("center"));
            default_key(obj, "justifyContent", json!("center"));
            lift_icon_child(obj);
            lift_label_child(obj, &["content", "label", "text"]);
            "frame"
        }
    };
    obj.insert("type".into(), json!(target));
    notes.push(format!("rewrote type \"{raw}\" as {target}"));
}

fn default_key(obj: &mut Map<String, Value>, key: &str, value: Value) {
    if obj.get(key).is_none_or(Value::is_null) {
        obj.insert(key.into(), value);
    }
}

/// Typography keys that belong on the text child, not on the frame.
const TEXT_KEYS: [&str; 9] = [
    "fontSize",
    "fontFamily",
    "fontWeight",
    "fontStyle",
    "lineHeight",
    "letterSpacing",
    "textAlign",
    "textGrowth",
    "underline",
];

/// Move an inline label (`content` / `label` / `text`) plus its typography
/// into an appended text child; `color` / `textColor` become the text fill.
fn lift_label_child(obj: &mut Map<String, Value>, label_keys: &[&str]) {
    let Some(content) = label_keys.iter().find_map(|key| match obj.get(*key) {
        Some(Value::String(text)) if !text.trim().is_empty() => Some(json!(text)),
        Some(Value::Number(number)) => Some(json!(number.to_string())),
        _ => None,
    }) else {
        return;
    };
    for key in label_keys {
        obj.remove(*key);
    }
    let mut text = Map::new();
    text.insert("type".into(), json!("text"));
    text.insert("content".into(), content);
    for key in TEXT_KEYS {
        if let Some(value) = obj.remove(key) {
            text.insert(key.into(), value);
        }
    }
    if let Some(color) = obj
        .get("textColor")
        .or_else(|| obj.get("color"))
        .filter(|c| c.is_string())
        .cloned()
    {
        text.insert("fill".into(), color);
    }
    obj.remove("textColor");
    push_child(obj, Value::Object(text));
}

/// A control's own `iconFontName` (or `icon`) becomes an icon_font child.
fn lift_icon_child(obj: &mut Map<String, Value>) {
    let name = ["iconFontName", "icon"]
        .iter()
        .find_map(|key| obj.get(*key).and_then(Value::as_str).map(str::to_string));
    let Some(name) = name.filter(|n| !n.trim().is_empty()) else {
        return;
    };
    obj.remove("iconFontName");
    obj.remove("icon");
    let size = obj.remove("iconSize").unwrap_or_else(|| json!(16));
    let mut icon = Map::new();
    icon.insert("type".into(), json!("icon_font"));
    icon.insert("iconFontName".into(), json!(name));
    if let Some(family) = obj.remove("iconFontFamily") {
        icon.insert("iconFontFamily".into(), family);
    }
    icon.insert("width".into(), size.clone());
    icon.insert("height".into(), size);
    if let Some(color) = obj
        .get("iconColor")
        .or_else(|| obj.get("color"))
        .filter(|c| c.is_string())
        .cloned()
    {
        icon.insert("fill".into(), color);
    }
    obj.remove("iconColor");
    push_child(obj, Value::Object(icon));
}

fn push_child(obj: &mut Map<String, Value>, child: Value) {
    match obj.get_mut("children") {
        Some(Value::Array(children)) => children.push(child),
        _ => {
            obj.insert("children".into(), Value::Array(vec![child]));
        }
    }
}

/// A divider is a 1px rule across its parent's cross axis: horizontal
/// unless it says `vertical` or is authored taller than wide. Its colour
/// comes from `fill`, else its stroke, else `color`.
fn shape_divider(obj: &mut Map<String, Value>) {
    let number = |key: &str| obj.get(key).and_then(Value::as_f64);
    let declared_vertical = ["orientation", "direction"].iter().any(|key| {
        obj.get(*key)
            .and_then(Value::as_str)
            .is_some_and(|v| v.eq_ignore_ascii_case("vertical"))
    });
    let vertical = declared_vertical
        || matches!((number("width"), number("height")), (Some(w), Some(h)) if w < h);
    obj.remove("orientation");
    obj.remove("direction");
    let (main, cross) = if vertical {
        ("height", "width")
    } else {
        ("width", "height")
    };
    default_key(obj, main, json!("fill_container"));
    default_key(obj, cross, json!(1));
    if obj.get("fill").is_none_or(Value::is_null) {
        let from_stroke = obj.remove("stroke").and_then(|stroke| match stroke {
            Value::String(color) => Some(json!(color)),
            Value::Object(mut body) => body.remove("fill").or_else(|| body.remove("color")),
            _ => None,
        });
        if let Some(color) = from_stroke.or_else(|| obj.remove("color")) {
            obj.insert("fill".into(), color);
        }
    }
}
