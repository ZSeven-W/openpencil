//! Ported TS element builders (nav). Real role-tagged subtrees
//! for kinds that previously fell to the generic kit placeholder.
#![allow(clippy::result_large_err)]

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::element_ported_helpers::*;
use crate::{ToolErrorCode, ToolOutcome};

pub(crate) fn ported_nav_alias_node_value(
    tool: &str,
    args: &std::collections::BTreeMap<String, String>,
) -> Result<Option<serde_json::Value>, crate::ToolOutcome> {
    let value = match tool {
        "add_tabs_v0" => build_tabs(args, false)?,
        "add_tabs_v1" => build_tabs(args, true)?,
        "add_toolbar_v0" => build_toolbar(args, false)?,
        "add_toolbar_v1" => build_toolbar(args, true)?,
        "add_top_nav_bar_v0" => build_top_nav_bar(args, false)?,
        "add_top_nav_bar_v1" => build_top_nav_bar(args, true)?,
        "add_sidebar_nav_v0" => build_sidebar_nav(args, false)?,
        "add_sidebar_nav_v1" => build_sidebar_nav(args, true)?,
        "add_pagination_v0" => build_pagination(args, false)?,
        "add_pagination_v1" => build_pagination(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

// ===== add_tabs =====

/// Ports `buildTabs` / `buildTabsV1` (tabs.ts + tabs-v1.ts).
///
/// v0 and v1 produce IDENTICAL trees: the only color is the #2563EB
/// accent underline, which per spec §3.4 is a brand token kept hardcoded
/// across all theme modes. `theme_aware` is therefore ignored.
fn build_tabs(args: &BTreeMap<String, String>, _theme_aware: bool) -> Result<Value, ToolOutcome> {
    let items = parse_object_items(args, "items", &["label"], "items[].label is required")?;

    let tabs: Vec<Value> = items
        .iter()
        .map(|item| {
            let label = string_field(item, "label").unwrap_or("");
            let active = bool_field(item, "active");

            let inner = json!({
                "id": next_id("tabs_tab_content"),
                "type": "frame",
                "name": "Tab Content",
                "width": "fill_container",
                "padding": [12, 16],
                "layout": "horizontal",
                "alignItems": "center",
                "justifyContent": "center",
                "children": [{
                    "id": next_id("tabs_label"),
                    "type": "text",
                    "name": "Label",
                    "role": "label",
                    "content": label,
                    "fontSize": 14,
                    "fontWeight": if active { 600 } else { 500 },
                }],
            });

            let mut children = vec![inner];
            if active {
                children.push(json!({
                    "id": next_id("tabs_underline"),
                    "type": "rectangle",
                    "name": "Underline",
                    "role": "tab-underline",
                    "width": "fill_container",
                    "height": 2,
                    "fill": [{ "type": "solid", "color": "#2563EB" }],
                }));
            }

            json!({
                "id": next_id("tabs_tab"),
                "type": "frame",
                "name": format!("Tab ({label})"),
                "role": if active { "tab-active" } else { "tab" },
                "width": "fill_container",
                "layout": "vertical",
                "alignItems": "stretch",
                "children": children,
            })
        })
        .collect();

    Ok(json!({
        "id": next_id("tabs_root"),
        "type": "frame",
        "name": "Tabs",
        "role": "tabs",
        "width": "fill_container",
        "layout": "horizontal",
        "gap": 4,
        "alignItems": "stretch",
        "children": tabs,
    }))
}

// ===== add_toolbar =====

/// Ports `buildToolbar` / `buildToolbarV1` (toolbar.ts + toolbar-v1.ts).
///
/// Desktop horizontal row of 36x36 icon buttons with optional vertical
/// dividers between groups. v1 swaps 6 color slots per theme; light mode
/// is byte-equal to v0.
///
/// NOTE: toolbar-v1 coerces an empty/missing `items` to a 3-item default
/// (`bold`, `italic` divider_after, `underline`) instead of erroring. v0
/// throws on a missing array. We mirror that: v0 (theme_aware=false)
/// requires `items`; v1 (theme_aware=true) falls back to the default set.
fn build_toolbar(args: &BTreeMap<String, String>, theme_aware: bool) -> Result<Value, ToolOutcome> {
    let items = if theme_aware {
        // v1: coerceNonEmptyArray fallback.
        match maybe_object_items(args, "items")? {
            Some(items) if !items.is_empty() => items,
            _ => vec![
                json!({ "icon": "bold" }),
                json!({ "icon": "italic", "divider_after": true }),
                json!({ "icon": "underline" }),
            ],
        }
    } else {
        // v0: required array, each item must carry a non-empty `icon`.
        parse_object_items(args, "items", &["icon"], "items[].icon is required")?
    };

    let theme = opt(args, "theme").unwrap_or("light");
    let is_light = theme != "dark" && theme != "system";
    // Resolved theme color slots (light literals == v0 for byte-parity).
    let surface = if is_light {
        "#FFFFFF"
    } else {
        tk(theme, "surface")
    };
    let border = if is_light {
        "#E2E8F0"
    } else {
        tk(theme, "border")
    };
    let active_bg = if is_light {
        "#F1F5F9"
    } else {
        tk(theme, "surface2")
    };
    let active_icon = if is_light {
        "#0F172A"
    } else {
        tk(theme, "text_primary")
    };
    let inactive_icon = if is_light {
        "#475569"
    } else {
        tk(theme, "text_muted")
    };
    let divider = if is_light {
        "#E2E8F0"
    } else {
        tk(theme, "border")
    };

    let len = items.len();
    let mut children = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let icon = string_field(item, "icon").unwrap_or("");
        let active = bool_field(item, "active");

        let mut tool = json!({
            "id": next_id("toolbar_item"),
            "type": "frame",
            "name": format!("Tool ({icon})"),
            "role": if active { "toolbar-item-active" } else { "toolbar-item" },
            "width": 36,
            "height": 36,
            "cornerRadius": 6,
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "children": [{
                "id": next_id("toolbar_icon"),
                "type": "icon_font",
                "name": "Icon",
                "iconFontName": icon,
                "iconFontFamily": "lucide",
                "width": 18,
                "height": 18,
                "fill": [{ "type": "solid", "color": if active { active_icon } else { inactive_icon } }],
            }],
        });
        if active {
            tool["fill"] = json!([{ "type": "solid", "color": active_bg }]);
        }
        children.push(tool);

        if bool_field(item, "divider_after") && i < len - 1 {
            children.push(json!({
                "id": next_id("toolbar_divider"),
                "type": "frame",
                "name": "Toolbar Divider",
                "role": "toolbar-divider",
                "width": 1,
                "height": 20,
                "fill": [{ "type": "solid", "color": divider }],
                "children": [],
            }));
        }
    }

    Ok(json!({
        "id": next_id("toolbar_root"),
        "type": "frame",
        "name": "Toolbar",
        "role": "toolbar",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 4,
        "padding": [4, 6],
        "cornerRadius": 8,
        "fill": [{ "type": "solid", "color": surface }],
        "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": border }] },
        "children": children,
    }))
}

// ===== add_top_nav_bar =====

/// Ports `buildTopNavBar` / `buildTopNavBarV1`
/// (top-nav-bar.ts + top-nav-bar-v1.ts).
///
/// v0 and v1 produce IDENTICAL trees: there are no hardcoded surface
/// colors in v0 (the bar has no fill/border; icons + title inherit the
/// canvas default), so every theme mode is the same. `theme_aware` is
/// ignored. Empty leading/trailing slots become same-footprint spacers
/// so the centered title stays centered.
fn build_top_nav_bar(
    args: &BTreeMap<String, String>,
    _theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let title = required(args, "title")?;
    let height = number_arg(args, "height", 56.0, 0.0);
    let leading = opt(args, "leading_icon").filter(|s| !s.is_empty());
    let trailing = opt(args, "trailing_icon").filter(|s| !s.is_empty());

    Ok(json!({
        "id": next_id("top_nav_bar_root"),
        "type": "frame",
        "name": "Top Nav Bar",
        "role": "top-nav-bar",
        "width": "fill_container",
        "height": height,
        "layout": "horizontal",
        "justifyContent": "space_between",
        "alignItems": "center",
        "padding": [0, 16],
        "children": [
            top_nav_icon_slot(leading, "leading"),
            {
                "id": next_id("top_nav_bar_title"),
                "type": "text",
                "name": "Title",
                "role": "heading",
                "content": title,
                "fontSize": 17,
                "fontWeight": 600,
            },
            top_nav_icon_slot(trailing, "trailing"),
        ],
    }))
}

/// Mirrors top-nav-bar.ts `buildIconSlot`: a 44x44 icon button, or a
/// same-footprint spacer when no icon is supplied.
fn top_nav_icon_slot(icon: Option<&str>, position: &str) -> Value {
    match icon {
        None => json!({
            "id": next_id("top_nav_spacer"),
            "type": "frame",
            "name": format!("{position} Spacer"),
            "role": "nav-spacer",
            "width": 44,
            "height": 44,
            "layout": "none",
            "children": [],
        }),
        Some(icon) => json!({
            "id": next_id("top_nav_icon_button"),
            "type": "frame",
            "name": format!("{position} Icon Button"),
            "role": "icon-button",
            "width": 44,
            "height": 44,
            "layout": "horizontal",
            "justifyContent": "center",
            "alignItems": "center",
            "cornerRadius": 8,
            "children": [{
                "id": next_id("top_nav_icon"),
                "type": "icon_font",
                "name": "Icon",
                "iconFontName": icon,
                "iconFontFamily": "lucide",
                "width": 24,
                "height": 24,
            }],
        }),
    }
}

// ===== add_sidebar_nav =====

/// Ports `buildSidebarNav` / `buildSidebarNavV1`
/// (sidebar-nav.ts + sidebar-nav-v1.ts).
///
/// Persistent vertical left rail: optional brand/title row + N icon+label
/// rows. Active row gets a pill bg + bolder darker label. v1 swaps 5 color
/// slots per theme (light == v0) and additionally runs the item icon
/// through `coerceNavTabIcon` to correct common wrong-glyph picks.
fn build_sidebar_nav(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let items = parse_object_items(args, "items", &["label"], "items[].label is required")?;
    // width: Math.min(320, Math.max(180, Math.floor(width ?? 240)))
    let width = number_arg(args, "width", 240.0, f64::NEG_INFINITY)
        .floor()
        .clamp(180.0, 320.0);
    let title = opt(args, "title").filter(|s| !s.is_empty());

    let theme = opt(args, "theme").unwrap_or("light");
    let is_light = !theme_aware || (theme != "dark" && theme != "system");
    let sidebar_bg = if is_light {
        "#FFFFFF"
    } else {
        tk(theme, "surface")
    };
    let title_color = if is_light {
        "#0F172A"
    } else {
        tk(theme, "text_primary")
    };
    let active_label = if is_light {
        "#0F172A"
    } else {
        tk(theme, "text_primary")
    };
    let active_item_bg = if is_light {
        "#F1F5F9"
    } else {
        tk(theme, "surface2")
    };
    let inactive_label = if is_light {
        "#475569"
    } else {
        tk(theme, "text_muted")
    };

    let mut children = Vec::new();
    if let Some(title) = title {
        children.push(json!({
            "id": next_id("sidebar_nav_title"),
            "type": "frame",
            "name": "Sidebar Title",
            "role": "sidebar-nav-title",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "alignItems": "center",
            "padding": [8, 12, 24, 12],
            "children": [{
                "id": next_id("sidebar_nav_title_text"),
                "type": "text",
                "name": "Title",
                "role": "sidebar-nav-title-text",
                "content": title,
                "fontSize": 16,
                "fontWeight": 700,
                "fill": [{ "type": "solid", "color": title_color }],
            }],
        }));
    }

    for item in &items {
        let label = string_field(item, "label").unwrap_or("");
        let raw_icon = string_field(item, "icon").unwrap_or("");
        // v1 corrects common wrong-glyph picks (Cart->shopping-cart, etc).
        let icon = if theme_aware {
            coerce_nav_tab_icon(label, raw_icon)
        } else {
            raw_icon
        };
        let active = bool_field(item, "active");

        let mut node = json!({
            "id": next_id("sidebar_nav_item"),
            "type": "frame",
            "name": format!("Item ({label})"),
            "role": if active { "sidebar-nav-item-active" } else { "sidebar-nav-item" },
            "width": "fill_container",
            "height": 40,
            "cornerRadius": 8,
            "layout": "horizontal",
            "alignItems": "center",
            "gap": 12,
            "padding": [0, 12],
            "children": [
                {
                    "id": next_id("sidebar_nav_icon"),
                    "type": "icon_font",
                    "name": "Icon",
                    "role": "sidebar-nav-icon",
                    "iconFontName": icon,
                    "iconFontFamily": "lucide",
                    "width": 18,
                    "height": 18,
                },
                {
                    "id": next_id("sidebar_nav_label"),
                    "type": "text",
                    "name": "Label",
                    "role": "sidebar-nav-label",
                    "content": label,
                    "fontSize": 14,
                    "fontWeight": if active { 600 } else { 500 },
                    "fill": [{ "type": "solid", "color": if active { active_label } else { inactive_label } }],
                },
            ],
        });
        if active {
            node["fill"] = json!([{ "type": "solid", "color": active_item_bg }]);
        }
        children.push(node);
    }

    Ok(json!({
        "id": next_id("sidebar_nav_root"),
        "type": "frame",
        "name": "Sidebar Nav",
        "role": "sidebar-nav",
        "width": width,
        "height": "fill_container",
        "layout": "vertical",
        "gap": 4,
        "padding": [16, 12],
        "fill": [{ "type": "solid", "color": sidebar_bg }],
        "children": children,
    }))
}

// ===== add_pagination =====

/// Ports `buildPagination` / `buildPaginationV1`
/// (pagination.ts + pagination-v1.ts).
///
/// Row of page-number pills flanked by optional prev/next arrows. The
/// current page is a filled pill (accent bg, white text); other pages are
/// ghost. Google-style ellipses collapse the out-of-window pages. v1 swaps
/// arrow / inactive-text / ellipsis colors per theme; the active pill bg
/// (accent_color) and white active text are brand-invariant.
fn build_pagination(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    // total = max(1, floor(total)); required.
    let total_raw = required(args, "total")?
        .parse::<f64>()
        .map_err(|e| ToolOutcome::Err(ToolErrorCode::InvalidArgument, format!("total: {e}")))?;
    let total = (total_raw.floor() as i64).max(1);
    // current = clamp(floor(current ?? 1), 1, total)
    let current =
        (number_arg(args, "current", 1.0, f64::NEG_INFINITY).floor() as i64).clamp(1, total);
    // siblings = max(0, floor(siblings ?? 1))
    let siblings = (number_arg(args, "siblings", 1.0, f64::NEG_INFINITY).floor() as i64).max(0);
    // show_arrows: default true; only an explicit "false" disables it
    // (TS: params.show_arrows !== false).
    let show_arrows = opt(args, "show_arrows") != Some("false");
    let accent = opt(args, "accent_color")
        .filter(|s| !s.is_empty())
        .unwrap_or("#0F172A");

    let theme = opt(args, "theme").unwrap_or("light");
    let is_light = !theme_aware || (theme != "dark" && theme != "system");
    let arrow_color = if is_light {
        "#334155"
    } else {
        tk(theme, "text_body")
    };
    let inactive_color = if is_light {
        "#334155"
    } else {
        tk(theme, "text_body")
    };
    let ellipsis_color = if is_light {
        "#64748B"
    } else {
        tk(theme, "text_muted")
    };
    let active_pill_text = "#FFFFFF";

    // Visible-page window: always 1, total, current, plus the
    // [current-siblings, current+siblings] band; dedup + sort.
    let mut page_set: std::collections::BTreeSet<i64> = [1, total, current].into_iter().collect();
    for i in 1..=siblings {
        page_set.insert(current - i);
        page_set.insert(current + i);
    }
    let sorted: Vec<i64> = page_set
        .into_iter()
        .filter(|&p| p >= 1 && p <= total)
        .collect();

    // Entries with ellipsis fillers where there's a gap > 1.
    enum Entry {
        Page(i64),
        Ellipsis,
    }
    let mut entries = Vec::new();
    for (i, &n) in sorted.iter().enumerate() {
        entries.push(Entry::Page(n));
        if let Some(&next) = sorted.get(i + 1) {
            if next > n + 1 {
                entries.push(Entry::Ellipsis);
            }
        }
    }

    let mut children = Vec::new();

    if show_arrows {
        children.push(json!({
            "id": next_id("pagination_prev"),
            "type": "frame",
            "name": "Prev",
            "role": "pagination-prev",
            "width": 32,
            "height": 32,
            "cornerRadius": 6,
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "fill": [],
            "children": [{
                "id": next_id("pagination_prev_icon"),
                "type": "icon_font",
                "name": "Prev Icon",
                "iconFontName": "chevron-left",
                "iconFontFamily": "lucide",
                "width": 16,
                "height": 16,
                "fill": [{ "type": "solid", "color": arrow_color }],
            }],
        }));
    }

    for entry in &entries {
        match entry {
            Entry::Ellipsis => children.push(json!({
                "id": next_id("pagination_ellipsis"),
                "type": "text",
                "name": "Ellipsis",
                "role": "pagination-ellipsis",
                "content": "\u{2026}",
                "fontSize": 13,
                "fontWeight": 400,
                "fill": [{ "type": "solid", "color": ellipsis_color }],
            })),
            Entry::Page(n) => {
                let is_active = *n == current;
                children.push(json!({
                    "id": next_id("pagination_page"),
                    "type": "frame",
                    "name": format!("Page {n}"),
                    "role": if is_active { "pagination-page-active" } else { "pagination-page" },
                    "width": 36,
                    "height": 32,
                    "cornerRadius": 6,
                    "layout": "horizontal",
                    "alignItems": "center",
                    "justifyContent": "center",
                    "fill": if is_active { json!([{ "type": "solid", "color": accent }]) } else { json!([]) },
                    "children": [{
                        "id": next_id("pagination_label"),
                        "type": "text",
                        "name": "Label",
                        "content": n.to_string(),
                        "fontSize": 13,
                        "fontWeight": if is_active { 600 } else { 400 },
                        "fill": [{ "type": "solid", "color": if is_active { active_pill_text } else { inactive_color } }],
                    }],
                }));
            }
        }
    }

    if show_arrows {
        children.push(json!({
            "id": next_id("pagination_next"),
            "type": "frame",
            "name": "Next",
            "role": "pagination-next",
            "width": 32,
            "height": 32,
            "cornerRadius": 6,
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "fill": [],
            "children": [{
                "id": next_id("pagination_next_icon"),
                "type": "icon_font",
                "name": "Next Icon",
                "iconFontName": "chevron-right",
                "iconFontFamily": "lucide",
                "width": 16,
                "height": 16,
                "fill": [{ "type": "solid", "color": arrow_color }],
            }],
        }));
    }

    Ok(json!({
        "id": next_id("pagination_root"),
        "type": "frame",
        "name": "Pagination",
        "role": "pagination",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 4,
        "children": children,
    }))
}

// ===== _shared_helpers =====

// ───────────────────────── shared helpers ─────────────────────────
// Used by build_toolbar, build_sidebar_nav, build_pagination (theme
// color resolution) and build_toolbar (non-erroring item parse).
// build_sidebar_nav additionally uses coerce_nav_tab_icon. These are
// the ONLY helpers this module defines beyond the contract helpers;
// the rest (next_id / required / opt / number_arg / bool_arg /
// parse_object_items / string_field / bool_field / icon_node) are
// assumed in scope per the module contract.

/// Resolve a v1 theme color slot to its concrete value for the given
/// theme mode. `theme` is only ever "dark" or "system" at the call site
/// (light callers short-circuit to v0 literals before calling `tk`).
/// Mirrors `resolveTheme(theme).colors.<slot>` from resolve-theme.ts:
/// dark → SEMANTIC_PALETTE dark hex; system → `$color-*` ref string.
fn tk(theme: &str, slot: &str) -> &'static str {
    let system = theme == "system";
    match slot {
        "surface" => {
            if system {
                "$color-surface"
            } else {
                "#1E293B"
            }
        }
        "surface2" => {
            if system {
                "$color-surface-2"
            } else {
                "#334155"
            }
        }
        "border" => {
            if system {
                "$color-border"
            } else {
                "#334155"
            }
        }
        "text_primary" => {
            if system {
                "$color-text-primary"
            } else {
                "#F1F5F9"
            }
        }
        "text_body" => {
            if system {
                "$color-text-body"
            } else {
                "#CBD5E1"
            }
        }
        "text_muted" => {
            if system {
                "$color-text-muted"
            } else {
                "#94A3B8"
            }
        }
        // Unknown slot: fall back to a visible debug ref rather than panic.
        _ => {
            if system {
                "$color-surface"
            } else {
                "#1E293B"
            }
        }
    }
}

/// Non-erroring variant of `parse_object_items` for the v1 toolbar
/// `coerceNonEmptyArray` path: returns Ok(None) when the arg is absent,
/// Ok(Some(items)) when present and a valid JSON array (no per-item
/// field validation — the caller supplies a default when empty).
fn maybe_object_items(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<Vec<Value>>, ToolOutcome> {
    let Some(raw) = args.get(key) else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON array: {e}"),
        )
    })?;
    match value {
        Value::Array(items) => Ok(Some(items)),
        // Non-array (incl. null): treat as "unset" so the v1 default kicks in.
        _ => Ok(None),
    }
}

/// Title -> canonical lucide icon swap, ported from coerce-params.ts
/// `coerceNavTabIcon`. Only swaps when the title has a canonical icon AND
/// the emitted icon is a known wrong-glyph alt for it; otherwise the
/// emitted icon passes through untouched.
fn coerce_nav_tab_icon<'a>(title: &str, icon: &'a str) -> &'a str {
    let lower = title.trim().to_lowercase();
    let trimmed = title.trim();
    let canonical = nav_title_to_canonical(&lower).or_else(|| nav_title_to_canonical(trimmed));
    let Some(canonical) = canonical else {
        return icon;
    };
    if icon == canonical {
        return icon;
    }
    let wrong_alts: &[&str] = match canonical {
        "shopping-cart" => &["shopping-bag", "bag", "package", "tote"],
        "user" => &["profile", "account", "avatar", "circle-user-round"],
        "house" => &["home"],
        "bell" => &["notification", "alarm"],
        "message-circle" => &["message", "chat"],
        "clipboard-list" => &["list", "orders", "receipt"],
        _ => &[],
    };
    if wrong_alts.contains(&icon) {
        canonical
    } else {
        icon
    }
}

/// NAV_TITLE_TO_CANONICAL_ICON table from coerce-params.ts (lowercase
/// keys; bilingual EN/中文). Returns the canonical lucide slug or None.
fn nav_title_to_canonical(key: &str) -> Option<&'static str> {
    Some(match key {
        "cart" | "购物车" => "shopping-cart",
        "bag" | "购物袋" => "shopping-bag",
        "home" | "首页" => "house",
        "search" | "搜索" => "search",
        "profile" | "account" | "我的" | "账户" => "user",
        "orders" | "订单" => "clipboard-list",
        "inbox" | "收件箱" => "inbox",
        "notifications" | "通知" => "bell",
        "messages" | "消息" => "message-circle",
        "settings" | "设置" => "settings",
        "favorites" | "likes" | "收藏" => "heart",
        "explore" | "discover" | "发现" => "compass",
        _ => return None,
    })
}
