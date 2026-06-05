//! Misc inline/navigation TS element alias builders.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::{ToolErrorCode, ToolOutcome};

static NEXT_MISC_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn misc_alias_node_value(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<Value>, ToolOutcome> {
    let value = match tool {
        "add_legend_item_v0" => build_legend_item(args, false)?,
        "add_legend_item_v1" => build_legend_item(args, true)?,
        "add_price_v0" | "add_price_v1" => build_price(args)?,
        "add_quote_block_v0" => build_quote_block(args, false)?,
        "add_quote_block_v1" => build_quote_block(args, true)?,
        "add_nav_chip_row_v0" | "add_nav_chip_row_v1" => build_nav_chip_row(args)?,
        "add_tag_v0" | "add_tag_v1" => build_tag(args)?,
        "add_stepper_v0" => build_stepper(args, false)?,
        "add_stepper_v1" => build_stepper(args, true)?,
        "add_timeline_v0" => build_timeline(args, false)?,
        "add_timeline_v1" => build_timeline(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn build_legend_item(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let color = required(args, "color")?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = legend_colors(theme, theme_aware);

    let mut children = vec![
        json!({
            "id": next_id("legend_marker"),
            "type": "frame",
            "name": "Marker",
            "role": "legend-item-marker",
            "width": 10,
            "height": 10,
            "cornerRadius": 2,
            "fill": [{ "type": "solid", "color": color }],
            "children": [],
        }),
        json!({
            "id": next_id("legend_label"),
            "type": "text",
            "name": "Label",
            "role": "legend-item-label",
            "content": label,
            "fontSize": 13,
            "fontWeight": 400,
            "fill": [{ "type": "solid", "color": colors.label }],
        }),
    ];
    if let Some(value) = args.get("value").filter(|value| !value.is_empty()) {
        children.push(json!({
            "id": next_id("legend_value"),
            "type": "text",
            "name": "Value",
            "role": "legend-item-value",
            "content": value,
            "fontSize": 13,
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": colors.value }],
        }));
    }

    Ok(json!({
        "id": next_id("legend_item"),
        "type": "frame",
        "name": "Legend Item",
        "role": "legend-item",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 8,
        "children": children,
    }))
}

fn build_price(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let amount = required(args, "amount")?;
    let currency = args.get("currency").map(String::as_str).unwrap_or("$");
    let mut children = vec![
        json!({
            "id": next_id("price_currency"),
            "type": "text",
            "name": "Currency",
            "role": "price-currency",
            "content": currency,
            "fontSize": 20,
            "fontWeight": 500,
        }),
        json!({
            "id": next_id("price_amount"),
            "type": "text",
            "name": "Amount",
            "role": "price-amount",
            "content": amount,
            "fontSize": 40,
            "fontWeight": 700,
            "lineHeight": 1.0,
        }),
    ];
    if let Some(period) = args.get("period").filter(|period| !period.is_empty()) {
        children.push(json!({
            "id": next_id("price_period"),
            "type": "text",
            "name": "Period",
            "role": "price-period",
            "content": period,
            "fontSize": 14,
            "fontWeight": 500,
        }));
    }
    Ok(json!({
        "id": next_id("price"),
        "type": "frame",
        "name": "Price",
        "role": "price",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "end",
        "gap": 2,
        "children": children,
    }))
}

fn build_quote_block(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let quote = required(args, "quote")?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let mut children = vec![json!({
        "id": next_id("quote_text"),
        "type": "text",
        "name": "Quote",
        "role": "quote-text",
        "content": quote,
        "fontSize": 16,
        "fontWeight": 400,
        "lineHeight": 1.5,
        "width": "fill_container",
        "textGrowth": "fixed-width",
    })];
    if let Some(author) = args.get("author").filter(|author| !author.is_empty()) {
        children.push(json!({
            "id": next_id("quote_author"),
            "type": "text",
            "name": "Author",
            "role": "quote-author",
            "content": format!("\u{2014} {author}"),
            "fontSize": 13,
            "fontWeight": 500,
        }));
    }
    Ok(json!({
        "id": next_id("quote_block"),
        "type": "frame",
        "name": "Quote Block",
        "role": "quote-block",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "padding": [16, 20],
        "gap": 8,
        "cornerRadius": 8,
        "fill": [{ "type": "solid", "color": quote_bg(theme, theme_aware) }],
        "children": children,
    }))
}

fn build_nav_chip_row(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let items = object_array_arg(args, "items")?;
    let chip_width = number_arg(args, "chip_width", 72.0);
    let gap = number_arg(args, "gap", 12.0);
    let chips = items
        .iter()
        .map(|item| nav_chip(item, chip_width))
        .collect::<Vec<_>>();
    Ok(scroll_wrapper("Nav Chip Row", gap, chips))
}

fn nav_chip(item: &Value, chip_width: f64) -> Value {
    let label = item_string(item, "label").unwrap_or_default();
    let active = item_bool(item, "active");
    let mut children = Vec::new();
    if let Some(icon) = item_string(item, "icon").filter(|icon| !icon.is_empty()) {
        children.push(icon_node("Icon", None, icon, 24.0, 24.0, None));
    }
    children.push(json!({
        "id": next_id("nav_chip_label"),
        "type": "text",
        "name": "Label",
        "role": "label",
        "content": label,
        "fontSize": 11,
        "fontWeight": if active { 600 } else { 500 },
    }));
    json!({
        "id": next_id("nav_chip"),
        "type": "frame",
        "name": format!("Chip ({label})"),
        "role": if active { "nav-chip-active" } else { "nav-chip" },
        "width": chip_width,
        "height": "fit_content",
        "cornerRadius": 12,
        "padding": [8, 12],
        "layout": "vertical",
        "alignItems": "center",
        "gap": 4,
        "children": children,
    })
}

fn build_tag(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let tone = tag_tone(args.get("tone").map(String::as_str).unwrap_or("default"))?;
    let removable = bool_arg(args, "removable", true);
    let mut children = vec![json!({
        "id": next_id("tag_label"),
        "type": "text",
        "name": "Label",
        "role": "tag-label",
        "content": label,
        "fontSize": 13,
        "fontWeight": 500,
        "fill": [{ "type": "solid", "color": tone.fg }],
    })];
    if removable {
        children.push(icon_node(
            "Remove",
            Some("tag-remove"),
            "x",
            14.0,
            14.0,
            Some(tone.fg),
        ));
    }
    Ok(json!({
        "id": next_id("tag"),
        "type": "frame",
        "name": "Tag",
        "role": "tag",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 6,
        "padding": [4, 10],
        "cornerRadius": 12,
        "fill": [{ "type": "solid", "color": tone.bg }],
        "children": children,
    }))
}

fn build_stepper(args: &BTreeMap<String, String>, theme_aware: bool) -> Result<Value, ToolOutcome> {
    let total = integer_arg(args, "total", 1)?.max(1);
    let current = integer_arg(args, "current", 0)?.clamp(0, total - 1);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = progress_colors(theme, theme_aware);
    let mut children = Vec::new();
    for i in 0..total {
        let done = i <= current;
        children.push(json!({
            "id": next_id("step"),
            "type": "frame",
            "name": format!("Step {}", i + 1),
            "role": if done { "step-active" } else { "step" },
            "width": 24,
            "height": 24,
            "cornerRadius": 12,
            "fill": [{ "type": "solid", "color": if done { "#2563EB" } else { colors.pending_fill } }],
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "children": [{
                "id": next_id("step_number"),
                "type": "text",
                "name": "Number",
                "content": (i + 1).to_string(),
                "fontSize": 13,
                "fontWeight": 600,
                "fill": [{ "type": "solid", "color": if done { "#FFFFFF" } else { colors.pending_text } }],
            }],
        }));
        if i < total - 1 {
            let done_connector = i < current;
            children.push(json!({
                "id": next_id("step_connector"),
                "type": "rectangle",
                "name": format!("Connector {i}"),
                "role": if done_connector { "step-connector-active" } else { "step-connector" },
                "width": "fill_container",
                "height": 2,
                "fill": [{ "type": "solid", "color": if done_connector { "#2563EB" } else { colors.pending_fill } }],
            }));
        }
    }
    Ok(json!({
        "id": next_id("stepper"),
        "type": "frame",
        "name": "Stepper",
        "role": "stepper",
        "width": "fill_container",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 0,
        "children": children,
    }))
}

fn build_timeline(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let items = timeline_items(args, theme_aware)?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = progress_colors(theme, theme_aware);
    let last = items.len().saturating_sub(1);
    let rows = items
        .iter()
        .enumerate()
        .map(|(i, item)| timeline_item(item, i, i == last, &colors))
        .collect::<Vec<_>>();
    Ok(json!({
        "id": next_id("timeline"),
        "type": "frame",
        "name": "Timeline",
        "role": "timeline",
        "width": "fill_container",
        "layout": "vertical",
        "gap": 0,
        "children": rows,
    }))
}

fn timeline_item(item: &Value, index: usize, is_last: bool, colors: &ProgressColors) -> Value {
    let active = item_bool(item, "active");
    let mut icon_children = vec![json!({
        "id": next_id("timeline_dot"),
        "type": "frame",
        "name": "Dot",
        "role": if active { "timeline-dot-active" } else { "timeline-dot" },
        "width": 24,
        "height": 24,
        "cornerRadius": 12,
        "fill": [{ "type": "solid", "color": if active { "#2563EB" } else { colors.pending_fill } }],
    })];
    if !is_last {
        icon_children.push(json!({
            "id": next_id("timeline_connector"),
            "type": "rectangle",
            "name": "Connector",
            "role": "timeline-connector",
            "width": 2,
            "height": 24,
            "fill": [{ "type": "solid", "color": colors.pending_fill }],
        }));
    }

    let mut content_children = vec![json!({
        "id": next_id("timeline_title"),
        "type": "text",
        "name": "Title",
        "role": "timeline-title",
        "content": item_string(item, "title").unwrap_or_default(),
        "fontSize": 14,
        "fontWeight": 600,
    })];
    if let Some(subtitle) = item_string(item, "subtitle").filter(|subtitle| !subtitle.is_empty()) {
        content_children.push(json!({
            "id": next_id("timeline_subtitle"),
            "type": "text",
            "name": "Subtitle",
            "role": "timeline-subtitle",
            "content": subtitle,
            "fontSize": 12,
            "fontWeight": 400,
            "fill": [{ "type": "solid", "color": colors.pending_text }],
        }));
    }

    json!({
        "id": next_id("timeline_item"),
        "type": "frame",
        "name": format!("Item {}", index + 1),
        "role": "timeline-item",
        "width": "fill_container",
        "layout": "horizontal",
        "alignItems": "start",
        "gap": 12,
        "children": [
            {
                "id": next_id("timeline_icon_column"),
                "type": "frame",
                "name": "Icon Column",
                "role": "timeline-icon-column",
                "width": 24,
                "height": "fit_content",
                "layout": "vertical",
                "alignItems": "center",
                "gap": 0,
                "children": icon_children,
            },
            {
                "id": next_id("timeline_content"),
                "type": "frame",
                "name": "Content",
                "role": "timeline-content",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "gap": 4,
                "children": content_children,
            },
        ],
    })
}

fn scroll_wrapper(row_name: &str, gap: f64, inner_children: Vec<Value>) -> Value {
    json!({
        "id": next_id("scroll_wrapper"),
        "type": "frame",
        "name": row_name,
        "role": "scroll-row-wrapper",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "clipContent": true,
        "children": [{
            "id": next_id("scroll_row"),
            "type": "frame",
            "name": "Scroll Inner Row",
            "role": "scroll-row",
            "width": "fit_content",
            "height": "fit_content",
            "layout": "horizontal",
            "gap": gap,
            "padding": [0, 20],
            "children": inner_children,
        }],
    })
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

fn object_array_arg(args: &BTreeMap<String, String>, key: &str) -> Result<Vec<Value>, ToolOutcome> {
    let raw = required(args, key)?;
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON object array: {e}"),
        )
    })?;
    let Some(values) = value.as_array() else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON object array"),
        ));
    };
    Ok(values
        .iter()
        .filter(|value| value.is_object())
        .cloned()
        .collect())
}

fn timeline_items(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Vec<Value>, ToolOutcome> {
    let items = object_array_arg(args, "items").or_else(|err| {
        if theme_aware {
            Ok(vec![json!({ "title": "Item 1" })])
        } else {
            Err(err)
        }
    })?;
    if items.is_empty() {
        if theme_aware {
            return Ok(vec![json!({ "title": "Item 1" })]);
        }
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "buildTimeline: items must contain at least one entry".to_string(),
        ));
    }
    Ok(items)
}

fn item_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn item_bool(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn bool_arg(args: &BTreeMap<String, String>, key: &str, default: bool) -> bool {
    args.get(key)
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(default)
}

fn number_arg(args: &BTreeMap<String, String>, key: &str, default: f64) -> f64 {
    args.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}

fn integer_arg(
    args: &BTreeMap<String, String>,
    key: &str,
    default: i64,
) -> Result<i64, ToolOutcome> {
    let Some(value) = args.get(key) else {
        return Ok(default);
    };
    let n = value.parse::<f64>().map_err(|_| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a number"),
        )
    })?;
    if !n.is_finite() {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be finite"),
        ));
    }
    Ok(n.floor() as i64)
}

fn required<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, ToolOutcome> {
    args.get(key).map(String::as_str).ok_or_else(|| {
        ToolOutcome::Err(ToolErrorCode::MissingArgument, format!("{key} is required"))
    })
}

struct LegendColors {
    label: &'static str,
    value: &'static str,
}

fn legend_colors(theme: &str, theme_aware: bool) -> LegendColors {
    if !theme_aware || theme == "light" {
        return LegendColors {
            label: "#475569",
            value: "#0F172A",
        };
    }
    if theme == "system" {
        return LegendColors {
            label: "$color-text-body",
            value: "$color-text-primary",
        };
    }
    LegendColors {
        label: "#CBD5E1",
        value: "#F8FAFC",
    }
}

fn quote_bg(theme: &str, theme_aware: bool) -> &'static str {
    if !theme_aware || theme == "light" {
        "#F9FAFB"
    } else if theme == "system" {
        "$color-surface"
    } else {
        "#1E293B"
    }
}

struct ProgressColors {
    pending_fill: &'static str,
    pending_text: &'static str,
}

fn progress_colors(theme: &str, theme_aware: bool) -> ProgressColors {
    if !theme_aware || theme == "light" {
        return ProgressColors {
            pending_fill: "#E5E7EB",
            pending_text: "#6B7280",
        };
    }
    if theme == "system" {
        return ProgressColors {
            pending_fill: "$color-border",
            pending_text: "$color-text-muted",
        };
    }
    ProgressColors {
        pending_fill: "#334155",
        pending_text: "#94A3B8",
    }
}

struct TagTone {
    bg: &'static str,
    fg: &'static str,
}

fn tag_tone(tone: &str) -> Result<TagTone, ToolOutcome> {
    match tone {
        "default" => Ok(TagTone {
            bg: "#F1F5F9",
            fg: "#475569",
        }),
        "accent" => Ok(TagTone {
            bg: "#DBEAFE",
            fg: "#2563EB",
        }),
        "success" => Ok(TagTone {
            bg: "#DCFCE7",
            fg: "#166534",
        }),
        "warning" => Ok(TagTone {
            bg: "#FEF3C7",
            fg: "#B45309",
        }),
        "error" => Ok(TagTone {
            bg: "#FEE2E2",
            fg: "#B91C1C",
        }),
        _ => Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!(
                "add_tag_v0: invalid tone {tone:?}; expected one of: default, accent, success, warning, error"
            ),
        )),
    }
}

fn next_id(prefix: &str) -> String {
    let n = NEXT_MISC_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_misc_{prefix}_{n}")
}
