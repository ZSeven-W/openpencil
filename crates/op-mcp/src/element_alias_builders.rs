//! Semantic builders for high-frequency TS `pen-mcp` element aliases.
//!
//! The full TS catalog has many specialized templates. This module
//! ports the common atomic builders first so Rust MCP aliases do more
//! than instantiate generic starter-kit placeholders.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

use crate::batch_design::normalize_node_shape;
use crate::{ToolErrorCode, ToolOutcome};

static NEXT_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn semantic_alias_node(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<PenNode>, ToolOutcome> {
    let value = match tool {
        "add_heading_v0" | "add_heading_v1" => build_heading(args)?,
        "add_body_text_v0" | "add_body_text_v1" => build_body_text(args)?,
        "add_text_button_v0" | "add_text_button_v1" => build_text_button(args)?,
        "add_form_field_v0" | "add_form_field_v1" => build_form_field(args)?,
        "add_section_header_v0" | "add_section_header_v1" => build_section_header(args)?,
        _ => return Ok(None),
    };
    pen_node_from_value(value).map(Some)
}

fn build_heading(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let content = required(args, "content")?;
    let level = args.get("level").map(String::as_str).unwrap_or("h2");
    let (font_size, font_weight, line_height, letter_spacing) = match (detect_cjk(content), level) {
        (Some(_), "display") => (48, 700, 1.3, None),
        (Some(_), "h1") => (32, 700, 1.3, None),
        (Some(_), "h2") => (24, 600, 1.35, None),
        (Some(_), "h3") => (20, 600, 1.4, None),
        (None, "display") => (48, 700, 1.0, Some(-0.5)),
        (None, "h1") => (32, 700, 1.1, None),
        (None, "h2") => (24, 600, 1.2, None),
        (None, "h3") => (20, 600, 1.25, None),
        _ => {
            return Err(ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("add_heading_v0: invalid level {level:?}; expected display, h1, h2, h3"),
            ));
        }
    };
    let mut value = json!({
        "id": next_id("heading"),
        "type": "text",
        "name": format!("Heading ({level})"),
        "role": "heading",
        "content": content,
        "fontSize": font_size,
        "fontWeight": font_weight,
        "lineHeight": line_height,
    });
    if let Some(spacing) = letter_spacing {
        value["letterSpacing"] = json!(spacing);
    }
    if let Some(script) = detect_cjk(content) {
        value["fontFamily"] = json!(cjk_font_family(script));
        value["letterSpacing"] = json!(0);
    }
    Ok(value)
}

fn build_body_text(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let content = required(args, "content")?;
    let is_cjk = detect_cjk(content).is_some();
    let mut value = json!({
        "id": next_id("body"),
        "type": "text",
        "name": "Body",
        "role": "body",
        "content": content,
        "fontSize": 16,
        "fontWeight": 400,
        "fontFamily": "Inter",
        "lineHeight": if is_cjk { 1.6 } else { 1.5 },
        "width": "fill_container",
        "textGrowth": "fixed-width",
    });
    if is_cjk {
        value["letterSpacing"] = json!(0);
    }
    Ok(value)
}

fn build_text_button(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let mut children = Vec::new();
    if let Some(icon) = args.get("leading_icon").or_else(|| args.get("leadingIcon")) {
        children.push(icon_node("Icon", icon, 16, 16));
    }
    children.push(json!({
        "id": next_id("button_label"),
        "type": "text",
        "name": "Label",
        "role": "label",
        "content": label,
        "fontSize": 14,
        "fontWeight": 500,
    }));
    Ok(json!({
        "id": next_id("button"),
        "type": "frame",
        "name": "Text Button",
        "role": "button",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "gap": 8,
        "padding": [12, 20],
        "cornerRadius": 8,
        "children": children,
    }))
}

fn build_form_field(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let label_text = if bool_arg(args, "required") {
        format!("{label} *")
    } else {
        label.to_string()
    };
    let mut input_children = Vec::new();
    if let Some(icon) = args.get("leading_icon").or_else(|| args.get("leadingIcon")) {
        input_children.push(icon_node("Leading Icon", icon, 20, 20));
    }
    input_children.push(json!({
        "id": next_id("placeholder"),
        "type": "text",
        "name": "Placeholder",
        "content": args.get("placeholder").map(String::as_str).unwrap_or(""),
        "fontSize": 14,
        "fontWeight": 400,
    }));
    if let Some(icon) = args
        .get("trailing_icon")
        .or_else(|| args.get("trailingIcon"))
    {
        input_children.push(icon_node("Trailing Icon", icon, 20, 20));
    }
    Ok(json!({
        "id": next_id("field"),
        "type": "frame",
        "name": "Form Field",
        "role": "form-field",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 6,
        "children": [
            {
                "id": next_id("field_label"),
                "type": "text",
                "name": "Label",
                "role": "label",
                "content": label_text,
                "fontSize": 14,
                "fontWeight": 500,
            },
            {
                "id": next_id("input"),
                "type": "frame",
                "name": "Input",
                "role": "form-input",
                "width": "fill_container",
                "height": 48,
                "cornerRadius": 8,
                "layout": "horizontal",
                "alignItems": "center",
                "gap": 8,
                "padding": [12, 16],
                "children": input_children,
            }
        ],
    }))
}

fn build_section_header(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let title = required(args, "title")?;
    let mut children = vec![json!({
        "id": next_id("section_header_title_container"),
        "type": "frame",
        "name": "Title Container",
        "role": "section-header-title",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "children": [{
            "id": next_id("section_header_title"),
            "type": "text",
            "name": "Title",
            "role": "heading",
            "content": title,
            "fontSize": 20,
            "fontWeight": 700,
            "width": "fill_container",
            "textGrowth": "fixed-width",
        }],
    })];
    if let Some(action) = parse_action(args)? {
        children.push(action);
    }
    Ok(json!({
        "id": next_id("section_header"),
        "type": "frame",
        "name": "Section Header",
        "role": "section-header",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 16,
        "children": children,
    }))
}

fn parse_action(args: &BTreeMap<String, String>) -> Result<Option<Value>, ToolOutcome> {
    let Some(raw) = args.get("action") else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("action must be a JSON object: {e}"),
        )
    })?;
    let label = value.get("label").and_then(Value::as_str).ok_or_else(|| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "action.label is required".into(),
        )
    })?;
    let mut children = vec![json!({
        "id": next_id("section_header_action_label"),
        "type": "text",
        "name": "Action Label",
        "role": "label",
        "content": label,
        "fontSize": 14,
        "fontWeight": 500,
    })];
    if let Some(icon) = value.get("icon").and_then(Value::as_str) {
        children.push(icon_node("Action Icon", icon, 16, 16));
    }
    Ok(Some(json!({
        "id": next_id("section_header_action"),
        "type": "frame",
        "name": "Action",
        "role": "section-header-action",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 4,
        "children": children,
    })))
}

fn icon_node(name: &str, icon: &str, width: i32, height: i32) -> Value {
    json!({
        "id": next_id("icon"),
        "type": "icon_font",
        "name": name,
        "iconFontName": icon,
        "iconFontFamily": "lucide",
        "width": width,
        "height": height,
    })
}

fn pen_node_from_value(mut value: Value) -> Result<PenNode, ToolOutcome> {
    normalize_node_shape(&mut value);
    serde_json::from_value(value).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("semantic element payload is not a valid PenNode: {e}"),
        )
    })
}

fn required<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, ToolOutcome> {
    args.get(key)
        .map(String::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            ToolOutcome::Err(ToolErrorCode::MissingArgument, format!("{key} is required"))
        })
}

fn bool_arg(args: &BTreeMap<String, String>, key: &str) -> bool {
    matches!(args.get(key).map(String::as_str), Some("true" | "1"))
}

fn next_id(prefix: &str) -> String {
    let n = NEXT_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_{prefix}_{n}")
}

#[derive(Clone, Copy)]
enum CjkScript {
    Chinese,
    Japanese,
    Korean,
}

fn detect_cjk(s: &str) -> Option<CjkScript> {
    if s.chars().any(|c| ('\u{3040}'..='\u{30ff}').contains(&c)) {
        return Some(CjkScript::Japanese);
    }
    if s.chars().any(|c| ('\u{ac00}'..='\u{d7af}').contains(&c)) {
        return Some(CjkScript::Korean);
    }
    if s.chars()
        .any(|c| ('\u{3000}'..='\u{303f}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c))
    {
        return Some(CjkScript::Chinese);
    }
    None
}

fn cjk_font_family(script: CjkScript) -> &'static str {
    match script {
        CjkScript::Chinese => "Noto Sans SC",
        CjkScript::Japanese => "Noto Sans JP",
        CjkScript::Korean => "Noto Sans KR",
    }
}
