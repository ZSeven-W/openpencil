//! Basic TS element alias builders.
//!
//! These cover small atomic UI pieces whose TS `pen-core` builders are
//! simple enough to mirror directly.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::{ToolErrorCode, ToolOutcome};

static NEXT_BASIC_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn basic_alias_node_value(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<Value>, ToolOutcome> {
    let value = match tool {
        "add_badge_v0" | "add_badge_v1" => build_badge(args)?,
        "add_divider_v0" | "add_divider_v1" => build_divider(args)?,
        "add_avatar_v0" | "add_avatar_v1" => build_avatar(args)?,
        "add_icon_button_v0" | "add_icon_button_v1" => build_icon_button(args)?,
        "add_icon_label_v0" | "add_icon_label_v1" => build_icon_label(args)?,
        "add_skeleton_v0" => build_skeleton(args, false)?,
        "add_skeleton_v1" => build_skeleton(args, true)?,
        "add_spinner_v0" | "add_spinner_v1" => build_spinner(args)?,
        "add_tooltip_v0" | "add_tooltip_v1" => build_tooltip(args)?,
        "add_status_badge_v0" | "add_status_badge_v1" => build_status_badge(args)?,
        "add_progress_bar_v0" => build_progress_bar(args, false)?,
        "add_progress_bar_v1" => build_progress_bar(args, true)?,
        "add_rating_stars_v0" | "add_rating_stars_v1" => build_rating_stars(args)?,
        "add_fab_v0" => build_fab(args, false)?,
        "add_fab_v1" => build_fab(args, true)?,
        "add_link_v0" | "add_link_v1" => build_link(args)?,
        "add_kbd_v0" => build_kbd(args, false)?,
        "add_kbd_v1" => build_kbd(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn build_divider(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let orientation = args
        .get("orientation")
        .map(String::as_str)
        .unwrap_or("horizontal");
    let thickness = number_arg(args, "thickness", 1.0);
    Ok(json!({
        "id": next_id("divider"),
        "type": "rectangle",
        "name": "Divider",
        "role": "divider",
        "width": if orientation == "horizontal" {
            json!("fill_container")
        } else {
            json!(thickness)
        },
        "height": if orientation == "horizontal" {
            json!(thickness)
        } else {
            json!("fill_container")
        },
    }))
}

fn build_avatar(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let size = number_arg(args, "size", 40.0);
    let mut children = Vec::new();
    if let Some(initial) = args.get("initial").filter(|initial| !initial.is_empty()) {
        children.push(json!({
            "id": next_id("avatar_initial"),
            "type": "text",
            "name": "Initial",
            "role": "heading",
            "content": initial,
            "fontSize": (size * 0.4).round().max(12.0),
            "fontWeight": 600,
        }));
    }
    Ok(json!({
        "id": next_id("avatar"),
        "type": "frame",
        "name": "Avatar",
        "role": "avatar",
        "width": size,
        "height": size,
        "cornerRadius": size / 2.0,
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "children": children,
    }))
}

fn build_icon_button(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let icon = required(args, "icon")?;
    let size = number_arg(args, "size", 44.0);
    let icon_size = number_arg(args, "icon_size", 24.0);
    Ok(json!({
        "id": next_id("icon_button"),
        "type": "frame",
        "name": "Icon Button",
        "role": "icon-button",
        "width": size,
        "height": size,
        "layout": "horizontal",
        "justifyContent": "center",
        "alignItems": "center",
        "cornerRadius": 8,
        "children": [icon_node("Icon", None, icon, icon_size, icon_size, None)],
    }))
}

fn build_icon_label(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let icon = required(args, "icon")?;
    let label = required(args, "label")?;
    let gap = number_arg(args, "gap", 8.0);
    Ok(json!({
        "id": next_id("icon_label"),
        "type": "frame",
        "name": "Icon Label",
        "role": "icon-label",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": gap,
        "children": [
            icon_node("Icon", None, icon, 16.0, 16.0, None),
            {
                "id": next_id("icon_label_text"),
                "type": "text",
                "name": "Label",
                "role": "label",
                "content": label,
                "fontSize": 14,
                "fontWeight": 500,
            },
        ],
    }))
}

fn build_skeleton(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let rows = number_arg(args, "rows", 3.0).clamp(1.0, 20.0);
    let row_height = number_arg(args, "row_height", 16.0).clamp(4.0, 48.0);
    let row_gap = number_arg(args, "row_gap", 12.0).clamp(0.0, 32.0);
    let last_row_short = bool_arg(args, "last_row_short", true);
    let row_count = rows.floor() as usize;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let row_fill = if theme_aware {
        match theme {
            "dark" => "#334155",
            "system" => "$color-surface-2",
            _ => "#E2E8F0",
        }
    } else {
        "#E2E8F0"
    };

    let mut children = Vec::new();
    for idx in 0..row_count {
        let short = idx == row_count - 1 && last_row_short && row_count > 1;
        children.push(json!({
            "id": next_id("skeleton_row"),
            "type": "rectangle",
            "role": "skeleton-row",
            "width": if short { json!(220) } else { json!("fill_container") },
            "height": row_height,
            "cornerRadius": 4,
            "fill": [{ "type": "solid", "color": row_fill }],
        }));
    }

    Ok(json!({
        "id": next_id("skeleton"),
        "type": "frame",
        "name": "Skeleton",
        "role": "skeleton",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": row_gap,
        "alignItems": "start",
        "children": children,
    }))
}

fn build_spinner(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let size = number_arg(args, "size", 32.0).floor().clamp(16.0, 128.0);
    let thickness = number_arg(args, "thickness", 3.0).floor().clamp(1.0, 16.0);
    let track_color = args
        .get("track_color")
        .map(String::as_str)
        .unwrap_or("#E2E8F0");
    let active_color = args
        .get("active_color")
        .map(String::as_str)
        .unwrap_or("#2563EB");
    Ok(json!({
        "id": next_id("spinner"),
        "type": "frame",
        "name": "Spinner",
        "role": "spinner",
        "width": size,
        "height": size,
        "layout": "none",
        "children": [
            {
                "id": next_id("spinner_track"),
                "type": "ellipse",
                "name": "Track",
                "role": "spinner-track",
                "x": 0,
                "y": 0,
                "width": size,
                "height": size,
                "fill": [],
                "stroke": { "thickness": thickness, "fill": [{ "type": "solid", "color": track_color }] },
            },
            {
                "id": next_id("spinner_arc"),
                "type": "ellipse",
                "name": "Active Arc",
                "role": "spinner-arc",
                "x": 0,
                "y": 0,
                "width": size,
                "height": size,
                "startAngle": -90,
                "sweepAngle": 270,
                "fill": [],
                "stroke": { "thickness": thickness, "fill": [{ "type": "solid", "color": active_color }] },
            },
        ],
    }))
}

fn build_tooltip(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let text = required(args, "text")?;
    let position = args.get("position").map(String::as_str).unwrap_or("top");
    Ok(json!({
        "id": next_id("tooltip"),
        "type": "frame",
        "name": "Tooltip",
        "role": format!("tooltip-{position}"),
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "padding": [6, 10],
        "cornerRadius": 6,
        "fill": [{ "type": "solid", "color": "#111827" }],
        "children": [{
            "id": next_id("tooltip_text"),
            "type": "text",
            "name": "Text",
            "role": "tooltip-text",
            "content": text,
            "fontSize": 12,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        }],
    }))
}

fn build_badge(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    Ok(json!({
        "id": next_id("badge"),
        "type": "frame",
        "name": "Badge",
        "role": "badge",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "padding": [4, 10],
        "cornerRadius": 999,
        "children": [{
            "id": next_id("badge_label"),
            "type": "text",
            "name": "Label",
            "role": "label",
            "content": label,
            "fontSize": 11,
            "fontWeight": 600,
        }],
    }))
}

fn build_status_badge(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let tone = args.get("tone").map(String::as_str).unwrap_or("neutral");
    Ok(json!({
        "id": next_id("status_badge"),
        "type": "frame",
        "name": "Status Badge",
        "role": "status-badge",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 6,
        "children": [
            {
                "id": next_id("status_dot"),
                "type": "frame",
                "name": "Status Dot",
                "role": "status-dot",
                "width": 8,
                "height": 8,
                "cornerRadius": 4,
                "fill": [{ "type": "solid", "color": status_dot_color(tone) }],
            },
            {
                "id": next_id("status_label"),
                "type": "text",
                "name": "Label",
                "role": "status-label",
                "content": label,
                "fontSize": 13,
                "fontWeight": 500,
            }
        ],
    }))
}

fn build_progress_bar(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let value = number_arg(args, "value", 50.0).clamp(0.0, 100.0);
    let bar_width = number_arg(args, "bar_width", 240.0);
    let fill_width = ((bar_width * value) / 100.0).round().max(0.0);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let track_color = if theme_aware {
        match theme {
            "dark" => "#334155",
            "system" => "$color-surface-2",
            _ => "#E5E7EB",
        }
    } else {
        "#E5E7EB"
    };

    let mut children = Vec::new();
    if fill_width > 0.0 {
        children.push(json!({
            "id": next_id("progress_fill"),
            "type": "rectangle",
            "name": "Fill",
            "role": "progress-bar-fill",
            "width": fill_width,
            "height": 8,
            "cornerRadius": 4,
            "fill": [{ "type": "solid", "color": "#2563EB" }],
        }));
    }

    Ok(json!({
        "id": next_id("progress_bar"),
        "type": "frame",
        "name": "Progress Bar",
        "role": "progress-bar",
        "width": bar_width,
        "height": 8,
        "cornerRadius": 4,
        "fill": [{ "type": "solid", "color": track_color }],
        "layout": "horizontal",
        "alignItems": "center",
        "children": children,
    }))
}

fn build_rating_stars(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let total = number_arg(args, "total", 5.0).floor().max(1.0) as i32;
    let filled = number_arg_required(args, "filled")?
        .floor()
        .max(0.0)
        .min(total as f64) as i32;
    let size = number_arg(args, "size", 16.0).floor().max(8.0);
    let mut children = Vec::new();
    for idx in 0..total {
        let is_filled = idx < filled;
        children.push(json!({
            "id": next_id("rating_star"),
            "type": "icon_font",
            "name": if is_filled { "Star Filled" } else { "Star Empty" },
            "role": if is_filled { "star-filled" } else { "star-empty" },
            "iconFontName": "star",
            "iconFontFamily": "lucide",
            "width": size,
            "height": size,
        }));
    }
    Ok(json!({
        "id": next_id("rating_stars"),
        "type": "frame",
        "name": "Rating Stars",
        "role": "rating-stars",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 2,
        "children": children,
    }))
}

fn build_fab(args: &BTreeMap<String, String>, theme_aware: bool) -> Result<Value, ToolOutcome> {
    let icon = required(args, "icon")?;
    let size = number_arg(args, "size", 56.0);
    let icon_size = (size * 0.43).round();
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let fab_bg = if theme_aware {
        match theme {
            "dark" => "#60A5FA",
            "system" => "$color-accent",
            _ => "#2563EB",
        }
    } else {
        "#2563EB"
    };
    Ok(json!({
        "id": next_id("fab"),
        "type": "frame",
        "name": "FAB",
        "role": "fab",
        "width": size,
        "height": size,
        "cornerRadius": size / 2.0,
        "fill": [{ "type": "solid", "color": fab_bg }],
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "children": [icon_node("Icon", None, icon, icon_size, icon_size, Some("#FFFFFF"))],
    }))
}

fn build_link(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let mut children = vec![json!({
        "id": next_id("link_label"),
        "type": "text",
        "name": "Label",
        "role": "link-label",
        "content": label,
        "fontSize": 14,
        "fontWeight": 500,
    })];
    if let Some(icon) = args.get("trailing_icon") {
        children.push(icon_node(
            "Trailing Icon",
            Some("link-icon"),
            icon,
            14.0,
            14.0,
            None,
        ));
    }
    Ok(json!({
        "id": next_id("link"),
        "type": "frame",
        "name": "Link",
        "role": "link",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 4,
        "children": children,
    }))
}

fn build_kbd(args: &BTreeMap<String, String>, theme_aware: bool) -> Result<Value, ToolOutcome> {
    let mut keys = parse_string_array_arg(args, "keys")?;
    keys.retain(|key| !key.is_empty());
    if keys.is_empty() && theme_aware {
        keys.push("?".to_string());
    } else if keys.is_empty() {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "keys must contain at least one non-empty string".into(),
        ));
    }
    let separator = args.get("separator").map(String::as_str).unwrap_or("+");
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let (key_bg, key_stroke) = if theme_aware {
        match theme {
            "dark" => ("#334155", "#334155"),
            "system" => ("$color-surface-2", "$color-border"),
            _ => ("#F3F4F6", "#D1D5DB"),
        }
    } else {
        ("#F3F4F6", "#D1D5DB")
    };

    let mut children = Vec::new();
    for (idx, key) in keys.iter().enumerate() {
        children.push(json!({
            "id": next_id("kbd_key"),
            "type": "frame",
            "name": format!("Key {}", idx + 1),
            "role": "kbd-key",
            "width": "fit_content",
            "height": "fit_content",
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "padding": [2, 6],
            "cornerRadius": 4,
            "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": key_stroke }] },
            "fill": [{ "type": "solid", "color": key_bg }],
            "children": [{
                "id": next_id("kbd_glyph"),
                "type": "text",
                "name": "Glyph",
                "role": "kbd-glyph",
                "content": key,
                "fontSize": 12,
                "fontWeight": 500,
            }],
        }));
        if idx < keys.len() - 1 && !separator.is_empty() {
            children.push(json!({
                "id": next_id("kbd_separator"),
                "type": "text",
                "name": "Separator",
                "role": "kbd-separator",
                "content": separator,
                "fontSize": 12,
                "fontWeight": 400,
            }));
        }
    }

    Ok(json!({
        "id": next_id("kbd"),
        "type": "frame",
        "name": "Keyboard Shortcut",
        "role": "kbd",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 4,
        "children": children,
    }))
}

fn status_dot_color(tone: &str) -> &'static str {
    match tone {
        "success" => "#10B981",
        "warning" => "#F59E0B",
        "error" => "#EF4444",
        "info" => "#3B82F6",
        _ => "#94A3B8",
    }
}

fn parse_string_array_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Vec<String>, ToolOutcome> {
    let raw = required(args, key)?;
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON string array: {e}"),
        )
    })?;
    let Some(values) = value.as_array() else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON string array"),
        ));
    };
    Ok(values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect())
}

fn required<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, ToolOutcome> {
    args.get(key).map(String::as_str).ok_or_else(|| {
        ToolOutcome::Err(ToolErrorCode::MissingArgument, format!("{key} is required"))
    })
}

fn number_arg(args: &BTreeMap<String, String>, key: &str, default: f64) -> f64 {
    args.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}

fn number_arg_required(args: &BTreeMap<String, String>, key: &str) -> Result<f64, ToolOutcome> {
    required(args, key)?
        .parse::<f64>()
        .map_err(|e| ToolOutcome::Err(ToolErrorCode::InvalidArgument, format!("{key}: {e}")))
}

fn bool_arg(args: &BTreeMap<String, String>, key: &str, default: bool) -> bool {
    args.get(key)
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(default)
}

fn icon_node(
    name: &str,
    role: Option<&str>,
    icon: &str,
    width: f64,
    height: f64,
    fill: Option<&str>,
) -> Value {
    let mut value = json!({
        "id": next_id("icon"),
        "type": "icon_font",
        "name": name,
        "iconFontName": icon,
        "iconFontFamily": "lucide",
        "width": width,
        "height": height,
    });
    if let Some(role) = role {
        value["role"] = json!(role);
    }
    if let Some(fill) = fill {
        value["fill"] = json!([{ "type": "solid", "color": fill }]);
    }
    value
}

fn next_id(prefix: &str) -> String {
    let n = NEXT_BASIC_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_basic_{prefix}_{n}")
}
