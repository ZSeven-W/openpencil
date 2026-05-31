//! Single-line TS batch_design operations that map to existing editor commands.

use op_editor_core::{EditorCommand, NodeId};

use super::batch_design::{find_top_level_char, normalize_node_shape};

pub(crate) fn parse_single_direct_operation(line: &str) -> Result<Option<EditorCommand>, String> {
    let line = line.trim().trim_end_matches(';').trim();
    for op in ["U", "D", "M"] {
        let prefix = format!("{op}(");
        if line.starts_with(&prefix) && line.ends_with(')') {
            let body = &line[prefix.len()..line.len() - 1];
            return match op {
                "U" => parse_update_operation(body).map(Some),
                "D" => Ok(Some(EditorCommand::DeleteNode {
                    node_id: NodeId::new(&parse_ref_token(body)?),
                })),
                "M" => parse_move_operation(body).map(Some),
                _ => unreachable!(),
            };
        }
    }
    Ok(None)
}

fn parse_update_operation(body: &str) -> Result<EditorCommand, String> {
    let Some(comma) = find_top_level_char(body, ',') else {
        return Err("U() requires node id and update JSON".into());
    };
    let node_id = NodeId::new(&parse_ref_token(body[..comma].trim())?);
    let mut value: serde_json::Value = serde_json::from_str(body[comma + 1..].trim())
        .map_err(|e| format!("invalid U JSON: {e}"))?;
    normalize_node_shape(&mut value);
    let Some(obj) = value.as_object() else {
        return Err("U() update JSON must be an object".into());
    };
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
    if find_top_level_char(rest, ',').is_some() {
        return Err("M() index argument is not supported by the Rust MoveNode command".into());
    }
    Ok(EditorCommand::MoveNode {
        node_id: NodeId::new(&parse_ref_token(body[..comma].trim())?),
        target_parent: parse_parent_node_id(rest)?,
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
