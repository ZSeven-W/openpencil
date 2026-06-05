//! Flow, list, and navigation TS element alias builders.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::{ToolErrorCode, ToolOutcome};

static NEXT_FLOW_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn flow_alias_node_value(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<Value>, ToolOutcome> {
    let value = match tool {
        "add_activity_log_v0" => build_activity_log(args, false)?,
        "add_activity_log_v1" => build_activity_log(args, true)?,
        "add_attachment_row_v0" => build_attachment_row(args, false)?,
        "add_attachment_row_v1" => build_attachment_row(args, true)?,
        "add_avatar_group_v0" => build_avatar_group(args, false)?,
        "add_avatar_group_v1" => build_avatar_group(args, true)?,
        "add_bottom_nav_v0" => build_bottom_nav(args, false)?,
        "add_bottom_nav_v1" => build_bottom_nav(args, true)?,
        "add_breadcrumb_v0" | "add_breadcrumb_v1" => build_breadcrumb(args)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn build_activity_log(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let actor = required(args, "actor")?;
    let action = required(args, "action")?;
    let timestamp = required(args, "timestamp")?;
    let tone = activity_tone(args.get("tone").map(String::as_str), theme_aware)?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = activity_log_colors(tone, theme, theme_aware);

    let mut row_children = Vec::new();
    if let Some(icon) = args.get("icon").filter(|icon| !icon.is_empty()) {
        row_children.push(json!({
            "id": next_id("activity_log_icon_dot"),
            "type": "frame",
            "name": "Icon Dot",
            "role": "activity-log-icon-dot",
            "width": 28,
            "height": 28,
            "cornerRadius": 14,
            "fill": [{ "type": "solid", "color": colors.tone_bg }],
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "children": [{
                "id": next_id("activity_log_icon"),
                "type": "icon_font",
                "name": "Icon",
                "role": "activity-log-icon",
                "iconFontName": icon,
                "iconFontFamily": "lucide",
                "width": 14,
                "height": 14,
                "fill": [{ "type": "solid", "color": colors.tone_fg }],
            }],
        }));
    }

    row_children.push(json!({
        "id": next_id("activity_log_body"),
        "type": "frame",
        "name": "Body",
        "role": "activity-log-body",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 0,
        "children": [{
            "id": next_id("activity_log_line"),
            "type": "text",
            "name": "Line",
            "role": "activity-log-line",
            "content": [
                { "text": actor, "fontWeight": 600, "fill": colors.actor_fg },
                { "text": format!(" {action}"), "fontWeight": 400, "fill": colors.action_fg },
            ],
            "fontSize": 14,
            "fontWeight": 400,
            "width": "fill_container",
            "textGrowth": "fixed-width",
            "fill": [{ "type": "solid", "color": colors.action_fg }],
        }],
    }));

    row_children.push(json!({
        "id": next_id("activity_log_timestamp"),
        "type": "text",
        "name": "Timestamp",
        "role": "activity-log-timestamp",
        "content": timestamp,
        "fontSize": 13,
        "fontWeight": 400,
        "fill": [{ "type": "solid", "color": colors.timestamp_fg }],
    }));

    Ok(json!({
        "id": next_id("activity_log"),
        "type": "frame",
        "name": "Activity Log Entry",
        "role": "activity-log",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 12,
        "padding": [10, 0],
        "children": row_children,
    }))
}

fn build_attachment_row(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let filename = required(args, "filename")?;
    let icon = args.get("icon").map(String::as_str).unwrap_or("file");
    let removable = bool_arg(args, "removable", true);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = attachment_colors(theme, theme_aware);

    let mut meta_children = vec![json!({
        "id": next_id("attachment_filename"),
        "type": "text",
        "name": "Filename",
        "role": "attachment-filename",
        "content": filename,
        "fontSize": 14,
        "fontWeight": 600,
        "fill": [{ "type": "solid", "color": colors.filename }],
    })];
    if let Some(size) = args.get("size").filter(|size| !size.is_empty()) {
        meta_children.push(json!({
            "id": next_id("attachment_size"),
            "type": "text",
            "name": "Size",
            "role": "attachment-size",
            "content": size,
            "fontSize": 12,
            "fontWeight": 400,
            "fill": [{ "type": "solid", "color": colors.size }],
        }));
    }

    let mut row_children = vec![
        icon_node(
            "Type Icon",
            Some("attachment-icon"),
            icon,
            24.0,
            24.0,
            Some(colors.icon),
        ),
        json!({
            "id": next_id("attachment_meta"),
            "type": "frame",
            "name": "Meta",
            "role": "attachment-meta",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "gap": 4,
            "children": meta_children,
        }),
    ];
    if removable {
        row_children.push(icon_node(
            "Remove",
            Some("attachment-remove"),
            "x",
            16.0,
            16.0,
            Some(colors.remove),
        ));
    }

    Ok(json!({
        "id": next_id("attachment"),
        "type": "frame",
        "name": "Attachment",
        "role": "attachment-row",
        "width": "fill_container",
        "height": "fit_content",
        "cornerRadius": 8,
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 10,
        "padding": [10, 12],
        "fill": [{ "type": "solid", "color": colors.surface }],
        "children": row_children,
    }))
}

fn build_avatar_group(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let size = number_arg(args, "size", 32.0).floor().clamp(24.0, 64.0);
    let max_visible = number_arg(args, "max_visible", 4.0)
        .floor()
        .clamp(1.0, 10.0) as usize;
    let items = if theme_aware {
        let parsed = optional_object_array_arg(args, "items")?;
        if parsed.is_empty() {
            vec![
                json!({ "initial": "A" }),
                json!({ "initial": "B" }),
                json!({ "initial": "C" }),
                json!({ "initial": "D" }),
                json!({ "initial": "E" }),
            ]
        } else {
            parsed
        }
    } else {
        optional_object_array_arg(args, "items")?
    };
    let visible = items.iter().take(max_visible).collect::<Vec<_>>();
    let overflow = items.len().saturating_sub(max_visible);
    let font_size = (size * 0.4).round().max(11.0);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = avatar_group_colors(theme, theme_aware);

    let mut children = Vec::new();
    for (idx, item) in visible.into_iter().enumerate() {
        children.push(avatar_group_tile(item, idx, size, font_size, &colors));
    }
    if overflow > 0 {
        children.push(json!({
            "id": next_id("avatar_group_overflow"),
            "type": "frame",
            "name": format!("Overflow +{overflow}"),
            "role": "avatar-group-overflow",
            "width": size,
            "height": size,
            "cornerRadius": size / 2.0,
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "fill": [{ "type": "solid", "color": colors.overflow_bg }],
            "stroke": avatar_ring(colors.ring),
            "children": [{
                "id": next_id("avatar_group_overflow_count"),
                "type": "text",
                "name": "Count",
                "role": "avatar-group-overflow-count",
                "content": format!("+{overflow}"),
                "fontSize": font_size,
                "fontWeight": 600,
                "fill": [{ "type": "solid", "color": colors.overflow_text }],
            }],
        }));
    }

    Ok(json!({
        "id": next_id("avatar_group"),
        "type": "frame",
        "name": "Avatar Group",
        "role": "avatar-group",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 4,
        "children": children,
    }))
}

fn build_bottom_nav(
    args: &BTreeMap<String, String>,
    coerce_icons: bool,
) -> Result<Value, ToolOutcome> {
    let items = object_array_arg(args, "items")?;
    let height = number_arg(args, "height", 62.0);
    let children = items
        .iter()
        .map(|item| bottom_nav_tab(item, coerce_icons))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "id": next_id("bottom_nav"),
        "type": "frame",
        "name": "Bottom Tab Bar",
        "role": "bottom-tab-bar",
        "width": "fill_container",
        "height": height,
        "layout": "horizontal",
        "justifyContent": "space_around",
        "alignItems": "center",
        "children": children,
    }))
}

fn build_breadcrumb(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let items = object_array_arg(args, "items")?;
    let last_idx = items.len().saturating_sub(1);
    let mut children = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let label = item_string(item, "label").unwrap_or_default();
        let active = item_bool(item, "active").unwrap_or(false) || idx == last_idx;
        children.push(json!({
            "id": next_id("breadcrumb_item"),
            "type": "text",
            "name": format!("Item ({label})"),
            "role": if active { "breadcrumb-item-active" } else { "breadcrumb-item" },
            "content": label,
            "fontSize": 13,
            "fontWeight": if active { 600 } else { 400 },
        }));
        if idx < last_idx {
            children.push(icon_node(
                "Separator",
                Some("breadcrumb-separator"),
                "chevron-right",
                14.0,
                14.0,
                None,
            ));
        }
    }
    Ok(json!({
        "id": next_id("breadcrumb"),
        "type": "frame",
        "name": "Breadcrumb",
        "role": "breadcrumb",
        "width": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 6,
        "children": children,
    }))
}

#[derive(Clone, Copy)]
enum ActivityTone {
    Info,
    Success,
    Warning,
    Danger,
    Neutral,
}

fn activity_tone(value: Option<&str>, coerce: bool) -> Result<ActivityTone, ToolOutcome> {
    match value.unwrap_or("info") {
        "info" => Ok(ActivityTone::Info),
        "success" => Ok(ActivityTone::Success),
        "warning" => Ok(ActivityTone::Warning),
        "danger" => Ok(ActivityTone::Danger),
        "neutral" => Ok(ActivityTone::Neutral),
        _ if coerce => Ok(ActivityTone::Info),
        other => Err(invalid_arg(format!(
            "add_activity_log_v0: invalid tone {other:?}; expected one of: info, success, warning, danger, neutral"
        ))),
    }
}

struct ActivityLogColors {
    actor_fg: &'static str,
    action_fg: &'static str,
    timestamp_fg: &'static str,
    tone_bg: &'static str,
    tone_fg: &'static str,
}

fn activity_log_colors(tone: ActivityTone, theme: &str, theme_aware: bool) -> ActivityLogColors {
    let (actor_fg, action_fg, timestamp_fg) = if !theme_aware || theme == "light" {
        ("#0F172A", "#475569", "#94A3B8")
    } else if theme == "system" {
        (
            "$color-text-primary",
            "$color-text-body",
            "$color-text-subtle",
        )
    } else {
        ("#F1F5F9", "#CBD5E1", "#64748B")
    };
    let (tone_bg, tone_fg) = tone_colors(tone, theme, theme_aware);
    ActivityLogColors {
        actor_fg,
        action_fg,
        timestamp_fg,
        tone_bg,
        tone_fg,
    }
}

fn tone_colors(tone: ActivityTone, theme: &str, theme_aware: bool) -> (&'static str, &'static str) {
    if !theme_aware || theme == "light" {
        return match tone {
            ActivityTone::Info => ("#DBEAFE", "#1D4ED8"),
            ActivityTone::Success => ("#DCFCE7", "#166534"),
            ActivityTone::Warning => ("#FEF3C7", "#92400E"),
            ActivityTone::Danger => ("#FEE2E2", "#991B1B"),
            ActivityTone::Neutral => ("#E2E8F0", "#475569"),
        };
    }
    if theme == "system" {
        return match tone {
            ActivityTone::Info => ("$color-info-bg", "$color-info-text"),
            ActivityTone::Success => ("$color-success-bg", "$color-success-text"),
            ActivityTone::Warning => ("$color-warning-bg", "$color-warning-text"),
            ActivityTone::Danger => ("$color-danger-bg", "$color-danger-text"),
            ActivityTone::Neutral => ("$color-surface", "$color-text-muted"),
        };
    }
    match tone {
        ActivityTone::Info => ("#1E3A8A", "#BFDBFE"),
        ActivityTone::Success => ("#14532D", "#BBF7D0"),
        ActivityTone::Warning => ("#78350F", "#FDE68A"),
        ActivityTone::Danger => ("#7F1D1D", "#FECACA"),
        ActivityTone::Neutral => ("#1E293B", "#94A3B8"),
    }
}

struct AttachmentColors {
    filename: &'static str,
    size: &'static str,
    icon: &'static str,
    remove: &'static str,
    surface: &'static str,
}

fn attachment_colors(theme: &str, theme_aware: bool) -> AttachmentColors {
    if !theme_aware || theme == "light" {
        return AttachmentColors {
            filename: "#0F172A",
            size: "#64748B",
            icon: "#64748B",
            remove: "#94A3B8",
            surface: "#F8FAFC",
        };
    }
    if theme == "system" {
        return AttachmentColors {
            filename: "$color-text-primary",
            size: "$color-text-muted",
            icon: "$color-text-muted",
            remove: "$color-text-subtle",
            surface: "$color-bg-deep",
        };
    }
    AttachmentColors {
        filename: "#F1F5F9",
        size: "#94A3B8",
        icon: "#94A3B8",
        remove: "#64748B",
        surface: "#0F172A",
    }
}

struct AvatarGroupColors {
    ring: &'static str,
    overflow_bg: &'static str,
    overflow_text: &'static str,
    initial_text: &'static str,
}

fn avatar_group_colors(theme: &str, theme_aware: bool) -> AvatarGroupColors {
    if !theme_aware || theme == "light" {
        return AvatarGroupColors {
            ring: "#FFFFFF",
            overflow_bg: "#F1F5F9",
            overflow_text: "#475569",
            initial_text: "#FFFFFF",
        };
    }
    if theme == "system" {
        return AvatarGroupColors {
            ring: "$color-surface",
            overflow_bg: "$color-surface-2",
            overflow_text: "$color-text-muted",
            initial_text: "$color-surface",
        };
    }
    AvatarGroupColors {
        ring: "#1E293B",
        overflow_bg: "#334155",
        overflow_text: "#94A3B8",
        initial_text: "#1E293B",
    }
}

fn avatar_group_tile(
    item: &Value,
    index: usize,
    size: f64,
    font_size: f64,
    colors: &AvatarGroupColors,
) -> Value {
    const PALETTE: [&str; 8] = [
        "#3B82F6", "#10B981", "#F59E0B", "#EF4444", "#8B5CF6", "#EC4899", "#14B8A6", "#F97316",
    ];
    let fill = item_string(item, "color").unwrap_or(PALETTE[index % PALETTE.len()]);
    let mut children = Vec::new();
    if let Some(initial) = item_string(item, "initial").filter(|initial| !initial.is_empty()) {
        children.push(json!({
            "id": next_id("avatar_group_initial"),
            "type": "text",
            "name": "Initial",
            "role": "avatar-group-initial",
            "content": initial,
            "fontSize": font_size,
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": colors.initial_text }],
        }));
    }
    json!({
        "id": next_id("avatar_group_item"),
        "type": "frame",
        "name": format!("Avatar {}", index + 1),
        "role": "avatar-group-item",
        "width": size,
        "height": size,
        "cornerRadius": size / 2.0,
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "fill": [{ "type": "solid", "color": fill }],
        "stroke": avatar_ring(colors.ring),
        "children": children,
    })
}

fn bottom_nav_tab(item: &Value, coerce_icons: bool) -> Result<Value, ToolOutcome> {
    let title = item_string(item, "title").unwrap_or_default();
    let icon = item_string(item, "icon").unwrap_or_default();
    let icon = if coerce_icons {
        coerce_nav_tab_icon(title, icon)
    } else {
        icon
    };
    let active = item_bool(item, "active").unwrap_or(false);
    Ok(json!({
        "id": next_id("bottom_nav_tab"),
        "type": "frame",
        "name": format!("Tab ({title})"),
        "role": if active { "nav-item-active" } else { "nav-item" },
        "width": "fit_content",
        "height": "fit_content",
        "layout": "vertical",
        "alignItems": "center",
        "gap": 4,
        "padding": [4, 12],
        "children": [
            icon_node("Icon", None, icon, 24.0, 24.0, None),
            {
                "id": next_id("bottom_nav_label"),
                "type": "text",
                "name": "Label",
                "role": "label",
                "content": title,
                "fontSize": 11,
                "fontWeight": if active { 600 } else { 500 },
            },
        ],
    }))
}

fn coerce_nav_tab_icon<'a>(title: &str, icon: &'a str) -> &'a str {
    let trimmed = title.trim();
    let lower = trimmed.to_ascii_lowercase();
    let canonical = match lower.as_str() {
        "cart" => Some("shopping-cart"),
        "bag" => Some("shopping-bag"),
        "home" => Some("house"),
        "search" => Some("search"),
        "profile" | "account" => Some("user"),
        "orders" => Some("clipboard-list"),
        "inbox" => Some("inbox"),
        "notifications" => Some("bell"),
        "messages" => Some("message-circle"),
        "settings" => Some("settings"),
        "favorites" | "likes" => Some("heart"),
        "explore" | "discover" => Some("compass"),
        _ => match trimmed {
            "购物车" => Some("shopping-cart"),
            "购物袋" => Some("shopping-bag"),
            "首页" => Some("house"),
            "搜索" => Some("search"),
            "我的" | "账户" => Some("user"),
            "订单" => Some("clipboard-list"),
            "收件箱" => Some("inbox"),
            "通知" => Some("bell"),
            "消息" => Some("message-circle"),
            "设置" => Some("settings"),
            "收藏" => Some("heart"),
            "发现" => Some("compass"),
            _ => None,
        },
    };
    let Some(canonical) = canonical else {
        return icon;
    };
    if icon == canonical {
        return icon;
    }
    let is_known_wrong = match canonical {
        "shopping-cart" => ["shopping-bag", "bag", "package", "tote"].contains(&icon),
        "user" => ["profile", "account", "avatar", "circle-user-round"].contains(&icon),
        "house" => ["home"].contains(&icon),
        "bell" => ["notification", "alarm"].contains(&icon),
        "message-circle" => ["message", "chat"].contains(&icon),
        "clipboard-list" => ["list", "orders", "receipt"].contains(&icon),
        _ => false,
    };
    if is_known_wrong {
        canonical
    } else {
        icon
    }
}

fn avatar_ring(color: &str) -> Value {
    json!({
        "thickness": 2,
        "fill": [{ "type": "solid", "color": color }],
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
    parse_object_array(key, raw)
}

fn optional_object_array_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Vec<Value>, ToolOutcome> {
    let Some(raw) = args.get(key).filter(|raw| !raw.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    parse_object_array(key, raw)
}

fn parse_object_array(key: &str, raw: &str) -> Result<Vec<Value>, ToolOutcome> {
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON object array: {e}"),
        )
    })?;
    let Some(values) = value.as_array() else {
        return Err(invalid_arg(format!("{key} must be a JSON object array")));
    };
    Ok(values
        .iter()
        .filter(|value| value.is_object())
        .cloned()
        .collect())
}

fn item_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn item_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
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

fn required<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, ToolOutcome> {
    args.get(key).map(String::as_str).ok_or_else(|| {
        ToolOutcome::Err(ToolErrorCode::MissingArgument, format!("{key} is required"))
    })
}

fn invalid_arg(message: impl Into<String>) -> ToolOutcome {
    ToolOutcome::Err(ToolErrorCode::InvalidArgument, message.into())
}

fn next_id(prefix: &str) -> String {
    let n = NEXT_FLOW_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_flow_{prefix}_{n}")
}
