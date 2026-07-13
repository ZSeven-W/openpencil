//! Single-line TS batch_design operations that map to existing editor commands.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};
use std::collections::BTreeMap;

use super::batch_design::{find_top_level_char, normalize_node_shape};
use super::insert_node_args::{insert_node_params, InsertNodeParams};
use super::update_node_data::ts_update_patch_json_value;
use super::ToolOutcome;

pub(crate) fn parse_single_direct_operation(line: &str) -> Result<Option<EditorCommand>, String> {
    let line = line.trim().trim_end_matches(';').trim();
    let line = strip_bound_direct_operation(line);
    for op in ["U", "D", "M", "C", "R", "G"] {
        let prefix = format!("{op}(");
        if line.starts_with(&prefix) && line.ends_with(')') {
            let body = &line[prefix.len()..line.len() - 1];
            return match op {
                "U" => parse_update_operation(body).map(Some),
                "D" => Ok(Some(EditorCommand::DeleteNode {
                    node_id: NodeId::new(&parse_ref_token(body)?),
                    page_id: None,
                })),
                "M" => parse_move_operation(body).map(Some),
                "C" => parse_copy_operation(body).map(Some),
                "R" => parse_replace_operation(body).map(Some),
                "G" => parse_image_operation(body).map(Some),
                _ => unreachable!(),
            };
        }
    }
    Ok(None)
}

/// Whether a legacy single-line operations payload is a `G()` call, with or
/// without a result binding. Phase tools use this before the flat direct
/// parser because they do not hold the document snapshot needed to enforce
/// image placement and target geometry safely.
pub(crate) fn is_direct_image_operation(line: &str) -> bool {
    let line = line.trim().trim_end_matches(';').trim();
    let operation = find_top_level_char(line, '=')
        .map(|eq| line[eq + 1..].trim())
        .unwrap_or(line);
    operation.starts_with("G(")
}

fn strip_bound_direct_operation(line: &str) -> &str {
    let Some(eq) = find_top_level_char(line, '=') else {
        return line;
    };
    let rhs = line[eq + 1..].trim();
    if ["C(", "R(", "M(", "G("]
        .iter()
        .any(|prefix| rhs.starts_with(prefix))
        && rhs.ends_with(')')
    {
        rhs
    } else {
        line
    }
}

fn parse_update_operation(body: &str) -> Result<EditorCommand, String> {
    let Some(comma) = find_top_level_char(body, ',') else {
        return Err("U() requires node id and update JSON".into());
    };
    let node_id = NodeId::new(&parse_ref_token(body[..comma].trim())?);
    let mut value: serde_json::Value = serde_json::from_str(body[comma + 1..].trim())
        .map_err(|e| format!("invalid U JSON: {e}"))?;
    normalize_node_shape(&mut value);
    update_command_from_value(node_id, &value)
}

/// Build the U() editor command from an already-parsed + normalized
/// update JSON value. Shared by the single-line direct-op parser above
/// and the multi-op DSL program executor (`batch_program.rs`), so both
/// paths route rich patches to `PatchNodeData` and flat geometry
/// patches to `UpdateNode` identically.
pub(crate) fn update_command_from_value(
    node_id: NodeId,
    value: &serde_json::Value,
) -> Result<EditorCommand, String> {
    let Some(obj) = value.as_object() else {
        return Err("U() update JSON must be an object".into());
    };
    if let Some(patch_json) = ts_update_patch_json_value(value) {
        return Ok(EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id: None,
        });
    }
    let x = parse_i32_json(obj, "x")?;
    let y = parse_i32_json(obj, "y")?;
    let width = parse_i32_json(obj, "width")?;
    let height = parse_i32_json(obj, "height")?;
    let name = obj.get("name").and_then(|v| v.as_str()).map(str::to_string);
    let fill_hex = obj
        .get("fill_hex")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| obj.get("fill").and_then(fill_hex_from_value));
    if x.is_none()
        && y.is_none()
        && width.is_none()
        && height.is_none()
        && name.is_none()
        && fill_hex.is_none()
    {
        return Err("U() must set at least one supported field".into());
    }
    if let Some(hex) = &fill_hex {
        if !validate_hex(hex) {
            return Err(format!(
                "fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {hex:?}"
            ));
        }
    }
    Ok(EditorCommand::UpdateNode {
        node_id,
        x,
        y,
        width,
        height,
        name,
        fill_hex,
        page_id: None,
    })
}

fn parse_move_operation(body: &str) -> Result<EditorCommand, String> {
    let Some(comma) = find_top_level_char(body, ',') else {
        return Err("M() requires node id and parent id".into());
    };
    let rest = body[comma + 1..].trim();
    let (parent_raw, index) = if let Some(index_comma) = find_top_level_char(rest, ',') {
        let raw_index = rest[index_comma + 1..]
            .trim()
            .trim_matches(|c| c == '"' || c == '\'');
        let index = raw_index
            .parse::<usize>()
            .map_err(|_| format!("M() index must be a non-negative integer, got {raw_index:?}"))?;
        (rest[..index_comma].trim(), Some(index))
    } else {
        (rest, None)
    };
    Ok(EditorCommand::MoveNode {
        node_id: NodeId::new(&parse_ref_token(body[..comma].trim())?),
        target_parent: parse_parent_node_id(parent_raw)?,
        page_id: None,
        index,
    })
}

fn parse_copy_operation(body: &str) -> Result<EditorCommand, String> {
    let Some(comma) = find_top_level_char(body, ',') else {
        return Err("C() requires source id and parent id".into());
    };
    let rest = body[comma + 1..].trim();
    let (parent_raw, overrides_json) = if let Some(overrides_comma) = find_top_level_char(rest, ',')
    {
        let mut value: serde_json::Value = serde_json::from_str(rest[overrides_comma + 1..].trim())
            .map_err(|e| format!("invalid C overrides JSON: {e}"))?;
        normalize_node_shape(&mut value);
        if !value.is_object() {
            return Err("C() overrides JSON must be an object".into());
        }
        (rest[..overrides_comma].trim(), Some(value.to_string()))
    } else {
        (rest, None)
    };
    Ok(EditorCommand::CopyNode {
        node_id: NodeId::new(&parse_ref_token(body[..comma].trim())?),
        target_parent: parse_parent_node_id(parent_raw)?,
        overrides_json,
        page_id: None,
    })
}

fn parse_replace_operation(body: &str) -> Result<EditorCommand, String> {
    let Some(comma) = find_top_level_char(body, ',') else {
        return Err("R() requires node id and node JSON".into());
    };
    let node_id = NodeId::new(&parse_ref_token(body[..comma].trim())?);
    let raw_data = body[comma + 1..].trim();
    let mut args = BTreeMap::new();
    args.insert("data".into(), raw_data.to_string());
    let params = insert_node_params(&args).map_err(tool_outcome_message)?;
    let InsertNodeParams {
        kind,
        name,
        x,
        y,
        width,
        height,
        fill_hex,
    } = params;
    Ok(EditorCommand::ReplaceNode {
        node_id,
        kind,
        name,
        x,
        y,
        width,
        height,
        fill_hex,
        drop_children: false,
        page_id: None,
    })
}

fn parse_image_operation(body: &str) -> Result<EditorCommand, String> {
    let parts = split_top_level_args(body);
    if !matches!(parts.len(), 3 | 4) {
        return Err("G() requires parent id, mode, prompt, and optional placement".into());
    }
    let parent_id = parse_parent_node_id(parts[0])?;
    let mode = parse_ref_token(parts[1])?;
    if !matches!(mode.as_str(), "search" | "generate") {
        return Err(format!(
            "G() mode must be \"search\" or \"generate\", got {mode:?}"
        ));
    }
    let prompt = parse_ref_token(parts[2])?;
    if let Some(raw) = parts.get(3) {
        let placement = parse_ref_token(raw)?;
        if !matches!(placement.as_str(), "slot" | "append") {
            return Err(format!(
                "G() placement must be \"slot\" or \"append\", got {placement:?}"
            ));
        }
    }
    let name = prompt.chars().take(40).collect::<String>();
    let (width, height) = if parent_id.is_real() {
        (
            serde_json::json!("fill_container"),
            serde_json::json!("fill_container"),
        )
    } else {
        (serde_json::json!(400), serde_json::json!(300))
    };
    let mut value = serde_json::json!({
        "type": "image",
        "id": "__op_tmp_image_1",
        "name": name,
        "src": "",
        "objectFit": "crop",
        "width": width,
        "height": height
    });
    if mode == "generate" {
        value["imagePrompt"] = serde_json::json!(prompt);
    } else {
        value["imageSearchQuery"] = serde_json::json!(prompt);
    }
    let node: PenNode =
        serde_json::from_value(value).map_err(|e| format!("invalid G() image node: {e}"))?;
    Ok(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id,
        page_id: None,
    })
}

fn parse_parent_node_id(raw: &str) -> Result<NodeId, String> {
    let raw = raw.trim();
    if matches!(raw, "null" | "undefined" | "\"\"" | "''" | "0" | "\"0\"") {
        return Ok(NodeId::NONE);
    }
    Ok(NodeId::new(&parse_ref_token(raw)?))
}

fn parse_ref_token(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.starts_with('"') {
        return serde_json::from_str::<String>(raw).map_err(|e| format!("invalid string ref: {e}"));
    }
    if raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2 {
        return Ok(raw[1..raw.len() - 1].to_string());
    }
    Ok(raw.to_string())
}

pub(crate) fn split_top_level_args(mut raw: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    loop {
        let trimmed = raw.trim();
        if let Some(comma) = find_top_level_char(trimmed, ',') {
            parts.push(trimmed[..comma].trim());
            raw = &trimmed[comma + 1..];
        } else {
            if !trimmed.is_empty() {
                parts.push(trimmed);
            }
            return parts;
        }
    }
}

fn parse_i32_json(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Option<i32>, String> {
    let Some(value) = obj.get(key) else {
        return Ok(None);
    };
    let Some(raw) = value.as_i64() else {
        return Err(format!("{key} must be an integer"));
    };
    i32::try_from(raw)
        .map(Some)
        .map_err(|_| format!("{key} is outside i32 range"))
}

fn fill_hex_from_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(obj) => obj.get("color")?.as_str().map(str::to_string),
        serde_json::Value::Array(items) => items.first().and_then(fill_hex_from_value),
        _ => None,
    }
}

fn validate_hex(s: &str) -> bool {
    let Some(rest) = s.trim().strip_prefix('#') else {
        return false;
    };
    matches!(rest.len(), 3 | 6 | 8) && rest.chars().all(|c| c.is_ascii_hexdigit())
}

fn tool_outcome_message(outcome: ToolOutcome) -> String {
    match outcome {
        ToolOutcome::Err(_, message) => message,
        _ => "invalid R() node JSON".into(),
    }
}
