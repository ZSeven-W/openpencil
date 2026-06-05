//! Ported TS element builders (cards-b). Real role-tagged subtrees
//! for kinds that previously fell to the generic kit placeholder.
#![allow(clippy::result_large_err)]

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::element_ported_helpers::*;
use crate::ToolOutcome;

pub(crate) fn ported_cards_b_alias_node_value(
    tool: &str,
    args: &std::collections::BTreeMap<String, String>,
) -> Result<Option<serde_json::Value>, crate::ToolOutcome> {
    let value = match tool {
        "add_profile_header_v0" => build_profile_header(args, false)?,
        "add_profile_header_v1" => build_profile_header(args, true)?,
        "add_metric_row_v0" | "add_metric_row_v1" => build_metric_row(args)?,
        "add_metric_comparison_v0" => build_metric_comparison(args, false)?,
        "add_metric_comparison_v1" => build_metric_comparison(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

// ===== add_profile_header =====

/// `add_profile_header_v0` / `add_profile_header_v1` — large centered
/// profile header (avatar + name + optional handle/bio). Ports
/// `pen-core/element-builders/profile-header.ts` (v0) and
/// `profile-header-v1.ts` (theme-aware). Light mode is byte-parity with v0.
///
/// Avatar bg (#3B82F6) and initial text (#FFFFFF) are brand-invariant
/// across all themes; only name/handle/bio fills shift in dark/system.
fn build_profile_header(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let name = required(args, "name")?;
    // size = min(160, max(64, floor(avatar_size ?? 96)))
    let size = number_arg(args, "avatar_size", 96.0, 64.0)
        .floor()
        .min(160.0);
    // initialFontSize = max(20, round(size * 0.4))
    let initial_font_size = (size * 0.4).round().max(20.0);

    // Resolve theme-aware text colors (light mode == v0 byte-parity).
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let is_light = !theme_aware || theme == "light";
    let (name_color, handle_color, bio_color): (&str, &str, &str) = if is_light {
        ("#0F172A", "#64748B", "#475569")
    } else {
        match theme {
            "dark" => ("#F1F5F9", "#94A3B8", "#CBD5E1"),
            // 'system' (and any unknown) emits $color-* refs
            _ => (
                "$color-text-primary",
                "$color-text-muted",
                "$color-text-body",
            ),
        }
    };
    // Avatar bg + initial text are brand-invariant.
    let avatar_bg = "#3B82F6";
    let initial_color = "#FFFFFF";

    // Avatar frame (+ optional centered initial).
    let mut avatar_children = Vec::new();
    if let Some(initial) = args.get("initial").map(String::as_str) {
        if !initial.is_empty() {
            avatar_children.push(json!({
                "id": next_id("profile_header_initial"),
                "type": "text",
                "name": "Initial",
                "role": "profile-header-initial",
                "content": initial,
                "fontSize": initial_font_size,
                "fontWeight": 600,
                "fill": [{ "type": "solid", "color": initial_color }],
            }));
        }
    }

    let mut stack_children = vec![
        json!({
            "id": next_id("profile_header_avatar"),
            "type": "frame",
            "name": "Avatar",
            "role": "profile-header-avatar",
            "width": size,
            "height": size,
            "cornerRadius": size / 2.0,
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "fill": [{ "type": "solid", "color": avatar_bg }],
            "children": avatar_children,
        }),
        json!({
            "id": next_id("profile_header_name"),
            "type": "text",
            "name": "Name",
            "role": "profile-header-name",
            "content": name,
            "fontSize": 22,
            "fontWeight": 700,
            "fill": [{ "type": "solid", "color": name_color }],
        }),
    ];

    if let Some(handle) = args.get("handle").map(String::as_str) {
        if !handle.is_empty() {
            stack_children.push(json!({
                "id": next_id("profile_header_handle"),
                "type": "text",
                "name": "Handle",
                "role": "profile-header-handle",
                "content": handle,
                "fontSize": 14,
                "fontWeight": 400,
                "fill": [{ "type": "solid", "color": handle_color }],
            }));
        }
    }
    if let Some(bio) = args.get("bio").map(String::as_str) {
        if !bio.is_empty() {
            stack_children.push(json!({
                "id": next_id("profile_header_bio"),
                "type": "text",
                "name": "Bio",
                "role": "profile-header-bio",
                "content": bio,
                "fontSize": 14,
                "fontWeight": 400,
                "lineHeight": 1.5,
                "fill": [{ "type": "solid", "color": bio_color }],
                "width": "fill_container",
                "textGrowth": "fixed-width",
            }));
        }
    }

    Ok(json!({
        "id": next_id("profile_header_root"),
        "type": "frame",
        "name": "Profile Header",
        "role": "profile-header",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "alignItems": "center",
        "gap": 12,
        "padding": [24, 16],
        "children": stack_children,
    }))
}

// ===== add_metric_row =====

/// `add_metric_row_v0` / `add_metric_row_v1` — horizontal scroll row of
/// metric tiles (small label + big value + optional icon). Ports
/// `pen-core/element-builders/metric-row.ts` + the shared
/// `helpers.ts::buildScrollWrapper`. v0 emits NO hardcoded colors, so
/// v1 (light/dark/system) is byte-identical — a single builder serves
/// both and `theme_aware` is ignored.
fn build_metric_row(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    // TS `buildMetricRow` reads item.label/item.value directly (missing →
    // empty), never throwing on item fields. Validate only the primary
    // `label`, matching TS runtime (don't over-reject a missing value).
    let items = parse_object_items(args, "items", &["label"], "items[].label is required")?;
    let tile_width = number_arg(args, "tile_width", 120.0, 1.0);
    let gap = number_arg(args, "gap", 12.0, 0.0);

    let tiles: Vec<Value> = items
        .iter()
        .map(|item| build_metric_tile(item, tile_width))
        .collect();

    // buildScrollWrapper({ rowName: 'Metric Row', innerChildren: tiles, gap })
    Ok(json!({
        "id": next_id("metric_row_root"),
        "type": "frame",
        "name": "Metric Row",
        "role": "scroll-row-wrapper",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "clipContent": true,
        "children": [{
            "id": next_id("metric_row_scroll_inner"),
            "type": "frame",
            "name": "Scroll Inner Row",
            "role": "scroll-row",
            "width": "fit_content",
            "height": "fit_content",
            "layout": "horizontal",
            "gap": gap,
            "padding": [0, 20],
            "children": tiles,
        }],
    }))
}

/// One metric tile: tile_width×100 frame, cornerRadius 16, padding 16,
/// vertical, gap 4. Optional icon (20×20, no role/fill), label (12/500
/// body), value (28/700 heading) — both text nodes fill_container.
fn build_metric_tile(item: &Value, tile_width: f64) -> Value {
    let label = string_field(item, "label").unwrap_or("");
    let value = string_field(item, "value").unwrap_or("");

    let mut children = Vec::new();
    if let Some(icon) = string_field(item, "icon") {
        if !icon.is_empty() {
            // Plain lucide icon_font (no role / no fill) — matches TS verbatim.
            children.push(icon_node("Icon", icon, 20, 20));
        }
    }
    children.push(json!({
        "id": next_id("metric_tile_label"),
        "type": "text",
        "name": "Label",
        "role": "body",
        "content": label,
        "fontSize": 12,
        "fontWeight": 500,
        "width": "fill_container",
    }));
    children.push(json!({
        "id": next_id("metric_tile_value"),
        "type": "text",
        "name": "Value",
        "role": "heading",
        "content": value,
        "fontSize": 28,
        "fontWeight": 700,
        "width": "fill_container",
    }));

    json!({
        "id": next_id("metric_tile"),
        "type": "frame",
        "name": "Metric Tile",
        "role": "metric-tile",
        "width": tile_width,
        "height": 100,
        "cornerRadius": 16,
        "padding": 16,
        "layout": "vertical",
        "gap": 4,
        "children": children,
    })
}

// ===== add_metric_comparison =====

/// `add_metric_comparison_v0` / `add_metric_comparison_v1` — KPI with
/// trend indicator: small label above a big value, optional arrow +
/// percent change. Ports `pen-core/element-builders/metric-comparison.ts`
/// (v0) and `metric-comparison-v1.ts` (theme-aware). Light mode is
/// byte-parity with v0.
fn build_metric_comparison(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let value = required(args, "value")?;
    let change = opt(args, "change").filter(|c| !c.is_empty());
    // trend defaults to 'flat' (TS: change ? 'flat' : 'flat' === always 'flat').
    let trend = opt(args, "trend").unwrap_or("flat");

    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let is_light = !theme_aware || theme == "light";

    // label color: light #64748B, dark textMuted, system $color-text-muted.
    let label_color: &str = if is_light {
        "#64748B"
    } else {
        match theme {
            "dark" => "#94A3B8",
            _ => "$color-text-muted",
        }
    };
    let trend_color = metric_trend_color(trend, is_light, theme);
    let trend_icon = match trend {
        "up" => "trending-up",
        "down" => "trending-down",
        _ => "minus",
    };

    // Value Row children: value text + optional change cluster.
    let mut row_children = vec![json!({
        "id": next_id("metric_comparison_value"),
        "type": "text",
        "name": "Value",
        "role": "metric-comparison-value",
        "content": value,
        "fontSize": 28,
        "fontWeight": 700,
    })];
    if let Some(change) = change {
        row_children.push(json!({
            "id": next_id("metric_comparison_change"),
            "type": "frame",
            "name": "Change",
            "role": "metric-comparison-change",
            "width": "fit_content",
            "height": "fit_content",
            "layout": "horizontal",
            "alignItems": "center",
            "gap": 2,
            "children": [
                {
                    "id": next_id("metric_comparison_arrow"),
                    "type": "icon_font",
                    "name": "Trend Arrow",
                    "role": "metric-comparison-arrow",
                    "iconFontName": trend_icon,
                    "iconFontFamily": "lucide",
                    "width": 14,
                    "height": 14,
                    "fill": [{ "type": "solid", "color": trend_color }],
                },
                {
                    "id": next_id("metric_comparison_change_text"),
                    "type": "text",
                    "name": "Change Amount",
                    "role": "metric-comparison-change-text",
                    "content": change,
                    "fontSize": 12,
                    "fontWeight": 500,
                    "fill": [{ "type": "solid", "color": trend_color }],
                }
            ],
        }));
    }

    Ok(json!({
        "id": next_id("metric_comparison_root"),
        "type": "frame",
        "name": "Metric Comparison",
        "role": "metric-comparison",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 4,
        "children": [
            {
                "id": next_id("metric_comparison_label"),
                "type": "text",
                "name": "Label",
                "role": "metric-comparison-label",
                "content": label,
                "fontSize": 12,
                "fontWeight": 500,
                "fill": [{ "type": "solid", "color": label_color }],
            },
            {
                "id": next_id("metric_comparison_row"),
                "type": "frame",
                "name": "Value Row",
                "role": "metric-comparison-row",
                "width": "fit_content",
                "height": "fit_content",
                "layout": "horizontal",
                "alignItems": "baseline",
                "gap": 8,
                "children": row_children,
            }
        ],
    }))
}

/// Trend color: up=success, down=destructive, flat/default=muted.
/// Light hex matches v0; dark/system from the semantic palette.
fn metric_trend_color(trend: &str, is_light: bool, theme: &str) -> &'static str {
    match trend {
        "up" => {
            if is_light {
                "#10B981"
            } else if theme == "dark" {
                "#34D399"
            } else {
                "$color-success"
            }
        }
        "down" => {
            if is_light {
                "#EF4444"
            } else if theme == "dark" {
                "#F87171"
            } else {
                "$color-destructive"
            }
        }
        // flat / unknown
        _ => {
            if is_light {
                "#64748B"
            } else if theme == "dark" {
                "#94A3B8"
            } else {
                "$color-text-muted"
            }
        }
    }
}
