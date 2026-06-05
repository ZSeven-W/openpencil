//! Ported TS element builders (rows-b). Real role-tagged subtrees
//! for kinds that previously fell to the generic kit placeholder.
#![allow(clippy::result_large_err)]

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::element_ported_helpers::*;
use crate::{ToolErrorCode, ToolOutcome};

pub(crate) fn ported_rows_b_alias_node_value(
    tool: &str,
    args: &std::collections::BTreeMap<String, String>,
) -> Result<Option<serde_json::Value>, crate::ToolOutcome> {
    let value = match tool {
        "add_invite_row_v0" => build_invite_row(args, false)?,
        "add_invite_row_v1" => build_invite_row(args, true)?,
        "add_share_row_v0" => build_share_row(args, false)?,
        "add_share_row_v1" => build_share_row(args, true)?,
        "add_social_login_row_v0" => build_social_login_row(args, false)?,
        "add_social_login_row_v1" => build_social_login_row(args, true)?,
        "add_data_table_row_v0" => build_data_table_row(args, false)?,
        "add_data_table_row_v1" => build_data_table_row(args, true)?,
        "add_inline_action_v0" => build_inline_action(args, false)?,
        "add_inline_action_v1" => build_inline_action(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

// ===== add_invite_row =====

/// Resolve a v1 color slot. `light_hex` is the byte-parity v0 value used
/// when `theme_aware == false` or `theme == "light"`; otherwise return the
/// dark hex (theme `"dark"`) or the `$color-*` ref (theme `"system"`).
fn v1_color<'a>(
    theme_aware: bool,
    theme: &str,
    light_hex: &'a str,
    dark_hex: &'a str,
    system_ref: &'a str,
) -> &'a str {
    if !theme_aware || theme == "light" {
        light_hex
    } else if theme == "system" {
        system_ref
    } else {
        dark_hex
    }
}

/// `add_invite_row_v0` / `_v1` — pending-invite list row:
/// avatar (email initial) + (email over optional role) + status pill +
/// trailing action label. Mirrors `invite-row.ts` / `invite-row-v1.ts`.
fn build_invite_row(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let email = required(args, "email")?;
    let theme = opt(args, "theme").unwrap_or("light");

    // Status enum: v0 throws on invalid, v1 coerces to "pending".
    let requested = opt(args, "status").unwrap_or("pending");
    let status = match requested {
        "pending" | "expired" | "accepted" => requested,
        _ if theme_aware => "pending",
        _ => {
            return Err(ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "add_invite_row_v0: invalid status \"{requested}\"; expected one of: pending, expired, accepted"
                ),
            ))
        }
    };

    let action_label = opt(args, "action_label").unwrap_or("Resend");
    let role = opt(args, "role").filter(|r| !r.is_empty());
    // initial = (email.charAt(0) ?? '?').toUpperCase()
    let initial = email
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());

    // Color slots.
    let name_fg = v1_color(
        theme_aware,
        theme,
        "#0F172A",
        "#F1F5F9",
        "$color-text-primary",
    );
    let sub_fg = v1_color(
        theme_aware,
        theme,
        "#64748B",
        "#94A3B8",
        "$color-text-muted",
    );
    let avatar_bg = v1_color(theme_aware, theme, "#E2E8F0", "#334155", "$color-surface-2");
    let action_fg = v1_color(theme_aware, theme, "#2563EB", "#60A5FA", "$color-accent");

    // Status pill colors. Light uses the v0 STATUS_TONE table; dark/system
    // map onto alert tokens.
    let (pill_bg, pill_fg, pill_text) = match status {
        "accepted" => (
            v1_color(
                theme_aware,
                theme,
                "#DCFCE7",
                "#14532D",
                "$color-success-bg",
            ),
            v1_color(
                theme_aware,
                theme,
                "#166534",
                "#BBF7D0",
                "$color-success-text",
            ),
            "Accepted",
        ),
        "expired" => (
            v1_color(theme_aware, theme, "#FEE2E2", "#7F1D1D", "$color-danger-bg"),
            v1_color(
                theme_aware,
                theme,
                "#991B1B",
                "#FECACA",
                "$color-danger-text",
            ),
            "Expired",
        ),
        _ => (
            v1_color(
                theme_aware,
                theme,
                "#FEF3C7",
                "#78350F",
                "$color-warning-bg",
            ),
            v1_color(
                theme_aware,
                theme,
                "#92400E",
                "#FDE68A",
                "$color-warning-text",
            ),
            "Pending",
        ),
    };

    let avatar = json!({
        "id": next_id("invite_row_avatar"),
        "type": "frame",
        "name": "Avatar",
        "role": "invite-row-avatar",
        "width": 36,
        "height": 36,
        "cornerRadius": 18,
        "fill": [{ "type": "solid", "color": avatar_bg }],
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "children": [{
            "id": next_id("invite_row_initial"),
            "type": "text",
            "name": "Initial",
            "role": "invite-row-avatar-initial",
            "content": initial,
            "fontSize": 14,
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": sub_fg }],
        }],
    });

    let mut text_stack_children = vec![json!({
        "id": next_id("invite_row_email"),
        "type": "text",
        "name": "Email",
        "role": "invite-row-email",
        "content": email,
        "fontSize": 14,
        "fontWeight": 500,
        "width": "fill_container",
        "textGrowth": "fixed-width",
        "fill": [{ "type": "solid", "color": name_fg }],
    })];
    if let Some(role) = role {
        text_stack_children.push(json!({
            "id": next_id("invite_row_role"),
            "type": "text",
            "name": "Role",
            "role": "invite-row-role",
            "content": role,
            "fontSize": 12,
            "fontWeight": 400,
            "width": "fill_container",
            "textGrowth": "fixed-width",
            "fill": [{ "type": "solid", "color": sub_fg }],
        }));
    }

    let status_pill = json!({
        "id": next_id("invite_row_status"),
        "type": "frame",
        "name": "Status Pill",
        "role": "invite-row-status",
        "width": "fit_content",
        "height": "fit_content",
        "cornerRadius": 12,
        "fill": [{ "type": "solid", "color": pill_bg }],
        "padding": [3, 10],
        "children": [{
            "id": next_id("invite_row_status_text"),
            "type": "text",
            "name": "Status Text",
            "role": "invite-row-status-text",
            "content": pill_text,
            "fontSize": 12,
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": pill_fg }],
        }],
    });

    let action = json!({
        "id": next_id("invite_row_action"),
        "type": "text",
        "name": "Action",
        "role": "invite-row-action",
        "content": action_label,
        "fontSize": 14,
        "fontWeight": 600,
        "fill": [{ "type": "solid", "color": action_fg }],
    });

    Ok(json!({
        "id": next_id("invite_row_root"),
        "type": "frame",
        "name": "Invite Row",
        "role": "invite-row",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 12,
        "padding": [12, 16],
        "children": [
            avatar,
            {
                "id": next_id("invite_row_text"),
                "type": "frame",
                "name": "Text Stack",
                "role": "invite-row-text",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "gap": 2,
                "children": text_stack_children,
            },
            status_pill,
            action,
        ],
    }))
}

// ===== add_share_row =====

/// `add_share_row_v0` / `_v1` — social-share button row: horizontal list
/// of 40x40 circular icon buttons each labeled below. Mirrors
/// `share-row.ts` / `share-row-v1.ts`.
fn build_share_row(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let theme = opt(args, "theme").unwrap_or("light");

    // targets: v0 has no default (required); v1 coerces an empty/missing
    // array to the 3 default share targets.
    let mut targets =
        parse_object_items(args, "targets", &["label"], "targets[].label is required")
            .unwrap_or_default();
    if targets.is_empty() {
        if theme_aware {
            targets = vec![
                json!({ "label": "Twitter", "icon": "twitter" }),
                json!({ "label": "Facebook", "icon": "facebook" }),
                json!({ "label": "Copy Link", "icon": "link" }),
            ];
        } else {
            return Err(ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "targets is required".into(),
            ));
        }
    }

    let icon_bg = v1_color(theme_aware, theme, "#F1F5F9", "#334155", "$color-surface-2");
    let icon_color = v1_color(
        theme_aware,
        theme,
        "#475569",
        "#94A3B8",
        "$color-text-muted",
    );

    let children: Vec<Value> = targets
        .iter()
        .enumerate()
        .map(|(i, target)| {
            let label = string_field(target, "label").unwrap_or("");
            let icon = string_field(target, "icon").unwrap_or("");
            json!({
                "id": next_id("share_target"),
                "type": "frame",
                "name": format!("Target {}", i + 1),
                "role": "share-target",
                "width": "fit_content",
                "height": "fit_content",
                "layout": "vertical",
                "alignItems": "center",
                "gap": 6,
                "children": [
                    {
                        "id": next_id("share_target_icon"),
                        "type": "frame",
                        "name": "Icon Button",
                        "role": "share-target-icon",
                        "width": 40,
                        "height": 40,
                        "cornerRadius": 20,
                        "layout": "horizontal",
                        "alignItems": "center",
                        "justifyContent": "center",
                        "fill": [{ "type": "solid", "color": icon_bg }],
                        "children": [{
                            "id": next_id("share_target_glyph"),
                            "type": "icon_font",
                            "name": "Icon",
                            "iconFontName": icon,
                            "iconFontFamily": "lucide",
                            "width": 18,
                            "height": 18,
                            "fill": [{ "type": "solid", "color": icon_color }],
                        }],
                    },
                    {
                        "id": next_id("share_target_label"),
                        "type": "text",
                        "name": "Label",
                        "role": "share-target-label",
                        "content": label,
                        "fontSize": 11,
                        "fontWeight": 500,
                        "fill": [{ "type": "solid", "color": icon_color }],
                    },
                ],
            })
        })
        .collect();

    Ok(json!({
        "id": next_id("share_row_root"),
        "type": "frame",
        "name": "Share Row",
        "role": "share-row",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "start",
        "gap": 16,
        "children": children,
    }))
}

// ===== add_social_login_row =====

/// Known provider-name -> lucide icon slug map (shared by v0 + v1).
fn social_login_known_icon(lower_name: &str) -> Option<&'static str> {
    match lower_name {
        "google" => Some("chrome"),
        "apple" => Some("apple"),
        "microsoft" => Some("monitor"),
        "github" => Some("github"),
        "gitlab" => Some("git-branch"),
        "facebook" => Some("facebook"),
        "twitter" | "x" => Some("twitter"),
        "linkedin" => Some("linkedin"),
        "discord" => Some("message-circle"),
        "slack" => Some("hash"),
        "email" => Some("mail"),
        "phone" => Some("smartphone"),
        _ => None,
    }
}

/// Capitalize the first char of `name` for the "Continue with X" label,
/// preserving the caller's casing when it is already capitalized.
/// Mirrors the TS prettyLabel logic.
fn social_login_pretty_label(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => "Continue with ".to_string(),
        Some(first) => {
            // If already uppercase (charAt(0) === charAt(0).toUpperCase()),
            // keep the caller's casing.
            let first_upper = first.to_uppercase().to_string();
            if first.to_string() == first_upper {
                format!("Continue with {name}")
            } else {
                format!("Continue with {first_upper}{}", chars.as_str())
            }
        }
    }
}

/// `add_social_login_row_v0` / `_v1` — "Continue with Google / Apple"
/// provider button column (vertical) or icon-only pill row (horizontal).
/// Mirrors `social-login-row.ts` / `social-login-row-v1.ts`.
fn build_social_login_row(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let theme = opt(args, "theme").unwrap_or("light");

    // providers: each must carry a non-empty `name` (SocialLoginProvider.name).
    // When the arg is present we validate+use it; v0 throws on empty/missing,
    // v1 coerces empty/missing to [{name:google}]. A supplied named provider
    // must NOT be silently replaced by the default.
    let supplied = args
        .get("providers")
        .map(|raw| !raw.trim().is_empty())
        .unwrap_or(false);
    let mut providers = if supplied {
        parse_object_items(args, "providers", &["name"], "providers[].name is required")?
    } else {
        Vec::new()
    };
    if providers.is_empty() {
        if theme_aware {
            providers = vec![json!({ "name": "google" })];
        } else {
            return Err(ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "buildSocialLoginRow: providers array must not be empty".into(),
            ));
        }
    }
    providers.truncate(6);

    let orientation = opt(args, "orientation").unwrap_or("vertical");
    let is_vertical = orientation == "vertical";
    let width = number_arg(args, "width", 320.0, 200.0).floor();

    let button_bg = v1_color(theme_aware, theme, "#FFFFFF", "#1E293B", "$color-surface");
    let border_color = v1_color(theme_aware, theme, "#E2E8F0", "#334155", "$color-border");
    let icon_fill = v1_color(
        theme_aware,
        theme,
        "#334155",
        "#94A3B8",
        "$color-text-muted",
    );
    let label_color = v1_color(
        theme_aware,
        theme,
        "#0F172A",
        "#F1F5F9",
        "$color-text-primary",
    );

    let buttons: Vec<Value> = providers
        .iter()
        .map(|provider| {
            // providers[].name may be missing (best-effort); the schema is
            // advertised generic, so fall back to empty string.
            let name = string_field(provider, "name").unwrap_or("");
            let lower = name.to_lowercase();
            let icon = string_field(provider, "icon")
                .filter(|i| !i.is_empty())
                .or_else(|| social_login_known_icon(&lower))
                .unwrap_or("log-in");
            let pretty_label = social_login_pretty_label(name);

            let icon_node = json!({
                "id": next_id("social_login_icon"),
                "type": "icon_font",
                "name": format!("{name} Icon"),
                "role": "social-login-button-icon",
                "iconFontName": icon,
                "iconFontFamily": "lucide",
                "width": 20,
                "height": 20,
                "fill": [{ "type": "solid", "color": icon_fill }],
            });

            if is_vertical {
                json!({
                    "id": next_id("social_login_button"),
                    "type": "frame",
                    "name": format!("{name} Button"),
                    "role": "social-login-button",
                    "width": "fill_container",
                    "height": 48,
                    "cornerRadius": 12,
                    "layout": "horizontal",
                    "alignItems": "center",
                    "gap": 12,
                    "padding": [0, 16],
                    "fill": [{ "type": "solid", "color": button_bg }],
                    "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": border_color }] },
                    "children": [
                        icon_node,
                        {
                            "id": next_id("social_login_label"),
                            "type": "text",
                            "name": "Label",
                            "role": "social-login-button-label",
                            "content": pretty_label,
                            "fontSize": 14,
                            "fontWeight": 500,
                            "fill": [{ "type": "solid", "color": label_color }],
                        },
                    ],
                })
            } else {
                json!({
                    "id": next_id("social_login_button"),
                    "type": "frame",
                    "name": format!("{name} Button"),
                    "role": "social-login-button-compact",
                    "width": 48,
                    "height": 48,
                    "cornerRadius": 12,
                    "layout": "horizontal",
                    "alignItems": "center",
                    "justifyContent": "center",
                    "fill": [{ "type": "solid", "color": button_bg }],
                    "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": border_color }] },
                    "children": [icon_node],
                })
            }
        })
        .collect();

    Ok(json!({
        "id": next_id("social_login_row_root"),
        "type": "frame",
        "name": "Social Login Row",
        "role": "social-login-row",
        "width": if is_vertical { json!(width) } else { json!("fit_content") },
        "height": "fit_content",
        "layout": if is_vertical { "vertical" } else { "horizontal" },
        "alignItems": "center",
        "gap": 10,
        "children": buttons,
    }))
}

// ===== add_data_table_row =====

/// `add_data_table_row_v0` / `_v1` — desktop data-table row: N evenly
/// fill_container cells with 16px gap, optional header/selected styling.
/// Mirrors `data-table-row.ts` / `data-table-row-v1.ts`.
fn build_data_table_row(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let theme = opt(args, "theme").unwrap_or("light");

    // columns: each must carry a non-empty `content` (DataTableRowColumn).
    // v0 has no default (required); v1 coerces empty/missing to 3. A supplied
    // column set is validated+used, never silently replaced by the default.
    let supplied = args
        .get("columns")
        .map(|raw| !raw.trim().is_empty())
        .unwrap_or(false);
    let mut columns = if supplied {
        parse_object_items(
            args,
            "columns",
            &["content"],
            "columns[].content is required",
        )?
    } else {
        Vec::new()
    };
    if columns.is_empty() {
        if theme_aware {
            columns = vec![
                json!({ "content": "Col 1" }),
                json!({ "content": "Col 2" }),
                json!({ "content": "Col 3" }),
            ];
        } else {
            return Err(ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "columns is required".into(),
            ));
        }
    }

    let is_header = bool_arg(args, "header");
    let is_selected = !is_header && bool_arg(args, "selected");
    let cell_text_size = if is_header { 12 } else { 14 };
    let cell_text_weight = if is_header { 600 } else { 400 };
    let cell_text_color = if is_header {
        v1_color(
            theme_aware,
            theme,
            "#64748B",
            "#94A3B8",
            "$color-text-muted",
        )
    } else {
        v1_color(
            theme_aware,
            theme,
            "#0F172A",
            "#F1F5F9",
            "$color-text-primary",
        )
    };
    let row_height = if is_header { 40 } else { 48 };

    let children: Vec<Value> = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let content = string_field(col, "content").unwrap_or("");
            json!({
                "id": next_id("data_table_cell"),
                "type": "frame",
                "name": format!("Cell {}", i + 1),
                "role": if is_header { "data-table-header-cell" } else { "data-table-cell" },
                "width": "fill_container",
                "height": "fill_container",
                "layout": "horizontal",
                "alignItems": "center",
                "clipContent": true,
                "children": [{
                    "id": next_id("data_table_cell_text"),
                    "type": "text",
                    "name": "Cell Text",
                    "role": if is_header { "data-table-header-text" } else { "data-table-cell-text" },
                    "content": content,
                    "fontSize": cell_text_size,
                    "fontWeight": cell_text_weight,
                    "width": "fill_container",
                    "textGrowth": "fixed-width",
                    "fill": [{ "type": "solid", "color": cell_text_color }],
                }],
            })
        })
        .collect();

    let mut node = json!({
        "id": next_id("data_table_row_root"),
        "type": "frame",
        "name": if is_header { "Data Table Header Row" } else { "Data Table Row" },
        "role": if is_header { "data-table-header-row" } else { "data-table-row" },
        "width": "fill_container",
        "height": row_height,
        "layout": "horizontal",
        "padding": [0, 16],
        "gap": 16,
        "alignItems": "center",
        "clipContent": true,
        "children": children,
    });

    if is_selected {
        let sel_bg = v1_color(theme_aware, theme, "#F8FAFC", "#0F172A", "$color-bg-deep");
        node["fill"] = json!([{ "type": "solid", "color": sel_bg }]);
    }

    Ok(node)
}

// ===== add_inline_action =====

/// `add_inline_action_v0` / `_v1` — inline status + action row: message
/// (optional leading icon) on the left, blue CTA text on the right.
/// Mirrors `inline-action.ts` / `inline-action-v1.ts`.
fn build_inline_action(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let message = required(args, "message")?;
    let action_label = required(args, "action_label")?;
    let theme = opt(args, "theme").unwrap_or("light");

    let icon_color = v1_color(theme_aware, theme, "#64748B", "#CBD5E1", "$color-text-body");
    let message_color = v1_color(theme_aware, theme, "#475569", "#CBD5E1", "$color-text-body");
    let cta_color = v1_color(theme_aware, theme, "#2563EB", "#60A5FA", "$color-accent");

    let mut left_children = Vec::new();
    if let Some(icon) = opt(args, "icon").filter(|i| !i.is_empty()) {
        left_children.push(json!({
            "id": next_id("inline_action_icon"),
            "type": "icon_font",
            "name": "Icon",
            "role": "inline-action-icon",
            "iconFontName": icon,
            "iconFontFamily": "lucide",
            "width": 16,
            "height": 16,
            "fill": [{ "type": "solid", "color": icon_color }],
        }));
    }
    left_children.push(json!({
        "id": next_id("inline_action_message"),
        "type": "text",
        "name": "Message",
        "role": "inline-action-message",
        "content": message,
        "fontSize": 13,
        "fontWeight": 400,
        "fill": [{ "type": "solid", "color": message_color }],
    }));

    Ok(json!({
        "id": next_id("inline_action_root"),
        "type": "frame",
        "name": "Inline Action",
        "role": "inline-action",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 12,
        "children": [
            {
                "id": next_id("inline_action_message_group"),
                "type": "frame",
                "name": "Message Group",
                "role": "inline-action-message-group",
                "width": "fit_content",
                "height": "fit_content",
                "layout": "horizontal",
                "alignItems": "center",
                "gap": 6,
                "children": left_children,
            },
            {
                "id": next_id("inline_action_cta"),
                "type": "text",
                "name": "Action",
                "role": "inline-action-cta",
                "content": action_label,
                "fontSize": 13,
                "fontWeight": 600,
                "fill": [{ "type": "solid", "color": cta_color }],
            },
        ],
    }))
}
