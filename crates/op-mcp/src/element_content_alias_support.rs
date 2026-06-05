//! Shared helpers for content/card alias builders.

#![allow(clippy::result_large_err)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::{ToolErrorCode, ToolOutcome};

static NEXT_CONTENT_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn scroll_wrapper(row_name: &str, gap: f64, inner_children: Vec<Value>) -> Value {
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

pub(crate) fn card_row_card(item: &Value, card_width: f64, colors: &ContentColors) -> Value {
    let mut children = Vec::new();
    if let Some(icon) = item_string(item, "icon").filter(|icon| !icon.is_empty()) {
        children.push(icon_node("Icon", None, icon, 24.0, 24.0, None));
    }
    let mut title = json!({
        "id": next_id("card_title"),
        "type": "text",
        "name": "Title",
        "role": "heading",
        "content": item_string(item, "title").unwrap_or_default(),
        "fontSize": 16,
        "fontWeight": 600,
        "width": "fill_container",
    });
    if let Some(fill) = colors.title {
        title["fill"] = json!([{ "type": "solid", "color": fill }]);
    }
    children.push(title);
    if let Some(subtitle) = item_string(item, "subtitle").filter(|subtitle| !subtitle.is_empty()) {
        let mut subtitle_node = json!({
            "id": next_id("card_subtitle"),
            "type": "text",
            "name": "Subtitle",
            "role": "body",
            "content": subtitle,
            "fontSize": 13,
            "fontWeight": 400,
            "width": "fill_container",
        });
        if let Some(fill) = colors.subtitle {
            subtitle_node["fill"] = json!([{ "type": "solid", "color": fill }]);
        }
        children.push(subtitle_node);
    }
    let mut card = json!({
        "id": next_id("card"),
        "type": "frame",
        "name": "Card",
        "role": "card",
        "width": card_width,
        "height": 160,
        "cornerRadius": 20,
        "padding": 16,
        "layout": "vertical",
        "gap": 8,
        "children": children,
    });
    if let Some(fill) = colors.card_bg {
        card["fill"] = json!([{ "type": "solid", "color": fill }]);
    }
    card
}

pub(crate) fn event_meta_row(
    row_name: &str,
    icon_role: &str,
    icon: &str,
    text_name: &str,
    text_role: &str,
    content: &str,
    fill: &str,
) -> Value {
    json!({
        "id": next_id("event_meta_row"),
        "type": "frame",
        "name": row_name,
        "role": "event-card-meta-row",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 6,
        "children": [
            icon_node(
                if icon == "clock" { "Time Icon" } else { "Location Icon" },
                Some(icon_role),
                icon,
                14.0,
                14.0,
                Some(fill),
            ),
            {
                "id": next_id("event_meta_text"),
                "type": "text",
                "name": text_name,
                "role": text_role,
                "content": content,
                "fontSize": 13,
                "fontWeight": 400,
                "fill": [{ "type": "solid", "color": fill }],
            },
        ],
    })
}

pub(crate) struct ContentColors {
    pub(crate) card_bg: Option<&'static str>,
    pub(crate) title: Option<&'static str>,
    pub(crate) subtitle: Option<&'static str>,
}

pub(crate) fn content_colors(theme: &str, theme_aware: bool) -> ContentColors {
    if !theme_aware || theme == "light" {
        return ContentColors {
            card_bg: None,
            title: None,
            subtitle: None,
        };
    }
    if theme == "system" {
        return ContentColors {
            card_bg: Some("$color-surface"),
            title: Some("$color-text-primary"),
            subtitle: Some("$color-text-muted"),
        };
    }
    ContentColors {
        card_bg: Some("#1E293B"),
        title: Some("#F1F5F9"),
        subtitle: Some("#94A3B8"),
    }
}

pub(crate) struct CommentColors {
    pub(crate) avatar_bg: &'static str,
    pub(crate) initial: &'static str,
    pub(crate) timestamp: &'static str,
}

pub(crate) fn comment_colors(theme: &str, theme_aware: bool) -> CommentColors {
    if !theme_aware || theme == "light" {
        return CommentColors {
            avatar_bg: "#E2E8F0",
            initial: "#475569",
            timestamp: "#64748B",
        };
    }
    if theme == "system" {
        return CommentColors {
            avatar_bg: "$color-surface-2",
            initial: "$color-text-body",
            timestamp: "$color-text-muted",
        };
    }
    CommentColors {
        avatar_bg: "#334155",
        initial: "#CBD5E1",
        timestamp: "#94A3B8",
    }
}

pub(crate) struct ChatColors<'a> {
    pub(crate) self_bg: Cow<'a, str>,
    pub(crate) other_bg: &'static str,
    pub(crate) other_text: &'static str,
    pub(crate) muted: &'static str,
}

pub(crate) fn chat_colors<'a>(
    theme: &str,
    theme_aware: bool,
    accent: Option<&'a str>,
) -> ChatColors<'a> {
    if !theme_aware || theme == "light" {
        return ChatColors {
            self_bg: Cow::Borrowed(accent.unwrap_or("#2563EB")),
            other_bg: "#F1F5F9",
            other_text: "#0F172A",
            muted: "#64748B",
        };
    }
    if theme == "system" {
        return ChatColors {
            self_bg: Cow::Borrowed("$color-accent"),
            other_bg: "$color-surface-2",
            other_text: "$color-text-primary",
            muted: "$color-text-muted",
        };
    }
    ChatColors {
        self_bg: Cow::Borrowed("#60A5FA"),
        other_bg: "#334155",
        other_text: "#F1F5F9",
        muted: "#94A3B8",
    }
}

pub(crate) struct FaqColors {
    pub(crate) question: Option<&'static str>,
    pub(crate) chevron: &'static str,
    pub(crate) answer: &'static str,
    pub(crate) divider: &'static str,
}

pub(crate) fn faq_colors(theme: &str, theme_aware: bool) -> FaqColors {
    if !theme_aware || theme == "light" {
        return FaqColors {
            question: None,
            chevron: "#64748B",
            answer: "#475569",
            divider: "#E2E8F0",
        };
    }
    if theme == "system" {
        return FaqColors {
            question: Some("$color-text-primary"),
            chevron: "$color-text-muted",
            answer: "$color-text-muted",
            divider: "$color-border",
        };
    }
    FaqColors {
        question: Some("#F1F5F9"),
        chevron: "#94A3B8",
        answer: "#94A3B8",
        divider: "#334155",
    }
}

pub(crate) struct EventColors {
    pub(crate) card_bg: &'static str,
    pub(crate) border: &'static str,
    pub(crate) date_bg: &'static str,
    pub(crate) title: &'static str,
    pub(crate) day: &'static str,
    pub(crate) meta: &'static str,
}

pub(crate) fn event_colors(theme: &str, theme_aware: bool) -> EventColors {
    if !theme_aware || theme == "light" {
        return EventColors {
            card_bg: "#FFFFFF",
            border: "#E2E8F0",
            date_bg: "#F1F5F9",
            title: "#0F172A",
            day: "#0F172A",
            meta: "#64748B",
        };
    }
    if theme == "system" {
        return EventColors {
            card_bg: "$color-surface",
            border: "$color-border",
            date_bg: "$color-surface-2",
            title: "$color-text-primary",
            day: "$color-text-primary",
            meta: "$color-text-muted",
        };
    }
    EventColors {
        card_bg: "#1E293B",
        border: "#334155",
        date_bg: "#334155",
        title: "#F1F5F9",
        day: "#F1F5F9",
        meta: "#94A3B8",
    }
}

pub(crate) fn icon_node(
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

pub(crate) fn object_array_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Vec<Value>, ToolOutcome> {
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

pub(crate) fn item_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

pub(crate) fn bool_arg(args: &BTreeMap<String, String>, key: &str, default: bool) -> bool {
    args.get(key)
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(default)
}

pub(crate) fn number_arg(args: &BTreeMap<String, String>, key: &str, default: f64) -> f64 {
    args.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}

pub(crate) fn required<'a>(
    args: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, ToolOutcome> {
    args.get(key).map(String::as_str).ok_or_else(|| {
        ToolOutcome::Err(ToolErrorCode::MissingArgument, format!("{key} is required"))
    })
}

pub(crate) fn next_id(prefix: &str) -> String {
    let n = NEXT_CONTENT_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_content_{prefix}_{n}")
}
