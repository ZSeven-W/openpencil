//! Ported TS element builders (cards-a). Real role-tagged subtrees
//! for kinds that previously fell to the generic kit placeholder.
#![allow(clippy::result_large_err)]

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::element_ported_helpers::*;
use crate::ToolOutcome;

pub(crate) fn ported_cards_a_alias_node_value(
    tool: &str,
    args: &std::collections::BTreeMap<String, String>,
) -> Result<Option<serde_json::Value>, crate::ToolOutcome> {
    let value = match tool {
        "add_stat_card_v0" => build_stat_card(args, false)?,
        "add_stat_card_v1" => build_stat_card(args, true)?,
        "add_stat_grid_v0" => build_stat_grid(args, false)?,
        "add_stat_grid_v1" => build_stat_grid(args, true)?,
        "add_step_card_v0" => build_step_card(args, false)?,
        "add_step_card_v1" => build_step_card(args, true)?,
        "add_user_card_v0" => build_user_card(args, false)?,
        "add_user_card_v1" => build_user_card(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

// ===== add_stat_card =====

/// Big-number stat card — label above, big metric, optional delta below,
/// optional icon in the header corner. Mirrors `buildStatCard` /
/// `buildStatCardV1`. Delta tone colors are status semantics — they stay
/// hardcoded in every theme.
fn build_stat_card(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let value = required(args, "value")?;
    let width = number_arg(args, "width", 240.0, 160.0).floor();
    let corner_radius = number_arg(args, "corner_radius", 16.0, 0.0).floor();
    let trend = opt(args, "trend").unwrap_or("flat");
    let delta_color = match trend {
        "up" => "#10B981",
        "down" => "#EF4444",
        _ => "#64748B",
    };
    let c = theme_colors(theme_aware, args);

    let mut header_children = vec![json!({
        "id": next_id("stat_card_label"),
        "type": "text",
        "name": "Label",
        "role": "stat-card-label",
        "content": label.to_uppercase(),
        "fontSize": 12,
        "fontWeight": 500,
        "letterSpacing": 1,
        "fill": [{ "type": "solid", "color": c.text_muted }],
    })];
    if let Some(icon) = opt(args, "icon").filter(|s| !s.is_empty()) {
        let mut icon = icon_node("Icon", icon, 18, 18);
        icon["role"] = json!("stat-card-icon");
        icon["fill"] = json!([{ "type": "solid", "color": c.text_subtle }]);
        header_children.push(icon);
    }

    let mut card_children = vec![
        json!({
            "id": next_id("stat_card_header"),
            "type": "frame",
            "name": "Header",
            "role": "stat-card-header",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "space-between",
            "children": header_children,
        }),
        json!({
            "id": next_id("stat_card_value"),
            "type": "text",
            "name": "Value",
            "role": "stat-card-value",
            "content": value,
            "fontSize": 32,
            "fontWeight": 700,
            "lineHeight": 1.1,
            "fill": [{ "type": "solid", "color": c.text_primary }],
        }),
    ];

    if let Some(delta) = opt(args, "delta").filter(|s| !s.is_empty()) {
        card_children.push(json!({
            "id": next_id("stat_card_delta"),
            "type": "text",
            "name": "Delta",
            "role": "stat-card-delta",
            "content": delta,
            "fontSize": 12,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": delta_color }],
        }));
    }

    Ok(json!({
        "id": next_id("stat_card_root"),
        "type": "frame",
        "name": "Stat Card",
        "role": "stat-card",
        "width": width,
        "height": "fit_content",
        "cornerRadius": corner_radius,
        "layout": "vertical",
        "gap": 12,
        "padding": 20,
        "fill": [{ "type": "solid", "color": c.surface }],
        "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": c.border }] },
        "children": card_children,
    }))
}

// ===== add_stat_grid =====

/// Non-scrolling stat grid — 2-5 mini-stats share the row via
/// `fill_container` cells. Mirrors `buildStatGrid` / `buildStatGridV1`.
/// v0 has no hardcoded fills (text inherits canvas theme), so the v1
/// variant is byte-identical across all theme modes — `theme_aware` is
/// accepted for API consistency but does not change the output.
fn build_stat_grid(
    args: &BTreeMap<String, String>,
    _theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    // TS `buildStatGrid` reads item.value/item.label directly (missing →
    // empty), never throwing on item fields. Validate only the primary
    // `label` so we don't over-reject input TS would render.
    let items = parse_object_items(args, "items", &["label"], "items[].label is required")?;
    let gap = number_arg(args, "gap", 16.0, 0.0);

    let mut cells = Vec::with_capacity(items.len());
    for item in &items {
        let value = string_field(item, "value").unwrap_or("");
        let label = string_field(item, "label").unwrap_or("");

        let mut children = Vec::new();
        if let Some(icon) = string_field(item, "icon").filter(|s| !s.is_empty()) {
            children.push(icon_node("Icon", icon, 20, 20));
        }
        children.push(json!({
            "id": next_id("stat_cell_value"),
            "type": "text",
            "name": "Value",
            "role": "heading",
            "content": value,
            "fontSize": 24,
            "fontWeight": 700,
            "width": "fill_container",
        }));
        children.push(json!({
            "id": next_id("stat_cell_label"),
            "type": "text",
            "name": "Label",
            "role": "body",
            "content": label,
            "fontSize": 12,
            "fontWeight": 500,
            "width": "fill_container",
        }));
        cells.push(json!({
            "id": next_id("stat_cell"),
            "type": "frame",
            "name": "Stat Cell",
            "role": "stat-cell",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "alignItems": "center",
            "gap": 4,
            "children": children,
        }));
    }

    Ok(json!({
        "id": next_id("stat_grid_root"),
        "type": "frame",
        "name": "Stat Grid",
        "role": "stat-grid",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "gap": gap,
        "alignItems": "center",
        "justifyContent": "space_between",
        "children": cells,
    }))
}

// ===== add_step_card =====

/// Onboarding / how-it-works step card — numbered circle (or check when
/// completed) on the left, title over description on the right. Mirrors
/// `buildStepCard` / `buildStepCardV1`. Accent (`#2563EB`) and the
/// on-accent check white stay hardcoded in every theme.
fn build_step_card(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    const ACCENT: &str = "#2563EB";
    let number_text = required(args, "number")?;
    let title = required(args, "title")?;
    let description = required(args, "description")?;
    let completed = bool_arg(args, "completed");
    let c = theme_colors(theme_aware, args);

    // Title text: light → slate-900, dark/system → textPrimary.
    let title_color = if theme_aware {
        c.text_primary
    } else {
        "#0F172A"
    };
    // Description text: light → slate-600 (#475569 literal), dark/system → textMuted.
    let desc_color = match (theme_aware, opt(args, "theme").unwrap_or("light")) {
        (true, "dark") | (true, "system") => c.text_muted,
        _ => "#475569",
    };
    // Incomplete circle bg: light → #FFFFFF, dark/system → surface.
    let ring_bg = if theme_aware { c.surface } else { "#FFFFFF" };

    let circle_child = if completed {
        json!({
            "id": next_id("step_card_check"),
            "type": "icon_font",
            "name": "Check",
            "role": "step-card-check",
            "iconFontName": "check",
            "iconFontFamily": "lucide",
            "width": 18,
            "height": 18,
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        })
    } else {
        json!({
            "id": next_id("step_card_number"),
            "type": "text",
            "name": "Number",
            "role": "step-card-number",
            "content": number_text,
            "fontSize": 15,
            "fontWeight": 700,
            "fill": [{ "type": "solid", "color": ACCENT }],
        })
    };

    let mut circle = json!({
        "id": next_id("step_card_circle"),
        "type": "frame",
        "name": "Number Circle",
        "role": "step-card-circle",
        "width": 36,
        "height": 36,
        "cornerRadius": 18,
        "fill": [{ "type": "solid", "color": if completed { ACCENT } else { ring_bg } }],
        "clipContent": true,
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "children": [circle_child],
    });
    if !completed {
        circle["stroke"] = json!({
            "thickness": 2,
            "fill": [{ "type": "solid", "color": ACCENT }],
        });
    }

    Ok(json!({
        "id": next_id("step_card_root"),
        "type": "frame",
        "name": "Step Card",
        "role": "step-card",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "flex-start",
        "gap": 14,
        "padding": [4, 0],
        "children": [
            circle,
            {
                "id": next_id("step_card_body"),
                "type": "frame",
                "name": "Body",
                "role": "step-card-body",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "gap": 4,
                "padding": [4, 0, 0, 0],
                "children": [
                    {
                        "id": next_id("step_card_title"),
                        "type": "text",
                        "name": "Title",
                        "role": "step-card-title",
                        "content": title,
                        "fontSize": 16,
                        "fontWeight": 600,
                        "width": "fill_container",
                        "textGrowth": "fixed-width",
                        "fill": [{ "type": "solid", "color": title_color }],
                    },
                    {
                        "id": next_id("step_card_description"),
                        "type": "text",
                        "name": "Description",
                        "role": "step-card-description",
                        "content": description,
                        "fontSize": 14,
                        "fontWeight": 400,
                        "lineHeight": 1.5,
                        "width": "fill_container",
                        "textGrowth": "fixed-width",
                        "fill": [{ "type": "solid", "color": desc_color }],
                    }
                ],
            }
        ],
    }))
}

// ===== add_user_card =====

/// Profile / contact card — circular avatar + (name + optional role)
/// text stack. Mirrors `buildUserCard` / `buildUserCardV1`. Avatar bg
/// (`#3B82F6`) and the initial white stay hardcoded in every theme.
fn build_user_card(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let name = required(args, "name")?;
    // size = clamp(floor(avatar_size ?? 48), 32, 96).
    let size = number_arg(args, "avatar_size", 48.0, 32.0)
        .floor()
        .clamp(32.0, 96.0);
    let initial_font_size = (size * 0.4).round().max(12.0);

    // Name: light → slate-900, dark/system → textPrimary.
    let name_color = if theme_aware {
        theme_colors(theme_aware, args).text_primary
    } else {
        "#0F172A"
    };
    // Role: light → slate-500, dark/system → textMuted.
    let role_color = if theme_aware {
        theme_colors(theme_aware, args).text_muted
    } else {
        "#64748B"
    };

    let mut stack_children = vec![json!({
        "id": next_id("user_card_name"),
        "type": "text",
        "name": "Name",
        "role": "user-card-name",
        "content": name,
        "fontSize": 15,
        "fontWeight": 600,
        "fill": [{ "type": "solid", "color": name_color }],
    })];
    if let Some(role) = opt(args, "role").filter(|s| !s.is_empty()) {
        stack_children.push(json!({
            "id": next_id("user_card_role"),
            "type": "text",
            "name": "Role",
            "role": "user-card-role",
            "content": role,
            "fontSize": 13,
            "fontWeight": 400,
            "fill": [{ "type": "solid", "color": role_color }],
        }));
    }

    let mut avatar_children = Vec::new();
    if let Some(initial) = opt(args, "initial").filter(|s| !s.is_empty()) {
        avatar_children.push(json!({
            "id": next_id("user_card_initial"),
            "type": "text",
            "name": "Initial",
            "role": "user-card-initial",
            "content": initial,
            "fontSize": initial_font_size,
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        }));
    }

    Ok(json!({
        "id": next_id("user_card_root"),
        "type": "frame",
        "name": "User Card",
        "role": "user-card",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 12,
        "children": [
            {
                "id": next_id("user_card_avatar"),
                "type": "frame",
                "name": "Avatar",
                "role": "user-card-avatar",
                "width": size,
                "height": size,
                "cornerRadius": size / 2.0,
                "layout": "horizontal",
                "alignItems": "center",
                "justifyContent": "center",
                "fill": [{ "type": "solid", "color": "#3B82F6" }],
                "children": avatar_children,
            },
            {
                "id": next_id("user_card_text"),
                "type": "frame",
                "name": "Text Stack",
                "role": "user-card-text",
                "width": "fit_content",
                "height": "fit_content",
                "layout": "vertical",
                "gap": 2,
                "children": stack_children,
            }
        ],
    }))
}

// ===== _shared_helpers_and_wiring =====

// ── Module header (top of crates/op-mcp/src/element_ported_alias_builders.rs) ──

/// Theme palette slots used by the ported v1 builders. light == v0
/// literals (byte parity); dark == semantic-palette dark hex; system ==
/// `$color-*` refs.
struct ThemeColors {
    surface: &'static str,
    border: &'static str,
    text_primary: &'static str,
    text_muted: &'static str,
    text_subtle: &'static str,
}

fn theme_colors(theme_aware: bool, args: &BTreeMap<String, String>) -> ThemeColors {
    let theme = if theme_aware {
        args.get("theme").map(String::as_str).unwrap_or("light")
    } else {
        "light"
    };
    match theme {
        "dark" => ThemeColors {
            surface: "#1E293B",
            border: "#334155",
            text_primary: "#F1F5F9",
            text_muted: "#94A3B8",
            text_subtle: "#64748B",
        },
        "system" => ThemeColors {
            surface: "$color-surface",
            border: "$color-border",
            text_primary: "$color-text-primary",
            text_muted: "$color-text-muted",
            text_subtle: "$color-text-subtle",
        },
        _ => ThemeColors {
            surface: "#FFFFFF",
            border: "#E2E8F0",
            text_primary: "#0F172A",
            text_muted: "#64748B",
            text_subtle: "#94A3B8",
        },
    }
}
