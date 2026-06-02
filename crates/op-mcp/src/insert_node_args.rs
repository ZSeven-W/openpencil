//! Argument normalization for the `insert_node` write tool.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::{ToolErrorCode, ToolOutcome};

pub(super) struct InsertNodeParams {
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub fill_hex: Option<String>,
}

#[allow(clippy::result_large_err)]
pub(super) fn insert_node_params(
    args: &BTreeMap<String, String>,
) -> Result<InsertNodeParams, ToolOutcome> {
    if !args.contains_key("kind") {
        if let Some(data) = args.get("data") {
            return ts_insert_node_params(data);
        }
    }
    let kind = match args.get("kind") {
        Some(s) => normalize_node_kind(s.clone()),
        None => {
            return Err(ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "kind is required".into(),
            ));
        }
    };
    let name = match args.get("name") {
        Some(s) => s.clone(),
        None => {
            return Err(ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            ));
        }
    };
    Ok(InsertNodeParams {
        kind,
        name,
        x: parse_i32_arg(args, "x")?,
        y: parse_i32_arg(args, "y")?,
        width: parse_i32_arg(args, "width")?,
        height: parse_i32_arg(args, "height")?,
        fill_hex: args.get("fill_hex").cloned(),
    })
}

#[allow(clippy::result_large_err)]
fn ts_insert_node_params(raw: &str) -> Result<InsertNodeParams, ToolOutcome> {
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("data must be a JSON object: {e}"),
        )
    })?;
    let Some(obj) = value.as_object() else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "data must be a JSON object".into(),
        ));
    };
    let kind = json_string_field(obj, &["kind", "type"])
        .map(normalize_node_kind)
        .ok_or_else(|| {
            ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "data.type is required".into(),
            )
        })?;
    let name = json_string_field(obj, &["name", "content"]).unwrap_or_else(|| kind.clone());
    Ok(InsertNodeParams {
        kind,
        name,
        x: json_i32_field(obj, "x", Some(0))?,
        y: json_i32_field(obj, "y", Some(0))?,
        width: json_i32_field(obj, "width", Some(100))?,
        height: json_i32_field(obj, "height", Some(100))?,
        fill_hex: json_fill_hex_field(obj)?,
    })
}

#[allow(clippy::result_large_err)]
fn parse_i32_arg(args: &BTreeMap<String, String>, key: &str) -> Result<i32, ToolOutcome> {
    let Some(raw) = args.get(key) else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            format!("{key} is required"),
        ));
    };
    raw.parse::<i32>().map_err(|_| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a decimal i32, got {raw:?}"),
        )
    })
}

fn normalize_node_kind(kind: String) -> String {
    match kind.as_str() {
        "rectangle" => "rect".into(),
        other => other.into(),
    }
}

#[allow(clippy::result_large_err)]
fn json_i32_field(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    default: Option<i32>,
) -> Result<i32, ToolOutcome> {
    let Some(value) = obj.get(key) else {
        return default.ok_or_else(|| {
            ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                format!("data.{key} is required"),
            )
        });
    };
    let Some(raw) = json_scalar_to_string(value) else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("data.{key} must be a string or number"),
        ));
    };
    raw.parse::<i32>().map_err(|_| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("data.{key} must be a decimal i32, got {raw:?}"),
        )
    })
}

fn json_string_field(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| json_scalar_to_string(obj.get(*key)?))
}

#[allow(clippy::result_large_err)]
fn json_fill_hex_field(
    obj: &serde_json::Map<String, Value>,
) -> Result<Option<String>, ToolOutcome> {
    if let Some(v) = obj.get("fill_hex").or_else(|| obj.get("fillHex")) {
        return json_scalar_to_string(v).map(Some).ok_or_else(|| {
            ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "data.fill_hex must be a string".into(),
            )
        });
    }
    let Some(fill) = obj.get("fill") else {
        return Ok(None);
    };
    match fill {
        Value::String(s) => Ok(Some(s.clone())),
        Value::Object(fill_obj) => Ok(json_string_field(fill_obj, &["color"])),
        Value::Array(items) => {
            for item in items {
                if let Value::Object(fill_obj) = item {
                    if let Some(color) = json_string_field(fill_obj, &["color"]) {
                        return Ok(Some(color));
                    }
                }
            }
            Ok(None)
        }
        _ => Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "data.fill must be a hex string, fill object, or fill array".into(),
        )),
    }
}

fn json_scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(v) => Some(v.to_string()),
        _ => None,
    }
}
