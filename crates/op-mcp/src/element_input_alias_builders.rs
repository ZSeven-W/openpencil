//! Input/form TS element alias builders.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::{ToolErrorCode, ToolOutcome};

static NEXT_INPUT_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn input_alias_node_value(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<Value>, ToolOutcome> {
    let value = match tool {
        "add_chip_input_v0" => build_chip_input(args, false)?,
        "add_chip_input_v1" => build_chip_input(args, true)?,
        "add_combobox_v0" => build_combobox(args, false)?,
        "add_combobox_v1" => build_combobox(args, true)?,
        "add_date_picker_v0" => build_date_picker(args, false)?,
        "add_date_picker_v1" => build_date_picker(args, true)?,
        "add_input_with_action_v0" => build_input_with_action(args, false)?,
        "add_input_with_action_v1" => build_input_with_action(args, true)?,
        "add_otp_input_v0" => build_otp_input(args, false)?,
        "add_otp_input_v1" => build_otp_input(args, true)?,
        "add_textarea_v0" => build_textarea(args)?,
        "add_textarea_v1" => build_textarea(args)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn build_chip_input(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let required_mark = bool_arg(args, "required", false);
    let chips = string_array_arg(args, "chips")?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = input_colors(theme, theme_aware);

    let mut field_children: Vec<Value> = chips
        .iter()
        .map(|chip| {
            json!({
                "id": next_id("chip"),
                "type": "frame",
                "name": format!("Chip ({chip})"),
                "role": "chip",
                "height": "fit_content",
                "layout": "horizontal",
                "alignItems": "center",
                "gap": 4,
                "padding": [6, 4, 6, 10],
                "cornerRadius": 16,
                "fill": [{ "type": "solid", "color": colors.surface2 }],
                "children": [
                    {
                        "id": next_id("chip_value"),
                        "type": "text",
                        "name": "Value",
                        "content": chip,
                        "fontSize": 13,
                        "fontWeight": 500,
                    },
                    icon_node("Remove", None, "x", 14, 14, colors.text_muted),
                ],
            })
        })
        .collect();
    field_children.push(json!({
        "id": next_id("chip_caret"),
        "type": "text",
        "name": "Caret",
        "role": "chip-input-caret",
        "content": args
            .get("placeholder")
            .map(String::as_str)
            .unwrap_or(if chips.is_empty() { "Add tag..." } else { "" }),
        "fontSize": 14,
        "fontWeight": 400,
        "fill": [{ "type": "solid", "color": colors.text_subtle }],
    }));

    Ok(json!({
        "id": next_id("chip_input"),
        "type": "frame",
        "name": "Chip Input",
        "role": "chip-input",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 6,
        "children": [
            {
                "id": next_id("chip_label"),
                "type": "text",
                "name": "Label",
                "role": "chip-input-label",
                "content": label_with_required(label, required_mark),
                "fontSize": 13,
                "fontWeight": 500,
            },
            {
                "id": next_id("chip_field"),
                "type": "frame",
                "name": "Field",
                "role": "chip-input-field",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "horizontal",
                "alignItems": "center",
                "gap": 6,
                "padding": [8, 12],
                "cornerRadius": 8,
                "fill": [{ "type": "solid", "color": colors.surface }],
                "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": colors.border }] },
                "children": field_children,
            },
        ],
    }))
}

fn build_combobox(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let options = object_array_arg(args, "options")?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = input_colors(theme, theme_aware);
    let value = args.get("value").map(String::as_str).unwrap_or("");
    let shown = if value.is_empty() {
        args.get("placeholder")
            .map(String::as_str)
            .unwrap_or("Search...")
    } else {
        value
    };
    let mut stack_children = Vec::new();
    if let Some(label) = args.get("label").filter(|label| !label.is_empty()) {
        stack_children.push(json!({
            "id": next_id("combobox_label"),
            "type": "text",
            "name": "Label",
            "role": "combobox-label",
            "content": label,
            "fontSize": 13,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": colors.text_primary }],
        }));
    }
    stack_children.push(json!({
        "id": next_id("combobox_input"),
        "type": "frame",
        "name": "Input",
        "role": "combobox-input",
        "width": "fill_container",
        "height": 44,
        "cornerRadius": 8,
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 8,
        "padding": [0, 12],
        "fill": [{ "type": "solid", "color": colors.surface }],
        "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": colors.accent }] },
        "children": [
            icon_node("Search Icon", None, "search", 16, 16, colors.text_muted),
            {
                "id": next_id("combobox_value"),
                "type": "text",
                "name": "Value",
                "role": "combobox-value",
                "content": shown,
                "fontSize": 14,
                "fontWeight": 400,
                "fill": [{ "type": "solid", "color": if value.is_empty() { colors.text_subtle } else { colors.text_primary } }],
            },
        ],
    }));
    let option_nodes: Vec<Value> = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let highlighted = item_bool(option, "highlighted");
            let mut node = json!({
                "id": next_id("combobox_option"),
                "type": "frame",
                "name": format!("Option {}", index + 1),
                "role": if highlighted { "combobox-option-active" } else { "combobox-option" },
                "width": "fill_container",
                "height": 36,
                "layout": "horizontal",
                "alignItems": "center",
                "padding": [0, 12],
                "children": [{
                    "id": next_id("combobox_option_label"),
                    "type": "text",
                    "name": "Label",
                    "role": "combobox-option-label",
                    "content": item_string(option, "label").unwrap_or_default(),
                    "fontSize": 14,
                    "fontWeight": if highlighted { 500 } else { 400 },
                    "fill": [{ "type": "solid", "color": colors.text_primary }],
                }],
            });
            if highlighted {
                node["fill"] = json!([{ "type": "solid", "color": colors.surface2 }]);
            }
            node
        })
        .collect();
    stack_children.push(json!({
        "id": next_id("combobox_dropdown"),
        "type": "frame",
        "name": "Dropdown",
        "role": "combobox-dropdown",
        "width": "fill_container",
        "height": "fit_content",
        "cornerRadius": 8,
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": colors.surface }],
        "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": colors.border }] },
        "effects": [{
            "type": "shadow",
            "offsetX": 0,
            "offsetY": 4,
            "blur": 12,
            "spread": 0,
            "color": "#0F172A1A",
        }],
        "children": option_nodes,
    }));

    Ok(json!({
        "id": next_id("combobox"),
        "type": "frame",
        "name": "Combobox",
        "role": "combobox",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 6,
        "children": stack_children,
    }))
}

fn build_date_picker(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let value = args.get("value").map(String::as_str).unwrap_or("");
    let has_value = !value.is_empty();
    let shown = if has_value {
        value
    } else {
        args.get("placeholder")
            .map(String::as_str)
            .unwrap_or("Select date")
    };
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = input_colors(theme, theme_aware);
    let mut right = Vec::new();
    if bool_arg(args, "clearable", false) && has_value {
        right.push(icon_node(
            "Clear",
            Some("date-picker-clear"),
            "x",
            16,
            16,
            colors.text_subtle,
        ));
    }
    right.push(icon_node(
        "Calendar Icon",
        Some("date-picker-icon"),
        "calendar",
        20,
        20,
        colors.text_muted,
    ));
    let mut label_node = json!({
        "id": next_id("date_label"),
        "type": "text",
        "name": "Label",
        "role": "date-picker-label",
        "content": label_with_required(label, bool_arg(args, "required", false)),
        "fontSize": 13,
        "fontWeight": 500,
    });
    if theme_aware && theme != "light" {
        label_node["fill"] = json!([{ "type": "solid", "color": colors.text_primary }]);
    }

    Ok(json!({
        "id": next_id("date_picker"),
        "type": "frame",
        "name": "Date Picker",
        "role": "date-picker",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 6,
        "children": [
            label_node,
            {
                "id": next_id("date_input"),
                "type": "frame",
                "name": "Input",
                "role": "date-picker-input",
                "width": "fill_container",
                "height": 48,
                "cornerRadius": 8,
                "layout": "horizontal",
                "alignItems": "center",
                "justifyContent": "space-between",
                "padding": [0, 12],
                "fill": [{ "type": "solid", "color": colors.surface }],
                "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": colors.border }] },
                "children": [
                    {
                        "id": next_id("date_value"),
                        "type": "text",
                        "name": if has_value { "Value" } else { "Placeholder" },
                        "role": if has_value { "date-picker-value" } else { "date-picker-placeholder" },
                        "content": shown,
                        "fontSize": 15,
                        "fontWeight": 400,
                        "fill": [{ "type": "solid", "color": if has_value { colors.text_primary } else { colors.text_subtle } }],
                    },
                    {
                        "id": next_id("date_right"),
                        "type": "frame",
                        "name": "Right",
                        "height": "fit_content",
                        "layout": "horizontal",
                        "alignItems": "center",
                        "gap": 8,
                        "children": right,
                    },
                ],
            },
        ],
    }))
}

fn build_input_with_action(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let placeholder = required(args, "placeholder")?;
    let value = args.get("value").map(String::as_str).unwrap_or("");
    let is_filled = !value.is_empty();
    let width = number_arg(args, "width", 400.0).floor().max(280.0);
    let kind = args
        .get("action_kind")
        .map(String::as_str)
        .unwrap_or("text");
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = input_colors(theme, theme_aware);
    let mut input_children = Vec::new();
    if let Some(icon) = args.get("leading_icon").filter(|icon| !icon.is_empty()) {
        input_children.push(icon_node(
            "Leading Icon",
            Some("input-with-action-leading-icon"),
            icon,
            18,
            18,
            colors.text_muted,
        ));
    }
    input_children.push(json!({
        "id": next_id("input_action_text"),
        "type": "text",
        "name": "Text",
        "role": "input-with-action-text",
        "content": if is_filled { value } else { placeholder },
        "fontSize": 14,
        "fontWeight": 400,
        "fill": [{ "type": "solid", "color": if is_filled { colors.text_primary } else { colors.text_muted } }],
    }));
    let button = if kind == "icon" {
        let icon = args
            .get("action_icon")
            .map(String::as_str)
            .unwrap_or("arrow-right");
        json!({
            "id": next_id("input_action_button"),
            "type": "frame",
            "name": "Action",
            "role": "input-with-action-button",
            "width": 44,
            "height": 44,
            "cornerRadius": 10,
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "fill": [{ "type": "solid", "color": colors.accent }],
            "children": [icon_node("Action Icon", Some("input-with-action-icon"), icon, 18, 18, "#FFFFFF")],
        })
    } else {
        json!({
            "id": next_id("input_action_button"),
            "type": "frame",
            "name": "Action",
            "role": "input-with-action-button",
            "width": "fit_content",
            "height": 44,
            "cornerRadius": 10,
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "padding": [0, 20],
            "fill": [{ "type": "solid", "color": colors.accent }],
            "children": [{
                "id": next_id("input_action_label"),
                "type": "text",
                "name": "Action Label",
                "role": "input-with-action-label",
                "content": args.get("action_label").map(String::as_str).unwrap_or("Submit"),
                "fontSize": 14,
                "fontWeight": 600,
                "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            }],
        })
    };

    Ok(json!({
        "id": next_id("input_with_action"),
        "type": "frame",
        "name": "Input With Action",
        "role": "input-with-action",
        "width": width,
        "height": 44,
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 8,
        "children": [
            {
                "id": next_id("input_action_input"),
                "type": "frame",
                "name": "Input",
                "role": "input-with-action-input",
                "width": "fill_container",
                "height": 44,
                "cornerRadius": 10,
                "layout": "horizontal",
                "alignItems": "center",
                "gap": if input_children.len() > 1 { 8 } else { 0 },
                "padding": [0, 14],
                "fill": [{ "type": "solid", "color": colors.surface }],
                "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": colors.input_border }] },
                "children": input_children,
            },
            button,
        ],
    }))
}

fn build_otp_input(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let length = number_arg(args, "length", 6.0).floor().clamp(4.0, 8.0) as usize;
    let digits = string_array_arg(args, "digits")?;
    let focused_index = number_arg(args, "focused_index", 0.0)
        .floor()
        .clamp(0.0, (length - 1) as f64) as usize;
    let slot_size = number_arg(args, "slot_size", 48.0)
        .floor()
        .clamp(32.0, 80.0);
    let gap = number_arg(args, "gap", 12.0).floor().clamp(0.0, 24.0);
    let accent = args
        .get("accent_color")
        .map(String::as_str)
        .unwrap_or("#2563EB");
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = input_colors(theme, theme_aware);
    let mut children = Vec::new();
    for index in 0..length {
        let digit = digits.get(index).map(String::as_str).unwrap_or("");
        let filled = !digit.is_empty();
        let focused = index == focused_index && !filled;
        let role = if focused {
            "otp-slot-focused"
        } else if filled {
            "otp-slot-filled"
        } else {
            "otp-slot"
        };
        let border = if focused {
            accent
        } else if filled {
            colors.border_strong
        } else {
            colors.input_border
        };
        let slot_children = if filled {
            vec![json!({
                "id": next_id("otp_digit"),
                "type": "text",
                "name": "Digit",
                "role": "otp-digit",
                "content": digit,
                "fontSize": 20,
                "fontWeight": 600,
                "fill": [{ "type": "solid", "color": colors.text_primary }],
            })]
        } else {
            Vec::new()
        };
        children.push(json!({
            "id": next_id("otp_slot"),
            "type": "frame",
            "name": format!("Slot {}", index + 1),
            "role": role,
            "width": slot_size,
            "height": slot_size,
            "cornerRadius": 8,
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "fill": [{ "type": "solid", "color": colors.surface }],
            "stroke": { "thickness": if focused { 2 } else { 1 }, "fill": [{ "type": "solid", "color": border }] },
            "children": slot_children,
        }));
    }
    Ok(json!({
        "id": next_id("otp_input"),
        "type": "frame",
        "name": "OTP Input",
        "role": "otp-input",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": gap,
        "children": children,
    }))
}

fn build_textarea(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let rows = number_arg(args, "rows", 4.0).clamp(2.0, 12.0);
    let height = rows * 24.0 + 24.0;
    Ok(json!({
        "id": next_id("textarea"),
        "type": "frame",
        "name": "Textarea",
        "role": "textarea",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 6,
        "children": [
            {
                "id": next_id("textarea_label"),
                "type": "text",
                "name": "Label",
                "role": "label",
                "content": label_with_required(label, bool_arg(args, "required", false)),
                "fontSize": 14,
                "fontWeight": 500,
            },
            {
                "id": next_id("textarea_input"),
                "type": "frame",
                "name": "Input",
                "role": "textarea-input",
                "width": "fill_container",
                "height": height,
                "cornerRadius": 8,
                "layout": "vertical",
                "alignItems": "start",
                "padding": [12, 16],
                "children": [{
                    "id": next_id("textarea_placeholder"),
                    "type": "text",
                    "name": "Placeholder",
                    "content": args.get("placeholder").map(String::as_str).unwrap_or(""),
                    "fontSize": 14,
                    "fontWeight": 400,
                    "lineHeight": 1.5,
                }],
            },
        ],
    }))
}

#[derive(Clone, Copy)]
struct InputColors {
    surface: &'static str,
    surface2: &'static str,
    border: &'static str,
    input_border: &'static str,
    border_strong: &'static str,
    text_primary: &'static str,
    text_muted: &'static str,
    text_subtle: &'static str,
    accent: &'static str,
}

fn input_colors(theme: &str, theme_aware: bool) -> InputColors {
    if !theme_aware || theme == "light" {
        return InputColors {
            surface: "#FFFFFF",
            surface2: "#F1F5F9",
            border: "#E2E8F0",
            input_border: "#CBD5E1",
            border_strong: "#334155",
            text_primary: "#0F172A",
            text_muted: "#64748B",
            text_subtle: "#94A3B8",
            accent: "#2563EB",
        };
    }
    if theme == "system" {
        return InputColors {
            surface: "$color-surface",
            surface2: "$color-surface-2",
            border: "$color-border",
            input_border: "$color-border",
            border_strong: "$color-border-strong",
            text_primary: "$color-text-primary",
            text_muted: "$color-text-muted",
            text_subtle: "$color-text-subtle",
            accent: "$color-accent",
        };
    }
    InputColors {
        surface: "#1E293B",
        surface2: "#334155",
        border: "#334155",
        input_border: "#334155",
        border_strong: "#475569",
        text_primary: "#F1F5F9",
        text_muted: "#94A3B8",
        text_subtle: "#64748B",
        accent: "#60A5FA",
    }
}

fn icon_node(
    name: &str,
    role: Option<&str>,
    icon: &str,
    width: i64,
    height: i64,
    color: &str,
) -> Value {
    let mut node = json!({
        "id": next_id("input_icon"),
        "type": "icon_font",
        "name": name,
        "iconFontName": icon,
        "iconFontFamily": "lucide",
        "width": width,
        "height": height,
        "fill": [{ "type": "solid", "color": color }],
    });
    if let Some(role) = role {
        node["role"] = json!(role);
    }
    node
}

fn label_with_required(label: &str, required: bool) -> String {
    if required {
        format!("{label} *")
    } else {
        label.to_string()
    }
}

fn required<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, ToolOutcome> {
    args.get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, format!("{key} is required"))
        })
}

fn bool_arg(args: &BTreeMap<String, String>, key: &str, default: bool) -> bool {
    args.get(key)
        .map(|value| matches!(value.as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(default)
}

fn number_arg(args: &BTreeMap<String, String>, key: &str, default: f64) -> f64 {
    args.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

fn string_array_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Vec<String>, ToolOutcome> {
    let Some(raw) = args.get(key).filter(|raw| !raw.is_empty()) else {
        return Ok(Vec::new());
    };
    let value = serde_json::from_str::<Value>(raw).map_err(|err| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON string array: {err}"),
        )
    })?;
    let Value::Array(items) = value else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON string array"),
        ));
    };
    Ok(items
        .into_iter()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect())
}

fn object_array_arg(args: &BTreeMap<String, String>, key: &str) -> Result<Vec<Value>, ToolOutcome> {
    let raw = args.get(key).ok_or_else(|| {
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, format!("{key} is required"))
    })?;
    let value = serde_json::from_str::<Value>(raw).map_err(|err| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON object array: {err}"),
        )
    })?;
    let Value::Array(items) = value else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON object array"),
        ));
    };
    Ok(items)
}

fn item_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .as_object()
        .and_then(|object| object.get(key))
        .and_then(Value::as_str)
}

fn item_bool(value: &Value, key: &str) -> bool {
    value
        .as_object()
        .and_then(|object| object.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn next_id(prefix: &str) -> String {
    let id = NEXT_INPUT_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{id}")
}
