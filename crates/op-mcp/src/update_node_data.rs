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
    ts_update_patch_json_value(&value)
        .map_err(|error| ToolOutcome::Err(ToolErrorCode::InvalidArgument, error.to_string()))
}

/// A rich-patch normalization failure. Both variants are malformed-`fill_hex`
/// reports; `Display` reproduces the previous message byte-for-byte (the
/// `U()` program path re-labels them as `ProgramError::Json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UpdateDataError {
    /// `fill_hex` / `fillHex` is present but is not a JSON string.
    FillHexNotString,
    /// `fill_hex` / `fillHex` is a string but not a supported hex spelling.
    InvalidFillHex(String),
}

impl std::fmt::Display for UpdateDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateDataError::FillHexNotString => f.write_str("fill_hex must be a string"),
            UpdateDataError::InvalidFillHex(hex) => {
                write!(f, "fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {hex:?}")
            }
        }
    }
}

impl std::error::Error for UpdateDataError {}

pub(super) fn ts_update_patch_json_value(value: &Value) -> Result<Option<String>, UpdateDataError> {
    let Some(obj) = value.as_object() else {
        return Ok(None);
    };
    let is_rich = obj.is_empty()
        || obj.keys().any(|key| !is_flat_update_data_key(key))
        || uses_sizing_keyword(obj)
        || uses_fractional_geometry(obj)
        || fill_requires_rich_patch(obj);
    if !is_rich {
        return Ok(None);
    }

    let mut patch = value.clone();
    canonicalize_fill_hex_shortcut(&mut patch)?;
    // Rich `fill` values stay on the canonical patch path. Normalize legacy
    // string/object spellings before `PatchNodeData` deserializes the node.
    super::batch_design::normalize_node_shape(&mut patch);
    Ok(Some(patch.to_string()))
}

/// `UpdateNode` stores width and height as integers, while canonical node data
/// also accepts content/fill sizing keywords. Keep literal numeric geometry on
/// the lightweight command, but route keyword sizing through the rich shallow
/// patch so serde can preserve its `SizingBehavior` variant.
fn uses_sizing_keyword(obj: &serde_json::Map<String, Value>) -> bool {
    ["width", "height"].into_iter().any(|key| {
        matches!(
            obj.get(key),
            Some(Value::String(value))
                if matches!(value.as_str(), "fit_content" | "fill_container")
        )
    })
}

/// `UpdateNode` also stores x and y as integers, so it rejects a fractional
/// number for any of the four geometry fields. Canonical node data stores
/// them as floats; route such values through the rich shallow patch.
fn uses_fractional_geometry(obj: &serde_json::Map<String, Value>) -> bool {
    ["x", "y", "width", "height"]
        .into_iter()
        .any(|key| matches!(obj.get(key), Some(Value::Number(value)) if value.is_f64()))
}

/// The lightweight command can faithfully carry one concrete solid color.
/// Tokens, gradients, multiple layers, opacity, and other canonical fill
/// structure must stay on `PatchNodeData` or they are either rejected as a
/// non-hex color or silently flattened.
fn fill_requires_rich_patch(obj: &serde_json::Map<String, Value>) -> bool {
    let Some(fill) = obj.get("fill") else {
        return false;
    };
    match fill {
        Value::String(color) => !validate_hex(color),
        Value::Object(layer) => !is_plain_solid_hex_layer(layer),
        Value::Array(layers) => {
            layers.len() != 1
                || layers
                    .first()
                    .and_then(Value::as_object)
                    .is_none_or(|layer| !is_plain_solid_hex_layer(layer))
        }
        _ => true,
    }
}

fn is_plain_solid_hex_layer(layer: &serde_json::Map<String, Value>) -> bool {
    layer
        .get("color")
        .and_then(Value::as_str)
        .is_some_and(validate_hex)
        && layer
            .get("type")
            .and_then(Value::as_str)
            .is_none_or(|kind| kind.eq_ignore_ascii_case("solid"))
        && layer
            .keys()
            .all(|key| matches!(key.as_str(), "type" | "color"))
}

/// `fill_hex` / `fillHex` belong to the lightweight update tool, not the
/// canonical PenNode schema. Once any rich field moves the update onto
/// `PatchNodeData`, rewrite the shortcut to a canonical solid fill so it does
/// not disappear during PenNode deserialization.
fn canonicalize_fill_hex_shortcut(value: &mut Value) -> Result<(), UpdateDataError> {
    let Some(obj) = value.as_object_mut() else {
        return Ok(());
    };
    let shortcut = obj.remove("fill_hex");
    let camel_shortcut = obj.remove("fillHex");
    let Some(shortcut) = shortcut.or(camel_shortcut) else {
        return Ok(());
    };
    let Some(hex) = shortcut.as_str() else {
        return Err(UpdateDataError::FillHexNotString);
    };
    if !validate_hex(hex) {
        return Err(UpdateDataError::InvalidFillHex(hex.to_string()));
    }
    obj.insert(
        "fill".into(),
        serde_json::json!([{ "type": "solid", "color": hex }]),
    );
    Ok(())
}

fn validate_hex(value: &str) -> bool {
    let Some(hex) = value.trim().strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_flat_update_data_key(key: &str) -> bool {
    // `fill_requires_rich_patch` separately keeps gradients and `$--token`
    // references off this lightweight path while preserving the established
    // fast path for one concrete solid hex fill.
    matches!(
        key,
        "name" | "x" | "y" | "width" | "height" | "fill" | "fill_hex" | "fillHex"
    )
}
