//! TS-compatible `insert_node(data)` preservation.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use serde_json::Value;

use super::batch_design::normalize_node_shape;
use super::{ToolErrorCode, ToolOutcome};

const FLAT_INSERT_NODE_KINDS: &[&str] = &[
    "frame", "group", "rect", "ellipse", "polygon", "line", "text", "path",
];

#[allow(clippy::result_large_err)]
pub(super) fn ts_data_node(
    args: &BTreeMap<String, String>,
) -> Result<Option<PenNode>, ToolOutcome> {
    let Some(raw) = args.get("data") else {
        return Ok(None);
    };
    let mut value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("data must be a JSON object: {e}"),
        )
    })?;
    let preserve = {
        let Some(obj) = value.as_object() else {
            return Err(ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "data must be a JSON object".into(),
            ));
        };
        should_preserve_ts_data_node(obj)
    };
    if !preserve {
        return Ok(None);
    }

    normalize_node_shape(&mut value);
    normalize_ts_data_node_type_aliases(&mut value);
    let mut next_id = 1usize;
    ensure_ts_data_node_ids(&mut value, &mut next_id);
    let node: PenNode = serde_json::from_value(value).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("data must be a valid PenNode tree: {e}"),
        )
    })?;
    Ok(Some(node))
}

fn should_preserve_ts_data_node(obj: &serde_json::Map<String, Value>) -> bool {
    if obj.contains_key("children") {
        return true;
    }
    let Some(kind) = ts_data_kind(obj) else {
        return false;
    };
    let flat_kind = flat_node_kind(&kind);
    if !FLAT_INSERT_NODE_KINDS.contains(&flat_kind.as_str()) {
        return true;
    }
    if flat_kind == "text" && obj.contains_key("content") {
        return true;
    }
    obj.keys().any(|key| !is_flat_insert_node_data_key(key))
}

fn ts_data_kind(obj: &serde_json::Map<String, Value>) -> Option<String> {
    obj.get("kind")
        .or_else(|| obj.get("type"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn is_flat_insert_node_data_key(key: &str) -> bool {
    matches!(
        key,
        "type" | "kind" | "name" | "x" | "y" | "width" | "height" | "fill" | "fill_hex" | "fillHex"
    )
}

fn normalize_ts_data_node_type_aliases(value: &mut Value) {
    let Value::Object(obj) = value else {
        return;
    };
    let kind_alias = if obj.contains_key("type") {
        None
    } else {
        obj.get("kind")
            .and_then(Value::as_str)
            .map(schema_node_type)
    };
    if let Some(type_name) = kind_alias {
        obj.insert("type".into(), Value::String(type_name));
    } else if let Some(Value::String(type_name)) = obj.get_mut("type") {
        if type_name == "rect" {
            *type_name = "rectangle".into();
        }
    }
    if let Some(Value::Array(children)) = obj.get_mut("children") {
        for child in children {
            normalize_ts_data_node_type_aliases(child);
        }
    }
}

fn ensure_ts_data_node_ids(value: &mut Value, next: &mut usize) {
    let Value::Object(obj) = value else {
        return;
    };
    if obj.contains_key("type") && !obj.contains_key("id") {
        obj.insert("id".into(), Value::String(format!("__op_tmp_data_{next}")));
        *next += 1;
    }
    if let Some(Value::Array(children)) = obj.get_mut("children") {
        for child in children {
            ensure_ts_data_node_ids(child, next);
        }
    }
}

fn flat_node_kind(kind: &str) -> String {
    match kind {
        "rectangle" => "rect".into(),
        other => other.into(),
    }
}

fn schema_node_type(kind: &str) -> String {
    match kind {
        "rect" => "rectangle".into(),
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::PenNode;
    use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

    fn args_with_data(data: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("data".to_string(), data.to_string())])
    }

    fn is_fit_content(value: &Option<SizingBehavior>) -> bool {
        matches!(
            value,
            Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
        )
    }

    #[test]
    fn preserved_text_data_defaults_missing_bounds_to_fit_content_size() {
        let node = ts_data_node(&args_with_data(
            r#"{"type":"text","name":"Label","content":"Text"}"#,
        ))
        .expect("text data parses")
        .expect("text content preserves subtree");
        let PenNode::Text(text) = node else {
            panic!("expected text node");
        };

        assert!(is_fit_content(&text.width));
        assert!(is_fit_content(&text.height));
    }

    #[test]
    fn preserved_text_data_defaults_long_content_to_fit_content_size() {
        let node = ts_data_node(&args_with_data(
            r#"{"type":"text","name":"Label","content":"N4068-测试-35","fontSize":14}"#,
        ))
        .expect("text data parses")
        .expect("text content preserves subtree");
        let PenNode::Text(text) = node else {
            panic!("expected text node");
        };

        assert!(is_fit_content(&text.width));
        assert!(is_fit_content(&text.height));
    }

    #[test]
    fn preserved_nested_text_data_defaults_missing_bounds_to_fit_content_size() {
        let node = ts_data_node(&args_with_data(
            r#"{"type":"frame","name":"Card","width":200,"height":120,"children":[{"type":"text","name":"Label","content":"Text"}]}"#,
        ))
        .expect("frame data parses")
        .expect("children preserve subtree");
        let PenNode::Frame(frame) = node else {
            panic!("expected frame node");
        };
        let [PenNode::Text(text)] = frame.children.as_deref().expect("frame children") else {
            panic!("expected one text child");
        };

        assert!(is_fit_content(&text.width));
        assert!(is_fit_content(&text.height));
    }

    #[test]
    fn preserved_nested_text_data_defaults_long_content_to_fit_content_size() {
        let node = ts_data_node(&args_with_data(
            r#"{"type":"frame","name":"Card","width":200,"height":120,"children":[{"type":"text","name":"Label","content":"N4076-测试-6","fontSize":14}]}"#,
        ))
        .expect("frame data parses")
        .expect("children preserve subtree");
        let PenNode::Frame(frame) = node else {
            panic!("expected frame node");
        };
        let [PenNode::Text(text)] = frame.children.as_deref().expect("frame children") else {
            panic!("expected one text child");
        };

        assert!(is_fit_content(&text.width));
        assert!(is_fit_content(&text.height));
    }
}
