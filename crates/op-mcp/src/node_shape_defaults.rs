//! Shared TS-shaped node defaults applied before schema deserialization.

use serde_json::{Map, Number, Value};

pub(crate) fn normalize_text_default_bounds(obj: &mut Map<String, Value>) {
    let kind = obj
        .get("type")
        .or_else(|| obj.get("kind"))
        .and_then(Value::as_str);
    if kind != Some("text") {
        return;
    }

    let (w, h) = op_editor_core::default_leaf_node_size("text");
    obj.entry("width")
        .or_insert_with(|| Value::Number(Number::from(w)));
    obj.entry("height")
        .or_insert_with(|| Value::Number(Number::from(h)));
}
