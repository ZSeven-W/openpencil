//! Lightweight TS element-tool schema extraction.
//!
//! Rust MCP keeps TS `add_*_v0/v1` aliases visible. Embedding the TS
//! definition shards lets `tools/list` advertise the same business
//! arguments instead of falling back to generic x/y placement args.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde_json::{json, Map, Value};

const TS_ELEMENT_DEFINITION_SOURCES: &[&str] = &[
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-base.ts"),
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-ext.ts"),
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-ext-2.ts"),
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-ext-3.ts"),
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-ext-4.ts"),
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-ext-5.ts"),
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-ext-6.ts"),
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-ext-7.ts"),
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-ext-8.ts"),
    include_str!("../../../packages/pen-mcp/src/routes/element-tool-defs-ext-9.ts"),
];

pub(crate) fn ts_alias_schema(tool: &str) -> Option<String> {
    ts_alias_schema_map().get(tool).cloned()
}

fn ts_alias_schema_map() -> &'static BTreeMap<String, String> {
    static MAP: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    MAP.get_or_init(build_ts_alias_schema_map)
}

fn build_ts_alias_schema_map() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for source in TS_ELEMENT_DEFINITION_SOURCES {
        parse_ts_alias_schemas(source, &mut out);
    }
    out
}

fn parse_ts_alias_schemas(source: &str, out: &mut BTreeMap<String, String>) {
    let needle = "name: '";
    let mut offset = 0;
    while let Some(rel) = source[offset..].find(needle) {
        let name_start = offset + rel + needle.len();
        let Some(name_end_rel) = source[name_start..].find('\'') else {
            break;
        };
        let name_end = name_start + name_end_rel;
        let name = &source[name_start..name_end];
        offset = name_end + 1;
        if !name.starts_with("add_") {
            continue;
        }
        let Some(object_start) = source[..offset].rfind('{') else {
            continue;
        };
        let Some(object_end) = find_matching(source, object_start, '{', '}') else {
            continue;
        };
        if let Some(schema) = schema_for_tool_object(name, &source[object_start..=object_end]) {
            out.insert(name.to_string(), schema);
        }
    }
}

fn schema_for_tool_object(name: &str, object_src: &str) -> Option<String> {
    let tool_body = object_body(object_src)?;
    let fields = object_fields(tool_body);
    let description = fields
        .iter()
        .find(|(key, _)| key == "description")
        .and_then(|(_, value)| ts_string_expr(value))
        .unwrap_or_else(|| format!("TS pen-mcp compatible element alias {name}."));
    let input_schema_src = fields
        .iter()
        .find(|(key, _)| key == "inputSchema")
        .map(|(_, value)| value.as_str())?;
    let input_schema_body = object_body(input_schema_src)?;
    let input_fields = object_fields(input_schema_body);
    let properties = input_fields
        .iter()
        .find(|(key, _)| key == "properties")
        .map(|(_, value)| properties_object(value))
        .unwrap_or_else(|| Value::Object(Map::new()));
    let required = input_fields
        .iter()
        .find(|(key, _)| key == "required")
        .map(|(_, value)| string_array(value))
        .unwrap_or_default();

    let schema = json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        },
    });
    serde_json::to_string(&schema).ok()
}

fn properties_object(value_src: &str) -> Value {
    let Some(body) = object_body(value_src) else {
        return Value::Object(Map::new());
    };
    let mut properties = Map::new();
    for (key, value) in object_fields(body) {
        properties.insert(key, schema_value(&value));
    }
    Value::Object(properties)
}

fn schema_value(value_src: &str) -> Value {
    let value = strip_ts_assertions(value_src.trim());
    match value {
        "schemaVersionProp" => json!({
            "type": "string",
            "enum": ["1.0"],
            "description": "Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit. Breaking schema changes ship as a new tool with _v1 suffix; old tools are kept one stage before being removed from ListTools.",
        }),
        "filePathProp" => json!({
            "type": "string",
            "description": "Path to .op file, or omit for live canvas",
        }),
        "parentIdProp" => json!({
            "type": "string",
            "description": "Target parent node id (must exist in the document). Omit for root-level insertion.",
        }),
        "pageIdProp" => json!({
            "type": "string",
            "description": "Target page ID (defaults to first page)",
        }),
        _ => schema_object_value(value),
    }
}

fn schema_object_value(value_src: &str) -> Value {
    let Some(body) = object_body(value_src) else {
        return json!({
            "type": "string",
            "description": "TS pen-mcp compatible argument.",
        });
    };
    let mut out = Map::new();
    for (key, value) in object_fields(body) {
        match key.as_str() {
            "type" => {
                let trimmed = strip_ts_assertions(value.trim());
                let type_value = if trimmed.starts_with('[') {
                    Value::Array(
                        string_array(trimmed)
                            .into_iter()
                            .map(Value::String)
                            .collect(),
                    )
                } else {
                    ts_string_expr(trimmed)
                        .map(Value::String)
                        .unwrap_or_else(|| Value::String("string".to_string()))
                };
                out.insert("type".to_string(), type_value);
            }
            "enum" => {
                out.insert(
                    "enum".to_string(),
                    Value::Array(
                        string_array(&value)
                            .into_iter()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
            "description" => {
                if let Some(description) = ts_string_expr(&value) {
                    out.insert("description".to_string(), Value::String(description));
                }
            }
            "items" => {
                out.insert("items".to_string(), schema_value(&value));
            }
            "properties" => {
                out.insert("properties".to_string(), properties_object(&value));
            }
            "required" => {
                out.insert(
                    "required".to_string(),
                    Value::Array(
                        string_array(&value)
                            .into_iter()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
            "oneOf" => {
                out.insert("oneOf".to_string(), schema_array(&value));
            }
            _ => {}
        }
    }
    if out.is_empty() {
        json!({
            "type": "string",
            "description": "TS pen-mcp compatible argument.",
        })
    } else {
        Value::Object(out)
    }
}

fn schema_array(value_src: &str) -> Value {
    let trimmed = value_src.trim();
    if !trimmed.starts_with('[') {
        return Value::Array(Vec::new());
    }
    let Some(end) = find_matching(trimmed, 0, '[', ']') else {
        return Value::Array(Vec::new());
    };
    let inner = &trimmed[1..end];
    Value::Array(
        split_top_level_entries(inner)
            .into_iter()
            .map(|entry| schema_value(&entry))
            .collect(),
    )
}

fn object_fields(body: &str) -> Vec<(String, String)> {
    split_top_level_entries(body)
        .into_iter()
        .filter_map(|entry| {
            let colon = find_top_level_char(&entry, ':')?;
            let key = parse_key(entry[..colon].trim())?;
            let value = entry[colon + 1..].trim().to_string();
            Some((key, value))
        })
        .collect()
}

fn split_top_level_entries(source: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (idx, ch) in source.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                let entry = source[start..idx].trim();
                if !entry.is_empty() {
                    entries.push(entry.to_string());
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    let entry = source[start..].trim();
    if !entry.is_empty() {
        entries.push(entry.to_string());
    }
    entries
}

fn find_top_level_char(source: &str, target: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (idx, ch) in source.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            _ if ch == target && depth == 0 => return Some(idx),
            _ => {}
        }
    }
    None
}

fn object_body(source: &str) -> Option<&str> {
    let trimmed = source.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let end = find_matching(trimmed, 0, '{', '}')?;
    Some(&trimmed[1..end])
}

fn find_matching(source: &str, open_idx: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (idx, ch) in source[open_idx..].char_indices() {
        let abs = open_idx + idx;
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            _ if ch == open => depth += 1,
            _ if ch == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(abs);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_key(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.starts_with('\'') || raw.starts_with('"') {
        return ts_string_expr(raw);
    }
    let key: String = raw
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '$')
        .collect();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

fn strip_ts_assertions(value: &str) -> &str {
    value
        .trim()
        .strip_suffix(" as const")
        .unwrap_or(value)
        .trim()
}

fn string_array(value_src: &str) -> Vec<String> {
    string_literals(value_src)
}

fn ts_string_expr(value_src: &str) -> Option<String> {
    let parts = string_literals(value_src);
    if parts.is_empty() {
        None
    } else {
        Some(parts.concat())
    }
}

fn string_literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = source.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch != '\'' && ch != '"' && ch != '`' {
            continue;
        }
        let quote = ch;
        let mut value = String::new();
        let mut escaped = false;
        for (_, c) in chars.by_ref() {
            if escaped {
                value.push(match c {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    other => other,
                });
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == quote {
                out.push(value);
                break;
            } else {
                value.push(c);
            }
        }
        if source[start..].len() == 1 {
            break;
        }
    }
    out
}
