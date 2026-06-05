//! Complex TS element alias builders.
//!
//! These mirror selected `pen-core` element-builders whose output is
//! too large for the atomic alias builder module.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::{ToolErrorCode, ToolOutcome};

static NEXT_COMPLEX_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn complex_alias_node_value(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<Value>, ToolOutcome> {
    let value = match tool {
        "add_action_menu_v0" => build_action_menu(args, false)?,
        "add_action_menu_v1" => build_action_menu(args, true)?,
        "add_chart_line_v0" => build_chart_line(args, false)?,
        "add_chart_line_v1" => build_chart_line(args, true)?,
        "add_pricing_card_v0" => build_pricing_card(args, false)?,
        "add_pricing_card_v1" => build_pricing_card(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn build_action_menu(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let items = parse_object_items(args, "items", "items[].label is required")?;
    let width = number_arg(args, "width", 200.0, 140.0).floor();
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let (surface, border, icon, label, destructive) = menu_colors(theme_aware, theme);

    let mut children = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        if bool_field(item, "divider_before") && idx > 0 {
            children.push(json!({
                "id": next_id("action_menu_divider"),
                "type": "rectangle",
                "name": "Divider",
                "role": "action-menu-divider",
                "width": "fill_container",
                "height": 1,
                "fill": [{ "type": "solid", "color": border }],
            }));
        }

        let label_text = string_field(item, "label").expect("validated label");
        let is_destructive = bool_field(item, "destructive");
        let item_color = if is_destructive { destructive } else { label };
        let mut item_children = Vec::new();
        if let Some(icon_name) = string_field(item, "icon") {
            item_children.push(icon_node(
                "Icon",
                None,
                icon_name,
                18,
                18,
                if is_destructive { destructive } else { icon },
            ));
        }
        item_children.push(json!({
            "id": next_id("action_menu_label"),
            "type": "text",
            "name": "Label",
            "content": label_text,
            "fontSize": 14,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": item_color }],
        }));
        children.push(json!({
            "id": next_id("action_menu_item"),
            "type": "frame",
            "name": format!("Item ({label_text})"),
            "role": if is_destructive { "action-menu-item-destructive" } else { "action-menu-item" },
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "alignItems": "center",
            "gap": 10,
            "padding": [8, 12],
            "children": item_children,
        }));
    }

    Ok(json!({
        "id": next_id("action_menu"),
        "type": "frame",
        "name": "Action Menu",
        "role": "action-menu",
        "width": width,
        "height": "fit_content",
        "layout": "vertical",
        "padding": [8, 0],
        "cornerRadius": 10,
        "fill": [{ "type": "solid", "color": surface }],
        "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": border }] },
        "effects": [{
            "type": "shadow",
            "color": "rgba(15, 23, 42, 0.08)",
            "offsetX": 0,
            "offsetY": 8,
            "blur": 24,
            "spread": 0,
        }],
        "children": children,
    }))
}

fn build_chart_line(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let values = parse_number_values(args, theme_aware)?;
    let max_value = values
        .iter()
        .copied()
        .fold(1.0_f64, |acc, value| acc.max(value));
    let spacing = number_arg(args, "point_spacing", 32.0, 8.0).floor();
    let chart_height = number_arg(args, "chart_height", 160.0, 40.0).floor();
    let dots = optional_bool_arg(args, "dots").unwrap_or(true);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let stroke_color = if theme_aware && theme != "light" {
        match theme {
            "system" => "$color-chart-1",
            _ => "#3B82F6",
        }
    } else {
        args.get("stroke_color")
            .map(String::as_str)
            .unwrap_or("#2563EB")
    };
    let total_width = spacing * values.len() as f64;

    let points: Vec<(f64, f64)> = values
        .iter()
        .enumerate()
        .map(|(idx, value)| {
            let x = idx as f64 * spacing + spacing / 2.0;
            let y_raw = chart_height - (value / max_value) * chart_height;
            let y = if *value > 0.0 {
                y_raw.min(chart_height - 2.0)
            } else {
                chart_height - 2.0
            };
            (x, y)
        })
        .collect();
    let d_path = points
        .iter()
        .enumerate()
        .map(|(idx, (x, y))| format!("{} {:.1} {:.1}", if idx == 0 { "M" } else { "L" }, x, y))
        .collect::<Vec<_>>()
        .join(" ");

    let mut children = vec![json!({
        "id": next_id("chart_line_path"),
        "type": "path",
        "name": "Line",
        "role": "chart-line-path",
        "d": d_path,
        "width": total_width,
        "height": chart_height,
        "fill": [],
        "stroke": { "thickness": 2, "fill": [{ "type": "solid", "color": stroke_color }] },
    })];
    if dots {
        for (idx, (x, y)) in points.iter().enumerate() {
            children.push(json!({
                "id": next_id("chart_line_dot"),
                "type": "ellipse",
                "name": format!("Dot {}", idx + 1),
                "role": "chart-line-dot",
                "x": x - 4.0,
                "y": y - 4.0,
                "width": 8,
                "height": 8,
                "fill": [{ "type": "solid", "color": stroke_color }],
            }));
        }
    }

    Ok(json!({
        "id": next_id("chart_line"),
        "type": "frame",
        "name": "Chart Line",
        "role": "chart-line",
        "width": total_width,
        "height": chart_height,
        "layout": "none",
        "children": children,
    }))
}

fn build_pricing_card(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let tier = required(args, "tier")?;
    let price = required(args, "price")?;
    let currency = args.get("currency").map(String::as_str).unwrap_or("$");
    let period = args.get("period").map(String::as_str);
    let features = parse_string_array_arg(args, "features")?.unwrap_or_default();
    let cta = args.get("cta").map(String::as_str).unwrap_or("Get started");
    let emphasis = args
        .get("emphasis")
        .map(String::as_str)
        .unwrap_or("default");
    let is_featured = emphasis == "featured";
    let width = number_arg(args, "width", 280.0, 220.0).floor();
    let corner_radius = number_arg(args, "corner_radius", 16.0, 0.0).floor();
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = pricing_colors(theme_aware, theme, is_featured);

    let mut children = Vec::new();
    let badge_label = match args.get("badge").map(String::as_str) {
        Some("") => None,
        Some(label) => Some(label),
        None if is_featured => Some("Most popular"),
        None => None,
    };
    if let Some(badge_label) = badge_label {
        children.push(json!({
            "id": next_id("pricing_badge"),
            "type": "frame",
            "name": "Badge",
            "role": "pricing-badge",
            "width": "fit_content",
            "height": 24,
            "cornerRadius": 999,
            "layout": "horizontal",
            "alignItems": "center",
            "padding": [0, 10],
            "fill": [{ "type": "solid", "color": colors.badge_bg }],
            "children": [{
                "id": next_id("pricing_badge_label"),
                "type": "text",
                "name": "Badge Label",
                "role": "pricing-badge-label",
                "content": badge_label,
                "fontSize": 11,
                "fontWeight": 600,
                "letterSpacing": 0.5,
                "fill": [{ "type": "solid", "color": colors.badge_text }],
            }],
        }));
    }

    let mut header_children = vec![json!({
        "id": next_id("pricing_tier"),
        "type": "text",
        "name": "Tier",
        "role": "pricing-tier",
        "content": tier,
        "fontSize": 18,
        "fontWeight": 600,
        "fill": [{ "type": "solid", "color": colors.tier }],
    })];
    if let Some(description) = args.get("description").map(String::as_str) {
        header_children.push(json!({
            "id": next_id("pricing_description"),
            "type": "text",
            "name": "Description",
            "role": "pricing-description",
            "content": description,
            "fontSize": 13,
            "fontWeight": 400,
            "fill": [{ "type": "solid", "color": colors.description }],
        }));
    }
    children.push(json!({
        "id": next_id("pricing_header"),
        "type": "frame",
        "name": "Header",
        "role": "pricing-header",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 6,
        "children": header_children,
    }));

    let mut price_children = vec![
        json!({
            "id": next_id("pricing_currency"),
            "type": "text",
            "name": "Currency",
            "role": "pricing-currency",
            "content": currency,
            "fontSize": 18,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": colors.tier }],
        }),
        json!({
            "id": next_id("pricing_amount"),
            "type": "text",
            "name": "Amount",
            "role": "pricing-amount",
            "content": price,
            "fontSize": 40,
            "fontWeight": 700,
            "lineHeight": 1.0,
            "fill": [{ "type": "solid", "color": colors.tier }],
        }),
    ];
    if let Some(period) = period {
        price_children.push(json!({
            "id": next_id("pricing_period"),
            "type": "text",
            "name": "Period",
            "role": "pricing-period",
            "content": period,
            "fontSize": 14,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": colors.period }],
        }));
    }
    children.push(json!({
        "id": next_id("pricing_price_row"),
        "type": "frame",
        "name": "Price Row",
        "role": "pricing-price-row",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "flex-end",
        "gap": 4,
        "children": price_children,
    }));

    if !features.is_empty() {
        let feature_children: Vec<Value> = features
            .iter()
            .take(12)
            .map(|feature| {
                json!({
                    "id": next_id("pricing_feature"),
                    "type": "frame",
                    "name": "Feature",
                    "role": "pricing-feature",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "horizontal",
                    "alignItems": "center",
                    "gap": 10,
                    "children": [
                        icon_node("Check", Some("pricing-feature-check"), "check", 16, 16, colors.check),
                        {
                            "id": next_id("pricing_feature_label"),
                            "type": "text",
                            "name": "Label",
                            "role": "pricing-feature-label",
                            "content": feature,
                            "fontSize": 14,
                            "fontWeight": 400,
                            "fill": [{ "type": "solid", "color": colors.feature_label }],
                        }
                    ],
                })
            })
            .collect();
        children.push(json!({
            "id": next_id("pricing_features"),
            "type": "frame",
            "name": "Features",
            "role": "pricing-features",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "gap": 10,
            "children": feature_children,
        }));
    }

    children.push(json!({
        "id": next_id("pricing_cta"),
        "type": "frame",
        "name": "CTA",
        "role": "pricing-cta",
        "width": "fill_container",
        "height": 44,
        "cornerRadius": 10,
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "fill": [{ "type": "solid", "color": colors.cta_bg }],
        "children": [{
            "id": next_id("pricing_cta_label"),
            "type": "text",
            "name": "CTA Label",
            "role": "pricing-cta-label",
            "content": cta,
            "fontSize": 14,
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        }],
    }));

    Ok(json!({
        "id": next_id("pricing_card"),
        "type": "frame",
        "name": "Pricing Card",
        "role": "pricing-card",
        "width": width,
        "height": "fit_content",
        "cornerRadius": corner_radius,
        "layout": "vertical",
        "gap": 20,
        "padding": 24,
        "fill": [{ "type": "solid", "color": colors.card_bg }],
        "stroke": { "thickness": if is_featured { 2 } else { 1 }, "fill": [{ "type": "solid", "color": colors.border }] },
        "children": children,
    }))
}

struct PricingColors<'a> {
    card_bg: &'a str,
    border: &'a str,
    cta_bg: &'a str,
    tier: &'a str,
    description: &'a str,
    period: &'a str,
    feature_label: &'a str,
    check: &'a str,
    badge_bg: &'a str,
    badge_text: &'a str,
}

fn menu_colors(
    theme_aware: bool,
    theme: &str,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    if !theme_aware {
        return ("#FFFFFF", "#E2E8F0", "#334155", "#0F172A", "#EF4444");
    }
    match theme {
        "dark" => ("#1E293B", "#334155", "#CBD5E1", "#F1F5F9", "#F87171"),
        "system" => (
            "$color-surface",
            "$color-border",
            "$color-text-body",
            "$color-text-primary",
            "$color-destructive",
        ),
        _ => ("#FFFFFF", "#E2E8F0", "#334155", "#0F172A", "#EF4444"),
    }
}

fn pricing_colors(theme_aware: bool, theme: &str, featured: bool) -> PricingColors<'static> {
    let accent = "#2563EB";
    let (
        card_bg,
        default_border,
        default_cta,
        tier,
        description,
        feature_label,
        check,
        badge_bg,
        badge_text,
    ) = if !theme_aware {
        (
            "#FFFFFF", "#E2E8F0", "#0F172A", "#0F172A", "#64748B", "#334155", "#10B981", "#F1F5F9",
            "#334155",
        )
    } else {
        match theme {
            "dark" => (
                "#1E293B", "#334155", "#F1F5F9", "#F1F5F9", "#94A3B8", "#CBD5E1", "#34D399",
                "#334155", "#CBD5E1",
            ),
            "system" => (
                "$color-surface",
                "$color-border",
                "$color-text-primary",
                "$color-text-primary",
                "$color-text-muted",
                "$color-text-body",
                "$color-success",
                "$color-surface-2",
                "$color-text-body",
            ),
            _ => (
                "#FFFFFF", "#E2E8F0", "#0F172A", "#0F172A", "#64748B", "#334155", "#10B981",
                "#F1F5F9", "#334155",
            ),
        }
    };
    PricingColors {
        card_bg,
        border: if featured { accent } else { default_border },
        cta_bg: if featured { accent } else { default_cta },
        tier,
        description,
        period: description,
        feature_label,
        check: if featured { accent } else { check },
        badge_bg: if featured { "#EFF6FF" } else { badge_bg },
        badge_text: if featured { "#1D4ED8" } else { badge_text },
    }
}

fn parse_object_items(
    args: &BTreeMap<String, String>,
    key: &str,
    label_error: &str,
) -> Result<Vec<Value>, ToolOutcome> {
    let raw = required(args, key)?;
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON array: {e}"),
        )
    })?;
    let Some(items) = value.as_array() else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON array"),
        ));
    };
    for item in items {
        string_field(item, "label")
            .filter(|label| !label.trim().is_empty())
            .ok_or_else(|| ToolOutcome::Err(ToolErrorCode::InvalidArgument, label_error.into()))?;
    }
    Ok(items.clone())
}

fn parse_number_values(
    args: &BTreeMap<String, String>,
    allow_default: bool,
) -> Result<Vec<f64>, ToolOutcome> {
    let raw = match args.get("values") {
        Some(raw) => raw,
        None if allow_default => return Ok(vec![10.0, 15.0, 12.0, 20.0, 18.0]),
        None => {
            return Err(ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "values is required".into(),
            ))
        }
    };
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("values must be a JSON number array: {e}"),
        )
    })?;
    let Some(values) = value.as_array() else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "values must be a JSON number array".into(),
        ));
    };
    if values.is_empty() && !allow_default {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "values must contain at least one number".into(),
        ));
    }
    let mut out = Vec::new();
    for value in values {
        out.push(value.as_f64().unwrap_or(0.0).max(0.0));
    }
    if out.is_empty() && allow_default {
        Ok(vec![10.0, 15.0, 12.0, 20.0, 18.0])
    } else {
        Ok(out)
    }
}

fn parse_string_array_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<Vec<String>>, ToolOutcome> {
    let Some(raw) = args.get(key) else {
        return Ok(None);
    };
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
    Ok(Some(
        values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
    ))
}

fn required<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, ToolOutcome> {
    args.get(key).map(String::as_str).ok_or_else(|| {
        ToolOutcome::Err(ToolErrorCode::MissingArgument, format!("{key} is required"))
    })
}

fn number_arg(args: &BTreeMap<String, String>, key: &str, default: f64, min: f64) -> f64 {
    args.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
        .max(min)
}

fn optional_bool_arg(args: &BTreeMap<String, String>, key: &str) -> Option<bool> {
    args.get(key)
        .map(|value| matches!(value.as_str(), "true" | "1"))
}

fn bool_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn icon_node(
    name: &str,
    role: Option<&str>,
    icon: &str,
    width: i32,
    height: i32,
    color: &str,
) -> Value {
    let mut value = json!({
        "id": next_id("icon"),
        "type": "icon_font",
        "name": name,
        "iconFontName": icon,
        "iconFontFamily": "lucide",
        "width": width,
        "height": height,
        "fill": [{ "type": "solid", "color": color }],
    });
    if let Some(role) = role {
        value["role"] = json!(role);
    }
    value
}

fn next_id(prefix: &str) -> String {
    let n = NEXT_COMPLEX_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_complex_{prefix}_{n}")
}
