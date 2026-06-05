//! Ported TS element builders (rows-a). Real role-tagged subtrees
//! for kinds that previously fell to the generic kit placeholder.
#![allow(clippy::result_large_err)]

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::element_ported_helpers::*;
use crate::{ToolErrorCode, ToolOutcome};

pub(crate) fn ported_rows_a_alias_node_value(
    tool: &str,
    args: &std::collections::BTreeMap<String, String>,
) -> Result<Option<serde_json::Value>, crate::ToolOutcome> {
    let value = match tool {
        "add_list_row_v0" => build_list_row(args, false)?,
        "add_list_row_v1" => build_list_row(args, true)?,
        "add_member_row_v0" => build_member_row(args, false)?,
        "add_member_row_v1" => build_member_row(args, true)?,
        "add_setting_row_v0" => build_setting_row(args, false)?,
        "add_setting_row_v1" => build_setting_row(args, true)?,
        "add_notification_row_v0" => build_notification_row(args, false)?,
        "add_notification_row_v1" => build_notification_row(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

// ===== add_list_row =====

// ── list-row (v0 == v1: no hardcoded colors, theme is byte-parity) ────
//
// list-row-v1.ts has NO color tokens — all modes (light/dark/system)
// are byte-identical to v0. The `theme` arg is accepted for API
// consistency but never read, so `theme_aware` is ignored here.

fn build_list_row(
    args: &BTreeMap<String, String>,
    _theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let title = required(args, "title")?;
    let mut row_children: Vec<Value> = Vec::new();

    if let Some(icon) = opt(args, "leading_icon") {
        row_children.push(json!({
            "id": next_id("list_row"),
            "type": "icon_font",
            "name": "Leading Icon",
            "iconFontName": icon,
            "iconFontFamily": "lucide",
            "width": 24,
            "height": 24,
        }));
    }

    let mut text_stack_children: Vec<Value> = vec![json!({
        "id": next_id("list_row"),
        "type": "text",
        "name": "Title",
        "role": "label",
        "content": title,
        "fontSize": 15,
        "fontWeight": 500,
        "width": "fill_container",
        "textGrowth": "fixed-width",
    })];
    if let Some(subtitle) = opt(args, "subtitle") {
        text_stack_children.push(json!({
            "id": next_id("list_row"),
            "type": "text",
            "name": "Subtitle",
            "role": "body",
            "content": subtitle,
            "fontSize": 13,
            "fontWeight": 400,
            "width": "fill_container",
            "textGrowth": "fixed-width",
        }));
    }
    row_children.push(json!({
        "id": next_id("list_row"),
        "type": "frame",
        "name": "Text Stack",
        "role": "list-row-text",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 2,
        "children": text_stack_children,
    }));

    if let Some(icon) = opt(args, "trailing_icon") {
        row_children.push(json!({
            "id": next_id("list_row"),
            "type": "icon_font",
            "name": "Trailing Icon",
            "iconFontName": icon,
            "iconFontFamily": "lucide",
            "width": 16,
            "height": 16,
        }));
    }

    Ok(json!({
        "id": next_id("list_row_root"),
        "type": "frame",
        "name": "List Row",
        "role": "list-row",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 12,
        "padding": [12, 16],
        "children": row_children,
    }))
}

// ===== add_member_row =====

// ── member-row ────────────────────────────────────────────────────────
//
// v0 (member-row.ts) hardcodes light hex; v1 (member-row-v1.ts) resolves
// surface2 / textPrimary / textMuted / textBody / textSubtle via
// resolveTheme(theme). Status-dot tones are semantic/fixed (not theme).
// v0 throws on invalid tone; v1 coerceEnum tolerates → 'online'.

const STATUS_TONE_FILL: &[(&str, &str)] = &[
    ("online", "#10B981"),
    ("busy", "#EF4444"),
    ("away", "#F59E0B"),
    ("offline", "#94A3B8"),
];

fn build_member_row(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let name = required(args, "name")?;
    let colors = row_colors(args, theme_aware);

    let size = 40;
    let avatar_color = opt(args, "avatar_color").unwrap_or("#3B82F6");
    let initial_char = member_initial(args, name);

    let avatar = json!({
        "id": next_id("member_row"),
        "type": "frame",
        "name": "Avatar",
        "role": "member-row-avatar",
        "width": size,
        "height": size,
        "cornerRadius": size / 2,
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "fill": solid(avatar_color),
        "children": [{
            "id": next_id("member_row"),
            "type": "text",
            "name": "Initial",
            "role": "member-row-avatar-initial",
            "content": initial_char,
            "fontSize": 15,
            "fontWeight": 600,
            "fill": solid("#FFFFFF"),
        }],
    });

    let mut text_stack_children: Vec<Value> = vec![json!({
        "id": next_id("member_row"),
        "type": "text",
        "name": "Name",
        "role": "member-row-name",
        "content": name,
        "fontSize": 15,
        "fontWeight": 500,
        "width": "fill_container",
        "textGrowth": "fixed-width",
        "fill": solid(colors.text_primary),
    })];
    if let Some(subtitle) = opt(args, "subtitle") {
        text_stack_children.push(json!({
            "id": next_id("member_row"),
            "type": "text",
            "name": "Subtitle",
            "role": "member-row-subtitle",
            "content": subtitle,
            "fontSize": 13,
            "fontWeight": 400,
            "width": "fill_container",
            "textGrowth": "fixed-width",
            "fill": solid(colors.text_muted),
        }));
    }

    let mut row_children: Vec<Value> = vec![
        avatar,
        json!({
            "id": next_id("member_row"),
            "type": "frame",
            "name": "Text Stack",
            "role": "member-row-text",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "gap": 2,
            "children": text_stack_children,
        }),
    ];

    if let Some(trailing) = parse_trailing(args)? {
        row_children.push(member_trailing(&trailing, colors, theme_aware)?);
    }

    Ok(json!({
        "id": next_id("member_row_root"),
        "type": "frame",
        "name": "Member Row",
        "role": "member-row",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 12,
        "padding": [12, 16],
        "children": row_children,
    }))
}

/// `(params.initial ?? name.charAt(0) ?? '?').slice(0, 2).toUpperCase()`.
fn member_initial(args: &BTreeMap<String, String>, name: &str) -> String {
    let source = opt(args, "initial")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| name.chars().next().map(|c| c.to_string()))
        .unwrap_or_else(|| "?".to_string());
    source.chars().take(2).collect::<String>().to_uppercase()
}

fn member_trailing(
    trailing: &Value,
    colors: RowColors,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let kind = string_field(trailing, "kind").unwrap_or("");
    match kind {
        "role_badge" => {
            let value = string_field(trailing, "value").ok_or_else(|| {
                ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "trailing.value is required for role_badge".into(),
                )
            })?;
            Ok(json!({
                "id": next_id("member_row"),
                "type": "frame",
                "name": "Role Badge",
                "role": "member-row-badge",
                "width": "fit_content",
                "height": "fit_content",
                "cornerRadius": 4,
                "fill": solid(colors.surface2),
                "padding": [3, 8],
                "children": [{
                    "id": next_id("member_row"),
                    "type": "text",
                    "name": "Role",
                    "role": "member-row-badge-text",
                    "content": value,
                    "fontSize": 12,
                    "fontWeight": 500,
                    "fill": solid(colors.text_body),
                }],
            }))
        }
        "menu" => Ok(json!({
            "id": next_id("member_row"),
            "type": "icon_font",
            "name": "Menu",
            "role": "member-row-menu",
            "iconFontName": "more-vertical",
            "iconFontFamily": "lucide",
            "width": 20,
            "height": 20,
            "fill": solid(colors.text_subtle),
        })),
        "status_dot" => {
            let tone = coerce_status_tone(string_field(trailing, "tone"), theme_aware)?;
            let fill = STATUS_TONE_FILL
                .iter()
                .find(|(name, _)| *name == tone)
                .map(|(_, hex)| *hex)
                .unwrap_or("#10B981");
            Ok(json!({
                "id": next_id("member_row"),
                "type": "frame",
                "name": "Status Dot",
                "role": "member-row-status",
                "width": 10,
                "height": 10,
                "cornerRadius": 5,
                "fill": solid(fill),
                "children": [],
            }))
        }
        other => Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("trailing.kind {other:?} is not a valid member-row trailing"),
        )),
    }
}

/// v0 throws on an invalid tone; v1 (`coerceEnum`) tolerates and falls
/// back to 'online' in every theme mode.
fn coerce_status_tone(tone: Option<&str>, theme_aware: bool) -> Result<&'static str, ToolOutcome> {
    const TONES: &[&str] = &["online", "busy", "away", "offline"];
    match tone {
        None => Ok("online"),
        Some(t) => match TONES.iter().find(|valid| **valid == t) {
            Some(valid) => Ok(valid),
            // v1 (theme_aware) coerceEnum tolerates → 'online'.
            None if theme_aware => Ok("online"),
            // v0 preserves the original hard error.
            None => Err(ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "add_member_row_v0: invalid trailing.tone {t:?}; expected one of: online, busy, away, offline"
                ),
            )),
        },
    }
}

// ===== add_setting_row =====

// ── setting-row ───────────────────────────────────────────────────────
//
// v0 (setting-row.ts) hardcodes light hex; v1 (setting-row-v1.ts)
// resolves textPrimary / textMuted / accent / borderStrong via
// resolveTheme. The badge keeps v0's builder-private literals in LIGHT
// mode (#DBEAFE/#1D4ED8) but switches to the info-alert tokens in
// dark/system. Knob is always white. Default trailing = chevron.

fn build_setting_row(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let title = required(args, "title")?;
    let colors = row_colors(args, theme_aware);

    let mut row_children: Vec<Value> = Vec::new();

    if let Some(icon) = opt(args, "leading_icon") {
        row_children.push(json!({
            "id": next_id("setting_row"),
            "type": "icon_font",
            "name": "Leading Icon",
            "role": "setting-row-icon",
            "iconFontName": icon,
            "iconFontFamily": "lucide",
            "width": 24,
            "height": 24,
            "fill": solid(colors.text_primary),
        }));
    }

    let mut text_stack_children: Vec<Value> = vec![json!({
        "id": next_id("setting_row"),
        "type": "text",
        "name": "Title",
        "role": "setting-row-title",
        "content": title,
        "fontSize": 15,
        "fontWeight": 500,
        "width": "fill_container",
        "textGrowth": "fixed-width",
        "fill": solid(colors.text_primary),
    })];
    if let Some(subtitle) = opt(args, "subtitle") {
        text_stack_children.push(json!({
            "id": next_id("setting_row"),
            "type": "text",
            "name": "Subtitle",
            "role": "setting-row-subtitle",
            "content": subtitle,
            "fontSize": 13,
            "fontWeight": 400,
            "width": "fill_container",
            "textGrowth": "fixed-width",
            "fill": solid(colors.text_muted),
        }));
    }

    row_children.push(json!({
        "id": next_id("setting_row"),
        "type": "frame",
        "name": "Text Stack",
        "role": "setting-row-text",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 2,
        "children": text_stack_children,
    }));

    // Default trailing = chevron when no `trailing` arg supplied.
    let trailing = parse_trailing(args)?;
    row_children.push(setting_trailing(trailing.as_ref(), colors)?);

    Ok(json!({
        "id": next_id("setting_row_root"),
        "type": "frame",
        "name": "Setting Row",
        "role": "setting-row",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 12,
        "padding": [14, 16],
        "children": row_children,
    }))
}

fn setting_trailing(trailing: Option<&Value>, colors: RowColors) -> Result<Value, ToolOutcome> {
    let kind = trailing
        .and_then(|t| string_field(t, "kind"))
        .unwrap_or("chevron");
    match kind {
        "value" => {
            let value = trailing
                .and_then(|t| string_field(t, "value"))
                .ok_or_else(|| {
                    ToolOutcome::Err(
                        ToolErrorCode::InvalidArgument,
                        "trailing.value is required for value".into(),
                    )
                })?;
            Ok(json!({
                "id": next_id("setting_row"),
                "type": "text",
                "name": "Trailing Value",
                "role": "setting-row-value",
                "content": value,
                "fontSize": 14,
                "fontWeight": 400,
                "fill": solid(colors.text_muted),
            }))
        }
        "switch" => {
            let on = trailing.map(|t| bool_field(t, "on")).unwrap_or(false);
            Ok(json!({
                "id": next_id("setting_row"),
                "type": "frame",
                "name": "Switch",
                "role": "setting-row-switch",
                "width": 36,
                "height": 22,
                "cornerRadius": 11,
                "fill": solid(if on { colors.accent } else { colors.border_strong }),
                "layout": "horizontal",
                "alignItems": "center",
                "justifyContent": if on { "flex-end" } else { "flex-start" },
                "padding": [0, 3],
                "children": [{
                    "id": next_id("setting_row"),
                    "type": "frame",
                    "name": "Knob",
                    "role": "setting-row-switch-knob",
                    "width": 16,
                    "height": 16,
                    "cornerRadius": 8,
                    "fill": solid("#FFFFFF"),
                    "children": [],
                }],
            }))
        }
        "badge" => {
            let value = trailing
                .and_then(|t| string_field(t, "value"))
                .ok_or_else(|| {
                    ToolOutcome::Err(
                        ToolErrorCode::InvalidArgument,
                        "trailing.value is required for badge".into(),
                    )
                })?;
            // Light keeps v0's builder-private literals; dark/system uses
            // the info alert tokens.
            let (badge_bg, badge_fg) = if colors.mode == Mode::Light {
                ("#DBEAFE", "#1D4ED8")
            } else {
                (colors.info_bg, colors.info_text)
            };
            Ok(json!({
                "id": next_id("setting_row"),
                "type": "frame",
                "name": "Badge",
                "role": "setting-row-badge",
                "width": "fit_content",
                "height": "fit_content",
                "cornerRadius": 4,
                "fill": solid(badge_bg),
                "padding": [2, 8],
                "children": [{
                    "id": next_id("setting_row"),
                    "type": "text",
                    "name": "Badge Text",
                    "role": "setting-row-badge-text",
                    "content": value,
                    "fontSize": 12,
                    "fontWeight": 600,
                    "fill": solid(badge_fg),
                }],
            }))
        }
        "chevron" => Ok(json!({
            "id": next_id("setting_row"),
            "type": "icon_font",
            "name": "Chevron",
            "role": "setting-row-chevron",
            "iconFontName": "chevron-right",
            "iconFontFamily": "lucide",
            "width": 20,
            "height": 20,
            "fill": solid(colors.text_muted),
        })),
        other => Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("trailing.kind {other:?} is not a valid setting-row trailing"),
        )),
    }
}

// ===== add_notification_row =====

// ── notification-row ──────────────────────────────────────────────────
//
// v0 (notification-row.ts) hardcodes slate hex; v1 (notification-row-v1.ts)
// keeps the v0 literals in LIGHT mode and maps timestamp→textSubtle,
// body→textBody, unread-dot→destructive in dark/system. NB v0 body color
// is #475569 which is NOT textBody-light (#334155) — light path must keep
// the literal, only dark/system use the token.

fn build_notification_row(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let title = required(args, "title")?;
    let icon = opt(args, "icon").unwrap_or("bell");
    let colors = row_colors(args, theme_aware);
    let is_light = colors.mode == Mode::Light;

    // Light mode keeps v0's hardcoded slate hex; dark/system uses tokens.
    let timestamp_color = if is_light {
        "#94A3B8"
    } else {
        colors.text_subtle
    };
    let body_color = if is_light {
        "#475569"
    } else {
        colors.text_body
    };
    let dot_color = if is_light {
        "#EF4444"
    } else {
        colors.destructive
    };

    let mut title_row_children: Vec<Value> = vec![json!({
        "id": next_id("notification_row"),
        "type": "text",
        "name": "Title",
        "role": "notification-title",
        "content": title,
        "fontSize": 14,
        "fontWeight": 600,
    })];
    if bool_arg(args, "unread") {
        title_row_children.push(json!({
            "id": next_id("notification_row"),
            "type": "frame",
            "name": "Unread Dot",
            "role": "notification-unread-dot",
            "width": 8,
            "height": 8,
            "cornerRadius": 4,
            "fill": solid(dot_color),
        }));
    }

    let mut header_row_children: Vec<Value> = vec![json!({
        "id": next_id("notification_row"),
        "type": "frame",
        "name": "Title Row",
        "role": "notification-title-row",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 6,
        "children": title_row_children,
    })];
    if let Some(timestamp) = opt(args, "timestamp") {
        header_row_children.push(json!({
            "id": next_id("notification_row"),
            "type": "text",
            "name": "Timestamp",
            "role": "notification-timestamp",
            "content": timestamp,
            "fontSize": 12,
            "fontWeight": 400,
            "fill": solid(timestamp_color),
        }));
    }

    let mut body_col_children: Vec<Value> = vec![json!({
        "id": next_id("notification_row"),
        "type": "frame",
        "name": "Header Row",
        "role": "notification-header",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "space_between",
        "gap": 8,
        "children": header_row_children,
    })];
    if let Some(body) = opt(args, "body") {
        body_col_children.push(json!({
            "id": next_id("notification_row"),
            "type": "text",
            "name": "Body Preview",
            "role": "notification-body",
            "content": body,
            "fontSize": 13,
            "fontWeight": 400,
            "lineHeight": 1.4,
            "fill": solid(body_color),
        }));
    }

    Ok(json!({
        "id": next_id("notification_row_root"),
        "type": "frame",
        "name": "Notification Row",
        "role": "notification-row",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "start",
        "padding": [12, 16],
        "gap": 12,
        "children": [
            {
                "id": next_id("notification_row"),
                "type": "icon_font",
                "name": "Leading Icon",
                "role": "notification-icon",
                "iconFontName": icon,
                "iconFontFamily": "lucide",
                "width": 20,
                "height": 20,
            },
            {
                "id": next_id("notification_row"),
                "type": "frame",
                "name": "Body Column",
                "role": "notification-body-column",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "gap": 2,
                "children": body_col_children,
            }
        ],
    }))
}

// ════════════════════════════════════════════════════════════════════
// SHARED SCAFFOLDING for the four row builders (paste once into the
// element_ported_alias_builders.rs module). Imports assumed in scope:
//   use std::collections::BTreeMap;
//   use serde_json::{json, Value};
//   use crate::{ToolErrorCode, ToolOutcome};
// plus the shared helpers (next_id/required/opt/number_arg/bool_arg/
// parse_object_items/string_field/bool_field/icon_node).
// ════════════════════════════════════════════════════════════════════

/// Theme mode mirrors `resolve-theme.ts`. A `_v0` builder is always
/// Light; a `_v1` builder reads the `theme` arg.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Light,
    Dark,
    System,
}

/// Resolved palette colors used across the row builders. Every field is
/// a `&'static str` (concrete hex or `$ref` literal).
#[derive(Clone, Copy)]
struct RowColors {
    mode: Mode,
    surface2: &'static str,
    border_strong: &'static str,
    text_primary: &'static str,
    text_body: &'static str,
    text_muted: &'static str,
    text_subtle: &'static str,
    accent: &'static str,
    destructive: &'static str,
    info_bg: &'static str,
    info_text: &'static str,
}

fn row_colors(args: &BTreeMap<String, String>, theme_aware: bool) -> RowColors {
    let mode = if theme_aware {
        match args.get("theme").map(String::as_str) {
            Some("dark") => Mode::Dark,
            Some("system") => Mode::System,
            _ => Mode::Light,
        }
    } else {
        Mode::Light
    };
    match mode {
        Mode::Light => RowColors {
            mode,
            surface2: "#F1F5F9",
            border_strong: "#CBD5E1",
            text_primary: "#0F172A",
            text_body: "#334155",
            text_muted: "#64748B",
            text_subtle: "#94A3B8",
            accent: "#2563EB",
            destructive: "#EF4444",
            info_bg: "#DBEAFE",
            info_text: "#1E40AF",
        },
        Mode::Dark => RowColors {
            mode,
            surface2: "#334155",
            border_strong: "#475569",
            text_primary: "#F1F5F9",
            text_body: "#CBD5E1",
            text_muted: "#94A3B8",
            text_subtle: "#64748B",
            accent: "#60A5FA",
            destructive: "#F87171",
            info_bg: "#1E3A8A",
            info_text: "#BFDBFE",
        },
        Mode::System => RowColors {
            mode,
            surface2: "$color-surface-2",
            border_strong: "$color-border-strong",
            text_primary: "$color-text-primary",
            text_body: "$color-text-body",
            text_muted: "$color-text-muted",
            text_subtle: "$color-text-subtle",
            accent: "$color-accent",
            destructive: "$color-destructive",
            info_bg: "$color-info-bg",
            info_text: "$color-info-text",
        },
    }
}

fn solid(color: &str) -> Value {
    json!([{ "type": "solid", "color": color }])
}

/// Parse the optional `trailing` arg (the TS discriminated union
/// `{ kind, ... }`) into a JSON object. Returns `None` when absent so
/// each caller can apply its own default.
fn parse_trailing(args: &BTreeMap<String, String>) -> Result<Option<Value>, ToolOutcome> {
    let Some(raw) = opt(args, "trailing") else {
        return Ok(None);
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("trailing must be a JSON object: {e}"),
        )
    })?;
    if !value.is_object() {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "trailing must be a JSON object".into(),
        ));
    }
    Ok(Some(value))
}
