//! TS-compatible `update_node(data)` patch detection.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{ToolErrorCode, ToolOutcome};

#[allow(clippy::result_large_err)]
pub(super) fn ts_update_patch_json(
    args: &BTreeMap<String, String>,
) -> Result<Option<String>, ToolOutcome> {
    let Some(raw) = args.get("data") else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("data must be a JSON object: {e}"),
        )
    })?;
    if !value.is_object() {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "data must be a JSON object".into(),
        ));
    };
    Ok(ts_update_patch_json_value(&value))
}

pub(super) fn ts_update_patch_json_value(value: &Value) -> Option<String> {
    let obj = value.as_object()?;
    if obj.is_empty() || obj.keys().any(|key| !is_flat_update_data_key(key)) {
        Some(value.to_string())
    } else {
        None
    }
}

fn is_flat_update_data_key(key: &str) -> bool {
    matches!(
        key,
        "name" | "x" | "y" | "width" | "height" | "fill" | "fill_hex" | "fillHex"
    )
}
