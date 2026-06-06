//! Shared TS-shaped node defaults applied before schema deserialization.

use serde_json::{Map, Number, Value};

const FIT_CONTENT: &str = "fit_content";

pub(crate) fn normalize_text_default_bounds(obj: &mut Map<String, Value>) {
    let kind = obj
        .get("type")
        .or_else(|| obj.get("kind"))
        .and_then(Value::as_str);
    if kind != Some("text") {
        return;
    }

    if text_has_content(obj) {
        obj.entry("width")
            .or_insert_with(|| Value::String(FIT_CONTENT.into()));
        obj.entry("height")
            .or_insert_with(|| Value::String(FIT_CONTENT.into()));
        return;
    }

    let (w, h) = op_editor_core::default_leaf_node_size("text");
    obj.entry("width")
        .or_insert_with(|| Value::Number(Number::from(w)));
    obj.entry("height")
        .or_insert_with(|| Value::Number(Number::from(h)));
}

fn text_has_content(obj: &Map<String, Value>) -> bool {
    match obj.get("content") {
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
        None => false,
    }
}
