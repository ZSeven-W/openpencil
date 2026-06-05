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
use crate::element_basic_alias_builders::basic_alias_node_value;
use crate::element_calendar_alias_builders::calendar_alias_node_value;
use crate::element_complex_alias_builders::complex_alias_node_value;
use crate::element_content_alias_builders::content_alias_node_value;
use crate::element_feedback_alias_builders::feedback_alias_node_value;
use crate::element_flow_alias_builders::flow_alias_node_value;
use crate::element_input_alias_builders::input_alias_node_value;
use crate::element_misc_alias_builders::misc_alias_node_value;
use crate::element_ported_cards_a::ported_cards_a_alias_node_value;
use crate::element_ported_cards_b::ported_cards_b_alias_node_value;
use crate::element_ported_inputs::ported_inputs_alias_node_value;
use crate::element_ported_nav::ported_nav_alias_node_value;
use crate::element_ported_rows_a::ported_rows_a_alias_node_value;
use crate::element_ported_rows_b::ported_rows_b_alias_node_value;
use crate::element_ported_shells::ported_shells_alias_node_value;
use crate::element_visual_alias_builders::visual_alias_node_value;
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
        "add_checkbox_v0" => build_checkbox(args, false)?,
        "add_checkbox_v1" => build_checkbox(args, true)?,
        "add_radio_v0" => build_radio(args, false)?,
        "add_radio_v1" => build_radio(args, true)?,
        "add_segmented_control_v0" => build_segmented_control(args, false)?,
        "add_segmented_control_v1" => build_segmented_control(args, true)?,
        "add_switch_v0" | "add_switch_v1" => build_switch(args)?,
        "add_form_field_v0" | "add_form_field_v1" => build_form_field(args)?,
        "add_section_header_v0" | "add_section_header_v1" => build_section_header(args)?,
        _ => {
            if let Some(value) = basic_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = feedback_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = visual_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = calendar_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = content_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = flow_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = input_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = misc_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = complex_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = ported_rows_a_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = ported_rows_b_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = ported_inputs_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = ported_cards_a_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = ported_cards_b_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = ported_nav_alias_node_value(tool, args)? {
                value
            } else if let Some(value) = ported_shells_alias_node_value(tool, args)? {
                value
            } else {
                return Ok(None);
            }
        }
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

fn build_checkbox(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let checked = bool_arg(args, "checked");
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let (accent_color, border_color) = if theme_aware {
        match theme {
            "dark" => ("#60A5FA", "#334155"),
            "system" => ("$color-accent", "$color-border"),
            _ => ("#2563EB", "#9CA3AF"),
        }
    } else {
        ("#2563EB", "#9CA3AF")
    };

    let mut box_node = json!({
        "id": next_id("checkbox_box"),
        "type": "frame",
        "name": if checked { "Checkbox (checked)" } else { "Checkbox" },
        "role": if checked { "checkbox-checked" } else { "checkbox" },
        "width": 20,
        "height": 20,
        "cornerRadius": 4,
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "children": [],
    });
    if checked {
        box_node["fill"] = json!([{ "type": "solid", "color": accent_color }]);
        box_node["children"] = json!([{
            "id": next_id("checkbox_check"),
            "type": "icon_font",
            "name": "Check",
            "iconFontName": "check",
            "iconFontFamily": "lucide",
            "width": 14,
            "height": 14,
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        }]);
    } else {
        box_node["fill"] = json!([]);
        box_node["stroke"] = json!({
            "thickness": 1.5,
            "fill": [{ "type": "solid", "color": border_color }],
        });
    }

    Ok(json!({
        "id": next_id("checkbox_row"),
        "type": "frame",
        "name": format!("Checkbox Row ({label})"),
        "role": "checkbox-row",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 8,
        "children": [
            box_node,
            {
                "id": next_id("checkbox_label"),
                "type": "text",
                "name": "Label",
                "role": "label",
                "content": label,
                "fontSize": 14,
                "fontWeight": 400,
            }
        ],
    }))
}

fn build_radio(args: &BTreeMap<String, String>, theme_aware: bool) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let selected = bool_arg(args, "selected");
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let ring_color = if selected {
        "#2563EB"
    } else if theme_aware {
        match theme {
            "dark" => "#334155",
            "system" => "$color-border",
            _ => "#9CA3AF",
        }
    } else {
        "#9CA3AF"
    };

    let mut outer = json!({
        "id": next_id("radio_outer"),
        "type": "frame",
        "name": if selected { "Radio (selected)" } else { "Radio" },
        "role": if selected { "radio-selected" } else { "radio" },
        "width": 20,
        "height": 20,
        "cornerRadius": 10,
        "fill": [],
        "stroke": {
            "thickness": 1.5,
            "fill": [{ "type": "solid", "color": ring_color }],
        },
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "children": [],
    });
    if selected {
        outer["children"] = json!([{
            "id": next_id("radio_dot"),
            "type": "frame",
            "name": "Dot",
            "role": "radio-dot",
            "width": 10,
            "height": 10,
            "cornerRadius": 5,
            "fill": [{ "type": "solid", "color": "#2563EB" }],
        }]);
    }

    Ok(json!({
        "id": next_id("radio_row"),
        "type": "frame",
        "name": format!("Radio Row ({label})"),
        "role": "radio-row",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 8,
        "children": [
            outer,
            {
                "id": next_id("radio_label"),
                "type": "text",
                "name": "Label",
                "role": "label",
                "content": label,
                "fontSize": 14,
                "fontWeight": 400,
            }
        ],
    }))
}

fn build_switch(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let active = bool_arg(args, "active");
    Ok(json!({
        "id": next_id("switch"),
        "type": "frame",
        "name": if active { "Switch (on)" } else { "Switch (off)" },
        "role": "switch",
        "width": 51,
        "height": 31,
        "cornerRadius": 16,
        "fill": [{ "type": "solid", "color": if active { "#34C759" } else { "#E5E5EA" } }],
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": if active { "flex-end" } else { "flex-start" },
        "padding": [2],
        "children": [{
            "id": next_id("switch_thumb"),
            "type": "frame",
            "name": "Thumb",
            "role": "switch-thumb",
            "width": 27,
            "height": 27,
            "cornerRadius": 14,
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        }],
    }))
}

fn build_segmented_control(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let items = parse_segment_items(args)?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let (track_bg, active_bg, active_label, inactive_label) = if theme_aware {
        match theme {
            "dark" => ("#334155", "#1E293B", "#F1F5F9", "#94A3B8"),
            "system" => (
                "$color-surface-2",
                "$color-surface",
                "$color-text-primary",
                "$color-text-muted",
            ),
            _ => ("#F3F4F6", "#FFFFFF", "#111827", "#4B5563"),
        }
    } else {
        ("#F3F4F6", "#FFFFFF", "#111827", "#4B5563")
    };

    let segments: Vec<Value> = items
        .into_iter()
        .map(|item| {
            let mut segment = json!({
                "id": next_id("segment"),
                "type": "frame",
                "name": format!("Segment ({})", item.label),
                "role": if item.active { "segment-active" } else { "segment" },
                "width": "fill_container",
                "height": "fill_container",
                "cornerRadius": 6,
                "layout": "horizontal",
                "alignItems": "center",
                "justifyContent": "center",
                "children": [{
                    "id": next_id("segment_label"),
                    "type": "text",
                    "name": "Label",
                    "role": "label",
                    "content": item.label,
                    "fontSize": 13,
                    "fontWeight": if item.active { 600 } else { 500 },
                    "fill": [{
                        "type": "solid",
                        "color": if item.active { active_label } else { inactive_label },
                    }],
                }],
            });
            segment["fill"] = if item.active {
                json!([{ "type": "solid", "color": active_bg }])
            } else {
                json!([])
            };
            segment
        })
        .collect();

    Ok(json!({
        "id": next_id("segmented_control"),
        "type": "frame",
        "name": "Segmented Control",
        "role": "segmented-control",
        "width": "fill_container",
        "height": 32,
        "cornerRadius": 8,
        "fill": [{ "type": "solid", "color": track_bg }],
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 4,
        "padding": [4],
        "children": segments,
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

struct SegmentItem {
    label: String,
    active: bool,
}

fn parse_segment_items(args: &BTreeMap<String, String>) -> Result<Vec<SegmentItem>, ToolOutcome> {
    let raw = required(args, "items")?;
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("items must be a JSON array: {e}"),
        )
    })?;
    let Some(items) = value.as_array() else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "items must be a JSON array".into(),
        ));
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let label = item
            .get("label")
            .and_then(Value::as_str)
            .filter(|label| !label.trim().is_empty())
            .ok_or_else(|| {
                ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "items[].label is required".into(),
                )
            })?;
        out.push(SegmentItem {
            label: label.to_string(),
            active: item.get("active").and_then(Value::as_bool).unwrap_or(false),
        });
    }
    Ok(out)
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
