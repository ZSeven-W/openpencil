//! Visual and chart TS element alias builders.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::{ToolErrorCode, ToolOutcome};

static NEXT_VISUAL_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn visual_alias_node_value(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<Value>, ToolOutcome> {
    let value = match tool {
        "add_activity_ring_v0" | "add_activity_ring_v1" => build_activity_ring(args)?,
        "add_carousel_dots_v0" => build_carousel_dots(args, false)?,
        "add_carousel_dots_v1" => build_carousel_dots(args, true)?,
        "add_color_swatch_v0" | "add_color_swatch_v1" => build_color_swatch(args)?,
        "add_chart_bars_v0" => build_chart_bars(args, false)?,
        "add_chart_bars_v1" => build_chart_bars(args, true)?,
        "add_chart_pie_v0" => build_chart_pie(args, false)?,
        "add_chart_pie_v1" => build_chart_pie(args, true)?,
        "add_empty_chart_v0" => build_empty_chart(args, false)?,
        "add_empty_chart_v1" => build_empty_chart(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn build_activity_ring(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let center_text = required(args, "center_text")?;
    let size = number_arg(args, "size", 80.0);
    let thickness = number_arg(args, "thickness", 8.0);
    Ok(json!({
        "id": next_id("activity_ring"),
        "type": "frame",
        "name": "Activity Ring",
        "role": "activity-ring",
        "width": size,
        "height": size,
        "cornerRadius": size / 2.0,
        "fill": [],
        "stroke": {
            "thickness": thickness,
            "fill": [{ "type": "solid", "color": "#000000" }],
        },
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "children": [{
            "id": next_id("activity_ring_text"),
            "type": "text",
            "name": "Center Text",
            "role": "heading",
            "content": center_text,
            "fontSize": 16,
            "fontWeight": 700,
        }],
    }))
}

fn build_carousel_dots(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let total = number_arg_required(args, "total")?.floor().max(1.0) as usize;
    let current = number_arg(args, "current", 0.0)
        .floor()
        .max(0.0)
        .min((total - 1) as f64) as usize;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let (active_fill, inactive_fill) = if theme_aware {
        match theme {
            "dark" => ("#F1F5F9", "#334155"),
            "system" => ("$color-text-primary", "$color-border"),
            _ => ("#111827", "#D1D5DB"),
        }
    } else {
        ("#111827", "#D1D5DB")
    };

    let mut children = Vec::new();
    for idx in 0..total {
        let active = idx == current;
        children.push(json!({
            "id": next_id("carousel_dot"),
            "type": "frame",
            "name": if active { "Dot Active" } else { "Dot" },
            "role": if active { "dot-active" } else { "dot" },
            "width": if active { 16 } else { 6 },
            "height": 6,
            "cornerRadius": 3,
            "fill": [{ "type": "solid", "color": if active { active_fill } else { inactive_fill } }],
        }));
    }

    Ok(json!({
        "id": next_id("carousel_dots"),
        "type": "frame",
        "name": "Carousel Dots",
        "role": "carousel-dots",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 6,
        "children": children,
    }))
}

fn build_color_swatch(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let color = required(args, "color")?;
    let size = number_arg(args, "size", 64.0).floor().max(16.0);
    let mut children = vec![json!({
        "id": next_id("color_swatch_square"),
        "type": "frame",
        "name": "Swatch Square",
        "role": "color-swatch-square",
        "width": size,
        "height": size,
        "cornerRadius": 12,
        "fill": [{ "type": "solid", "color": color }],
    })];
    if let Some(label) = args.get("label").filter(|label| !label.is_empty()) {
        children.push(json!({
            "id": next_id("color_swatch_label"),
            "type": "text",
            "name": "Label",
            "role": "color-swatch-label",
            "content": label,
            "fontSize": 12,
            "fontWeight": 500,
        }));
    }
    Ok(json!({
        "id": next_id("color_swatch"),
        "type": "frame",
        "name": "Color Swatch",
        "role": "color-swatch",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "vertical",
        "alignItems": "center",
        "gap": 8,
        "children": children,
    }))
}

fn build_chart_bars(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let values = parse_number_array_arg(args, "values")?;
    if values.is_empty() {
        return Err(invalid_arg(
            "buildChartBars: values must contain at least one number",
        ));
    }
    let values: Vec<f64> = values.into_iter().map(|value| value.max(0.0)).collect();
    let max = values.iter().fold(1.0_f64, |acc, value| acc.max(*value));
    let bar_width = number_arg(args, "bar_width", 24.0).floor().max(4.0);
    let gap = number_arg(args, "gap", 12.0).floor().max(0.0);
    let chart_height = number_arg(args, "chart_height", 160.0).floor().max(40.0);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let bar_color = if theme_aware {
        match theme {
            "system" => "$color-chart-1",
            "dark" => "#3B82F6",
            _ => "#2563EB",
        }
    } else {
        "#2563EB"
    };

    let children: Vec<Value> = values
        .iter()
        .enumerate()
        .map(|(idx, value)| {
            json!({
                "id": next_id("chart_bar"),
                "type": "rectangle",
                "name": format!("Bar {}", idx + 1),
                "role": "chart-bar",
                "width": bar_width,
                "height": ((*value / max) * chart_height).round().max(2.0),
                "cornerRadius": 4,
                "fill": [{ "type": "solid", "color": bar_color }],
            })
        })
        .collect();

    Ok(json!({
        "id": next_id("chart_bars"),
        "type": "frame",
        "name": "Chart Bars",
        "role": "chart-bars",
        "width": "fit_content",
        "height": chart_height,
        "layout": "horizontal",
        "alignItems": "flex-end",
        "gap": gap,
        "children": children,
    }))
}

fn build_chart_pie(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let mut values = parse_number_array_arg(args, "values")?;
    if values.is_empty() {
        return Err(invalid_arg(
            "buildChartPie: values must contain at least one number",
        ));
    }
    values = values.into_iter().map(|value| value.max(0.0)).collect();
    let mut total: f64 = values.iter().sum();
    if total <= 0.0 {
        if theme_aware {
            values = vec![1.0];
            total = 1.0;
        } else {
            return Err(invalid_arg("buildChartPie: values must sum to > 0"));
        }
    }
    let diameter = number_arg(args, "diameter", 160.0).floor().max(40.0);
    let inner_ratio = number_arg(args, "inner_radius_ratio", 0.0).clamp(0.0, 0.9);
    let caller_colors = optional_string_array_arg(args, "colors")?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let default_palette = chart_palette(theme, theme_aware);
    let mut current_angle = -90.0;
    let mut children = Vec::new();
    for (idx, value) in values.iter().enumerate() {
        let sweep = (*value / total) * 360.0;
        let color = caller_colors
            .get(idx)
            .map(String::as_str)
            .unwrap_or(default_palette[idx % default_palette.len()]);
        let mut slice = json!({
            "id": next_id("chart_pie_slice"),
            "type": "ellipse",
            "name": format!("Slice {}", idx + 1),
            "role": "chart-pie-slice",
            "x": 0,
            "y": 0,
            "width": diameter,
            "height": diameter,
            "startAngle": current_angle,
            "sweepAngle": sweep,
            "fill": [{ "type": "solid", "color": color }],
        });
        if inner_ratio > 0.0 {
            slice["innerRadius"] = json!(inner_ratio);
        }
        children.push(slice);
        current_angle += sweep;
    }

    Ok(json!({
        "id": next_id("chart_pie"),
        "type": "frame",
        "name": "Chart Pie",
        "role": "chart-pie",
        "width": diameter,
        "height": diameter,
        "layout": "none",
        "children": children,
    }))
}

fn build_empty_chart(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let width = number_arg(args, "width", 320.0).floor().max(120.0);
    let height = number_arg(args, "height", 200.0).floor().max(100.0);
    let title = args
        .get("title")
        .map(String::as_str)
        .unwrap_or("No data yet");
    let subtitle = args
        .get("subtitle")
        .map(String::as_str)
        .unwrap_or("Data will appear here once tracking begins.");
    let icon = args
        .get("icon")
        .map(String::as_str)
        .unwrap_or("bar-chart-2");
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = empty_chart_colors(theme, theme_aware);
    let corner_radius = match args.get("corner_radius") {
        Some(_) => json!(number_arg(args, "corner_radius", 12.0).floor().max(0.0)),
        None => json!(12),
    };
    let (gap, padding, title_size, subtitle_size, subtitle_weight) =
        (json!(8), json!(24), json!(14), json!(12), json!(400));

    Ok(json!({
        "id": next_id("empty_chart"),
        "type": "frame",
        "name": "Empty Chart",
        "role": "empty-chart",
        "width": width,
        "height": height,
        "cornerRadius": corner_radius,
        "layout": "vertical",
        "alignItems": "center",
        "justifyContent": "center",
        "gap": gap,
        "padding": padding,
        "fill": [{ "type": "solid", "color": colors.bg }],
        "stroke": {
            "thickness": 1,
            "fill": [{ "type": "solid", "color": colors.border }],
            "strokeDashArray": [4, 4],
        },
        "children": [
            icon_node("Icon", Some("empty-chart-icon"), icon, 40.0, 40.0, Some(colors.icon)),
            {
                "id": next_id("empty_chart_title"),
                "type": "text",
                "name": "Title",
                "role": "empty-chart-title",
                "content": title,
                "fontSize": title_size,
                "fontWeight": 600,
                "fill": [{ "type": "solid", "color": colors.title }],
            },
            {
                "id": next_id("empty_chart_subtitle"),
                "type": "text",
                "name": "Subtitle",
                "role": "empty-chart-subtitle",
                "content": subtitle,
                "fontSize": subtitle_size,
                "fontWeight": subtitle_weight,
                "fill": [{ "type": "solid", "color": colors.subtitle }],
            },
        ],
    }))
}

struct EmptyChartColors {
    bg: &'static str,
    border: &'static str,
    icon: &'static str,
    title: &'static str,
    subtitle: &'static str,
}

fn empty_chart_colors(theme: &str, theme_aware: bool) -> EmptyChartColors {
    if !theme_aware || theme == "light" {
        return EmptyChartColors {
            bg: "#F8FAFC",
            border: "#CBD5E1",
            icon: "#94A3B8",
            title: "#334155",
            subtitle: "#64748B",
        };
    }
    if theme == "system" {
        return EmptyChartColors {
            bg: "$color-surface-2",
            border: "$color-border",
            icon: "$color-text-muted",
            title: "$color-text-primary",
            subtitle: "$color-text-muted",
        };
    }
    EmptyChartColors {
        bg: "#1E293B",
        border: "#475569",
        icon: "#94A3B8",
        title: "#E2E8F0",
        subtitle: "#94A3B8",
    }
}

fn chart_palette(theme: &str, theme_aware: bool) -> &'static [&'static str] {
    if theme_aware && theme == "system" {
        &[
            "$color-chart-1",
            "$color-chart-2",
            "$color-chart-3",
            "$color-chart-4",
            "$color-chart-5",
            "$color-chart-6",
        ]
    } else if theme_aware && theme == "dark" {
        &[
            "#3B82F6", "#8B5CF6", "#EC4899", "#14B8A6", "#F59E0B", "#F97316",
        ]
    } else {
        &[
            "#2563EB", "#10B981", "#F59E0B", "#EF4444", "#8B5CF6", "#EC4899",
        ]
    }
}

fn parse_number_array_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Vec<f64>, ToolOutcome> {
    let raw = required(args, key)?;
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON number array: {e}"),
        )
    })?;
    let Some(values) = value.as_array() else {
        return Err(invalid_arg(format!("{key} must be a JSON number array")));
    };
    Ok(values.iter().filter_map(Value::as_f64).collect())
}

fn optional_string_array_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Vec<String>, ToolOutcome> {
    let Some(raw) = args.get(key) else {
        return Ok(Vec::new());
    };
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON string array: {e}"),
        )
    })?;
    let Some(values) = value.as_array() else {
        return Err(invalid_arg(format!("{key} must be a JSON string array")));
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

fn invalid_arg(message: impl Into<String>) -> ToolOutcome {
    ToolOutcome::Err(ToolErrorCode::InvalidArgument, message.into())
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
    let n = NEXT_VISUAL_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_visual_{prefix}_{n}")
}
