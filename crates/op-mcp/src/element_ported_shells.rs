//! Ported TS element builders (shells). Real role-tagged subtrees
//! for kinds that previously fell to the generic kit placeholder.
#![allow(clippy::result_large_err)]

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::element_ported_helpers::*;
use crate::ToolOutcome;

pub(crate) fn ported_shells_alias_node_value(
    tool: &str,
    args: &std::collections::BTreeMap<String, String>,
) -> Result<Option<serde_json::Value>, crate::ToolOutcome> {
    let value = match tool {
        "add_modal_shell_v0" => build_modal_shell(args, false)?,
        "add_modal_shell_v1" => build_modal_shell(args, true)?,
        "add_drawer_shell_v0" => build_drawer_shell(args, false)?,
        "add_drawer_shell_v1" => build_drawer_shell(args, true)?,
        "add_cookie_banner_v0" => build_cookie_banner(args, false)?,
        "add_cookie_banner_v1" => build_cookie_banner(args, true)?,
        "add_filter_group_v0" => build_filter_group(args, false)?,
        "add_filter_group_v1" => build_filter_group(args, true)?,
        "add_inbox_message_v0" => build_inbox_message(args, false)?,
        "add_inbox_message_v1" => build_inbox_message(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

// ===== add_modal_shell =====

/// Extended theme palette for the shell/banner ported builders. Adds the
/// `surface2`, `text_body`, and `accent` slots the existing `ThemeColors`
/// helper omits. In `light` mode these mirror the v0 hardcoded literals
/// (byte-parity); `dark` swaps in semantic-palette dark hex; `system`
/// emits `$color-*` refs.
struct ShellThemeColors {
    surface: &'static str,
    surface2: &'static str,
    border: &'static str,
    text_primary: &'static str,
    text_body: &'static str,
    text_muted: &'static str,
    text_subtle: &'static str,
    accent: &'static str,
}

/// Resolve the `theme` arg to the extended palette. `theme_aware == false`
/// always returns the light palette (the v0 builders never branch on
/// theme). Unknown theme values fall through to light, matching the TS
/// `theme ?? 'light'` + `isLight = theme === 'light'` semantics where any
/// non-light/dark/system string is treated as non-light by resolveTheme
/// but here we only special-case the three documented modes.
fn shell_theme_colors(theme_aware: bool, args: &BTreeMap<String, String>) -> ShellThemeColors {
    let theme = if theme_aware {
        args.get("theme").map(String::as_str).unwrap_or("light")
    } else {
        "light"
    };
    match theme {
        "dark" => ShellThemeColors {
            surface: "#1E293B",
            surface2: "#334155",
            border: "#334155",
            text_primary: "#F1F5F9",
            text_body: "#CBD5E1",
            text_muted: "#94A3B8",
            text_subtle: "#64748B",
            accent: "#60A5FA",
        },
        "system" => ShellThemeColors {
            surface: "$color-surface",
            surface2: "$color-surface-2",
            border: "$color-border",
            text_primary: "$color-text-primary",
            text_body: "$color-text-body",
            text_muted: "$color-text-muted",
            text_subtle: "$color-text-subtle",
            accent: "$color-accent",
        },
        _ => ShellThemeColors {
            surface: "#FFFFFF",
            surface2: "#F1F5F9",
            border: "#E2E8F0",
            text_primary: "#0F172A",
            text_body: "#334155",
            text_muted: "#64748B",
            text_subtle: "#94A3B8",
            accent: "#2563EB",
        },
    }
}

/// Modal dialog shell — full-bleed dimmed backdrop + centered card with
/// title + optional subtitle. Mirrors `buildModalShell` /
/// `buildModalShellV1`.
///
/// v0 parity: in light mode the title has NO fill (the v0 builder left it
/// unstyled). In dark/system the title gets `textPrimary`. cornerRadius=16
/// is a builder-private value hardcoded across all themes. The scrim is
/// black regardless of theme (a backdrop is "dim everything below");
/// `scrim_opacity` tunes only the darkness, clamped 0..1.
///
/// Fidelity caveat: typography sizes/weights are theme-agnostic numbers in
/// resolveTheme (h2 20/600, body 14/400, body line-height 1.5) so we emit
/// numbers in every mode. In `system` mode the TS would emit `$type-*` refs
/// for these; the Rust `fontSize`/`fontWeight`/`lineHeight` schema slots are
/// strict numbers, and the refs resolve to these same defaults at render
/// time, so the rendered result is identical. Padding/gap must ALSO stay
/// numeric in system mode: the schema accepts `$spacing-*` expression refs
/// but the Rust layout chain zeroes them (jian-core `container_to_style`
/// and op-pen-loader `gap_value` both fall through Expression to 0), which
/// collapses the card. 24/12 equal the semantic palette's spacing-5/3
/// values, so the rendered result matches a resolved ref.
fn build_modal_shell(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let title = required(args, "title")?;
    let card_width = number_arg(args, "card_width", 400.0, 280.0).floor();
    // scrim_opacity = clamp(scrim_opacity ?? 0.5, 0, 1).
    let scrim_opacity = args
        .get("scrim_opacity")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let theme = if theme_aware {
        args.get("theme").map(String::as_str).unwrap_or("light")
    } else {
        "light"
    };
    let c = shell_theme_colors(theme_aware, args);

    // Padding: explicit override (max(12, floor)) wins; absent → 24 in
    // every mode (numeric — see the fidelity caveat above).
    let card_padding: Value = match args.get("card_padding").and_then(|v| v.parse::<f64>().ok()) {
        Some(raw) => json!(raw.floor().max(12.0)),
        None => json!(24),
    };
    // gap: 12 in every mode (numeric — see the fidelity caveat above).
    let card_gap: Value = json!(12);

    // Title: v0/light has no fill; v1 dark/system gets textPrimary.
    let mut title_node = json!({
        "id": next_id("modal_title"),
        "type": "text",
        "name": "Title",
        "role": "modal-title",
        "content": title,
        "fontSize": 20,
        "fontWeight": 600,
    });
    if theme_aware && theme != "light" {
        title_node["fill"] = json!([{ "type": "solid", "color": c.text_primary }]);
    }

    let mut card_children = vec![title_node];
    if let Some(subtitle) = opt(args, "subtitle") {
        card_children.push(json!({
            "id": next_id("modal_subtitle"),
            "type": "text",
            "name": "Subtitle",
            "role": "modal-subtitle",
            "content": subtitle,
            "fontSize": 14,
            "fontWeight": 400,
            "lineHeight": 1.5,
            "fill": [{ "type": "solid", "color": c.text_muted }],
        }));
    }

    // Scrim fill: black @ opacity, or empty array when opacity == 0.
    let scrim_fill: Value = if scrim_opacity > 0.0 {
        json!([{ "type": "solid", "color": "#000000", "opacity": scrim_opacity }])
    } else {
        json!([])
    };

    Ok(json!({
        "id": next_id("modal_shell_root"),
        "type": "frame",
        "name": "Modal Shell",
        "role": "modal-scrim",
        "width": "fill_container",
        "height": "fill_container",
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "fill": scrim_fill,
        "children": [{
            "id": next_id("modal_card"),
            "type": "frame",
            "name": "Modal Card",
            "role": "modal-shell-card",
            "width": card_width,
            "height": "fit_content",
            "cornerRadius": 16,
            "padding": card_padding,
            "layout": "vertical",
            "gap": card_gap,
            "fill": [{ "type": "solid", "color": c.surface }],
            "effects": [{
                "type": "shadow",
                "offsetX": 0,
                "offsetY": 16,
                "blur": 40,
                "spread": 0,
                "color": "#00000026",
            }],
            "children": card_children,
        }],
    }))
}

// ===== add_drawer_shell =====

/// Slide-in drawer shell — full-height side panel with a header row
/// (title + close ×). Mirrors `buildDrawerShell` / `buildDrawerShellV1`.
/// Side is encoded in the role (`drawer-shell-left` / `drawer-shell-right`)
/// and drives the shadow offsetX (left → +8, right → -8). Width clamps to
/// 280..640. Shadow color is theme-agnostic (`#0F172A1F`).
fn build_drawer_shell(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let title = required(args, "title")?;
    let side = match opt(args, "side") {
        Some("left") => "left",
        _ => "right",
    };
    // width = clamp(floor(width ?? 400), 280, 640).
    let width = number_arg(args, "width", 400.0, 280.0).floor().min(640.0);
    let c = shell_theme_colors(theme_aware, args);
    // The close icon is NOT `text_muted`: drawer-shell{,v1}.ts maps it to
    // #475569 (slate-600) in light, diverging from `text_muted`'s #64748B.
    let close_icon_color = drawer_close_icon_color(theme_aware, args);

    let role = if side == "left" {
        "drawer-shell-left"
    } else {
        "drawer-shell-right"
    };
    let shadow_offset_x = if side == "left" { 8 } else { -8 };

    Ok(json!({
        "id": next_id("drawer_shell_root"),
        "type": "frame",
        "name": "Drawer Shell",
        "role": role,
        "width": width,
        "height": "fill_container",
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": c.surface }],
        "effects": [{
            "type": "shadow",
            "offsetX": shadow_offset_x,
            "offsetY": 0,
            "blur": 24,
            "spread": 0,
            "color": "#0F172A1F",
        }],
        "children": [{
            "id": next_id("drawer_header"),
            "type": "frame",
            "name": "Header",
            "role": "drawer-shell-header",
            "width": "fill_container",
            "height": 56,
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "space_between",
            "padding": [0, 20],
            "stroke": {
                "thickness": [0, 0, 1, 0],
                "fill": [{ "type": "solid", "color": c.border }],
            },
            "children": [
                {
                    "id": next_id("drawer_title"),
                    "type": "text",
                    "name": "Title",
                    "role": "drawer-shell-title",
                    "content": title,
                    "fontSize": 16,
                    "fontWeight": 600,
                    "fill": [{ "type": "solid", "color": c.text_primary }],
                },
                {
                    "id": next_id("drawer_close"),
                    "type": "frame",
                    "name": "Close Button",
                    "role": "drawer-shell-close",
                    "width": 32,
                    "height": 32,
                    "cornerRadius": 8,
                    "layout": "horizontal",
                    "alignItems": "center",
                    "justifyContent": "center",
                    "children": [{
                        "id": next_id("drawer_close_icon"),
                        "type": "icon_font",
                        "name": "Icon",
                        "iconFontName": "x",
                        "iconFontFamily": "lucide",
                        "width": 18,
                        "height": 18,
                        "fill": [{ "type": "solid", "color": close_icon_color }],
                    }],
                }
            ],
        }],
    }))
}

/// Drawer close-icon color, resolved independently of the shared palette
/// for byte-parity with `drawer-shell.ts` / `drawer-shell-v1.ts`:
/// `closeIconColor = isLight ? '#475569' : t.colors.textMuted`. v0
/// (`theme_aware == false`) is always light. In dark/system the value
/// coincides with `text_muted` (#94A3B8 / `$color-text-muted`), but light
/// must be #475569 (slate-600), NOT `text_muted`'s #64748B.
fn drawer_close_icon_color(theme_aware: bool, args: &BTreeMap<String, String>) -> &str {
    if theme_aware {
        match args.get("theme").map(String::as_str).unwrap_or("light") {
            "dark" => "#94A3B8",
            "system" => "$color-text-muted",
            _ => "#475569",
        }
    } else {
        "#475569"
    }
}

// ===== add_cookie_banner =====

/// JS `String(n)` for a JSON number — integers print without a trailing
/// `.0`, non-integers keep their fractional part. Used for facet counts.
fn js_number_string(v: &Value) -> Option<String> {
    let n = v.as_f64()?;
    if n.fract() == 0.0 && n.is_finite() {
        Some((n as i64).to_string())
    } else {
        Some(n.to_string())
    }
}

/// Cookie consent banner — title + body + accept/decline buttons + optional
/// fine-grained settings link. Mirrors `buildCookieBanner` /
/// `buildCookieBannerV1`.
///
/// v1 color mapping: card bg → surface, stroke → border, title →
/// textPrimary, body → textMuted, decline bg → surface2, decline fg →
/// textPrimary, accept bg → accent (brand-invariant), accept fg → white,
/// settings link → accent. Accept-button white text + shadow (`#0F172A26`)
/// are theme-agnostic. Width min 320.
fn build_cookie_banner(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let width = number_arg(args, "width", 720.0, 320.0).floor();
    let title = opt(args, "title").unwrap_or("We use cookies");
    let body = opt(args, "body").unwrap_or(
        "We use cookies to enhance your experience, analyze site traffic, and personalize content.",
    );
    let accept_label = opt(args, "accept_label").unwrap_or("Accept all");
    let decline_label = opt(args, "decline_label").unwrap_or("Reject");
    let show_settings = bool_arg(args, "show_settings_link");
    let settings_label = opt(args, "settings_label").unwrap_or("Cookie settings");
    let c = shell_theme_colors(theme_aware, args);

    let decline_button = json!({
        "id": next_id("cookie_decline"),
        "type": "frame",
        "name": "Decline Button",
        "role": "cookie-banner-decline",
        "width": "fit_content",
        "height": 40,
        "cornerRadius": 8,
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "padding": [0, 18],
        "fill": [{ "type": "solid", "color": c.surface2 }],
        "children": [{
            "id": next_id("cookie_decline_label"),
            "type": "text",
            "name": "Decline Label",
            "role": "cookie-banner-decline-label",
            "content": decline_label,
            "fontSize": 13,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": c.text_primary }],
        }],
    });
    let accept_button = json!({
        "id": next_id("cookie_accept"),
        "type": "frame",
        "name": "Accept Button",
        "role": "cookie-banner-accept",
        "width": "fit_content",
        "height": 40,
        "cornerRadius": 8,
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "padding": [0, 18],
        "fill": [{ "type": "solid", "color": c.accent }],
        "children": [{
            "id": next_id("cookie_accept_label"),
            "type": "text",
            "name": "Accept Label",
            "role": "cookie-banner-accept-label",
            "content": accept_label,
            "fontSize": 13,
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        }],
    });

    let mut children = vec![
        json!({
            "id": next_id("cookie_title"),
            "type": "text",
            "name": "Title",
            "role": "cookie-banner-title",
            "content": title,
            "fontSize": 16,
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": c.text_primary }],
        }),
        json!({
            "id": next_id("cookie_body"),
            "type": "text",
            "name": "Body",
            "role": "cookie-banner-body",
            "content": body,
            "fontSize": 13,
            "fontWeight": 400,
            "lineHeight": 1.5,
            "fill": [{ "type": "solid", "color": c.text_muted }],
        }),
        json!({
            "id": next_id("cookie_actions"),
            "type": "frame",
            "name": "Actions",
            "role": "cookie-banner-actions",
            "width": "fit_content",
            "height": "fit_content",
            "layout": "horizontal",
            "alignItems": "center",
            "gap": 12,
            "children": [decline_button, accept_button],
        }),
    ];
    if show_settings {
        children.push(json!({
            "id": next_id("cookie_settings"),
            "type": "text",
            "name": "Settings Link",
            "role": "cookie-banner-settings",
            "content": settings_label,
            "fontSize": 12,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": c.accent }],
        }));
    }

    Ok(json!({
        "id": next_id("cookie_banner_root"),
        "type": "frame",
        "name": "Cookie Banner",
        "role": "cookie-banner",
        "width": width,
        "height": "fit_content",
        "cornerRadius": 12,
        "layout": "vertical",
        "gap": 12,
        "padding": 20,
        "fill": [{ "type": "solid", "color": c.surface }],
        "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": c.border }] },
        "effects": [{
            "type": "shadow",
            "offsetX": 0,
            "offsetY": 8,
            "blur": 24,
            "spread": 0,
            "color": "#0F172A26",
        }],
        "children": children,
    }))
}

// ===== add_filter_group =====

/// Sidebar filter group / facet — heading over a vertical list of
/// checkbox-style option rows (box + label + optional count). Mirrors
/// `buildFilterGroup` / `buildFilterGroupV1`.
///
/// v1 color mapping: title → textPrimary, label → textBody, count →
/// textSubtle, unselected box bg → surface, unselected box stroke →
/// border, selected box bg/stroke → accent, check icon → white (on accent,
/// brand-invariant in every theme). Each option row: 16×16 box, label
/// (fill_container fixed-width), optional count.
fn build_filter_group(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let title = required(args, "title")?;
    // options[] — items require no `label` validation error string match to
    // the TS (TS does not validate), but parse_object_items enforces label
    // which matches the per-row `opt.label` contract (label is required by
    // the FilterGroupOption type).
    let options = parse_object_items(args, "options", &["label"], "options[].label is required")?;
    let c = shell_theme_colors(theme_aware, args);

    let mut children = vec![json!({
        "id": next_id("filter_group_title"),
        "type": "text",
        "name": "Title",
        "role": "filter-group-title",
        "content": title,
        "fontSize": 13,
        "fontWeight": 600,
        "letterSpacing": 1,
        "fill": [{ "type": "solid", "color": c.text_primary }],
    })];

    for opt_item in &options {
        let label = string_field(opt_item, "label").unwrap_or("");
        let selected = bool_field(opt_item, "selected");
        let box_color = if selected { c.accent } else { c.surface };
        let box_stroke = if selected { c.accent } else { c.border };

        let box_children: Vec<Value> = if selected {
            vec![json!({
                "id": next_id("filter_group_check"),
                "type": "icon_font",
                "name": "Check",
                "role": "filter-group-check",
                "iconFontName": "check",
                "iconFontFamily": "lucide",
                "width": 12,
                "height": 12,
                "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            })]
        } else {
            vec![]
        };

        let mut row_children = vec![
            json!({
                "id": next_id("filter_group_box"),
                "type": "frame",
                "name": "Box",
                "role": "filter-group-checkbox",
                "width": 16,
                "height": 16,
                "cornerRadius": 4,
                "fill": [{ "type": "solid", "color": box_color }],
                "stroke": { "thickness": 1.5, "fill": [{ "type": "solid", "color": box_stroke }] },
                "layout": "horizontal",
                "alignItems": "center",
                "justifyContent": "center",
                "children": box_children,
            }),
            json!({
                "id": next_id("filter_group_label"),
                "type": "text",
                "name": "Label",
                "role": "filter-group-label",
                "content": label,
                "fontSize": 14,
                "fontWeight": 400,
                "width": "fill_container",
                "textGrowth": "fixed-width",
                "fill": [{ "type": "solid", "color": c.text_body }],
            }),
        ];
        // count is optional; render only when present (TS: opt.count !== undefined).
        if let Some(count) = opt_item.get("count").and_then(js_number_string) {
            row_children.push(json!({
                "id": next_id("filter_group_count"),
                "type": "text",
                "name": "Count",
                "role": "filter-group-count",
                "content": count,
                "fontSize": 13,
                "fontWeight": 400,
                "fill": [{ "type": "solid", "color": c.text_subtle }],
            }));
        }

        children.push(json!({
            "id": next_id("filter_group_option"),
            "type": "frame",
            "name": format!("Option: {label}"),
            "role": "filter-group-option",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "alignItems": "center",
            "gap": 10,
            "padding": [6, 0],
            "children": row_children,
        }));
    }

    Ok(json!({
        "id": next_id("filter_group_root"),
        "type": "frame",
        "name": "Filter Group",
        "role": "filter-group",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 8,
        "children": children,
    }))
}

// ===== add_inbox_message =====

/// Inbox / email list row — sender (+ timestamp) over subject over
/// optional preview, with an optional unread dot on the left. Mirrors
/// `buildInboxMessage` / `buildInboxMessageV1`.
///
/// v1 color mapping: from/subject → textPrimary, timestamp → textSubtle,
/// preview → textMuted, unread dot → accent. `unread` switches from/subject
/// to heavier weights (from 500→700, subject 400→600) regardless of theme.
fn build_inbox_message(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let from = required(args, "from")?;
    let subject = required(args, "subject")?;
    let is_unread = bool_arg(args, "unread");
    let c = shell_theme_colors(theme_aware, args);

    let from_weight = if is_unread { 700 } else { 500 };
    let subject_weight = if is_unread { 600 } else { 400 };

    let mut sender_row_children = vec![json!({
        "id": next_id("inbox_from"),
        "type": "text",
        "name": "From",
        "role": "inbox-message-from",
        "content": from,
        "fontSize": 14,
        "fontWeight": from_weight,
        "fill": [{ "type": "solid", "color": c.text_primary }],
    })];
    if let Some(timestamp) = opt(args, "timestamp") {
        sender_row_children.push(json!({
            "id": next_id("inbox_timestamp"),
            "type": "text",
            "name": "Timestamp",
            "role": "inbox-message-timestamp",
            "content": timestamp,
            "fontSize": 12,
            "fontWeight": 400,
            "fill": [{ "type": "solid", "color": c.text_subtle }],
        }));
    }

    let mut stack_children = vec![
        json!({
            "id": next_id("inbox_sender_row"),
            "type": "frame",
            "name": "Sender Row",
            "role": "inbox-message-sender-row",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "space_between",
            "gap": 8,
            "children": sender_row_children,
        }),
        json!({
            "id": next_id("inbox_subject"),
            "type": "text",
            "name": "Subject",
            "role": "inbox-message-subject",
            "content": subject,
            "fontSize": 14,
            "fontWeight": subject_weight,
            "fill": [{ "type": "solid", "color": c.text_primary }],
            "width": "fill_container",
            "textGrowth": "fixed-width",
        }),
    ];
    if let Some(preview) = opt(args, "preview") {
        stack_children.push(json!({
            "id": next_id("inbox_preview"),
            "type": "text",
            "name": "Preview",
            "role": "inbox-message-preview",
            "content": preview,
            "fontSize": 13,
            "fontWeight": 400,
            "fill": [{ "type": "solid", "color": c.text_muted }],
            "width": "fill_container",
            "textGrowth": "fixed-width",
        }));
    }

    let mut row_children = Vec::new();
    if is_unread {
        row_children.push(json!({
            "id": next_id("inbox_unread"),
            "type": "frame",
            "name": "Unread Dot",
            "role": "inbox-message-unread",
            "width": 8,
            "height": 8,
            "cornerRadius": 4,
            "fill": [{ "type": "solid", "color": c.accent }],
            "children": [],
        }));
    }
    row_children.push(json!({
        "id": next_id("inbox_stack"),
        "type": "frame",
        "name": "Stack",
        "role": "inbox-message-stack",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 2,
        "children": stack_children,
    }));

    Ok(json!({
        "id": next_id("inbox_message_root"),
        "type": "frame",
        "name": "Inbox Message",
        "role": "inbox-message",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "start",
        "gap": 10,
        "padding": [12, 16],
        "children": row_children,
    }))
}

#[cfg(test)]
mod shells_tests {
    use super::*;

    fn theme_args(theme: Option<&str>) -> BTreeMap<String, String> {
        let mut a = BTreeMap::new();
        a.insert("title".to_string(), "Menu".to_string());
        if let Some(t) = theme {
            a.insert("theme".to_string(), t.to_string());
        }
        a
    }

    /// Pin the drawer close-icon color: light (v0 + v1) must be #475569,
    /// NOT `text_muted`'s #64748B. Dark/system coincide with text_muted.
    /// Mirrors `closeIconColor = isLight ? '#475569' : t.colors.textMuted`.
    #[test]
    fn drawer_close_icon_color_matches_ts_per_theme() {
        // v0 is always light regardless of any theme arg.
        assert_eq!(drawer_close_icon_color(false, &theme_args(None)), "#475569");
        assert_eq!(
            drawer_close_icon_color(false, &theme_args(Some("dark"))),
            "#475569"
        );
        // v1 branches on theme; light defaults when unspecified.
        assert_eq!(drawer_close_icon_color(true, &theme_args(None)), "#475569");
        assert_eq!(
            drawer_close_icon_color(true, &theme_args(Some("light"))),
            "#475569"
        );
        assert_eq!(
            drawer_close_icon_color(true, &theme_args(Some("dark"))),
            "#94A3B8"
        );
        assert_eq!(
            drawer_close_icon_color(true, &theme_args(Some("system"))),
            "$color-text-muted"
        );
    }

    /// The emitted subtree must carry the resolved close-icon color in the
    /// icon fill (light v1 ⇒ #475569), proving the helper is wired in.
    #[test]
    fn build_drawer_shell_emits_resolved_close_icon_color() {
        let node = build_drawer_shell(&theme_args(None), true).expect("drawer builds");
        let icon_fill = &node["children"][0]["children"][1]["children"][0]["fill"][0]["color"];
        assert_eq!(icon_fill, "#475569");
    }
}
