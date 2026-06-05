//! Feedback and media placeholder TS element alias builders.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::{ToolErrorCode, ToolOutcome};

static NEXT_FEEDBACK_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn feedback_alias_node_value(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<Value>, ToolOutcome> {
    let value = match tool {
        "add_alert_v0" | "add_alert_v1" => build_alert(args)?,
        "add_callout_v0" => build_callout(args, false)?,
        "add_callout_v1" => build_callout(args, true)?,
        "add_toast_v0" => build_toast(args, false)?,
        "add_toast_v1" => build_toast(args, true)?,
        "add_empty_state_v0" | "add_empty_state_v1" => build_empty_state(args)?,
        "add_image_placeholder_v0" => build_image_placeholder(args, false)?,
        "add_image_placeholder_v1" => build_image_placeholder(args, true)?,
        "add_video_placeholder_v0" | "add_video_placeholder_v1" => build_video_placeholder(args)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn build_alert(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let message = required(args, "message")?;
    let mut children = Vec::new();
    if let Some(icon) = args.get("icon").filter(|icon| !icon.is_empty()) {
        children.push(icon_node("Leading Icon", None, icon, 20.0, 20.0, None));
    }
    children.push(json!({
        "id": next_id("alert_message"),
        "type": "text",
        "name": "Message",
        "role": "alert-message",
        "content": message,
        "fontSize": 14,
        "fontWeight": 400,
    }));
    if bool_arg(args, "dismissible", false) {
        children.push(icon_node(
            "Close",
            Some("alert-close"),
            "x",
            16.0,
            16.0,
            None,
        ));
    }
    Ok(json!({
        "id": next_id("alert"),
        "type": "frame",
        "name": "Alert",
        "role": "alert",
        "width": "fill_container",
        "cornerRadius": 8,
        "padding": [12, 16],
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 12,
        "children": children,
    }))
}

fn build_callout(args: &BTreeMap<String, String>, theme_aware: bool) -> Result<Value, ToolOutcome> {
    let body = required(args, "body")?;
    let tone = callout_tone(args.get("tone").map(String::as_str), theme_aware)?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let (bg, fg, icon) = callout_colors(tone, theme, theme_aware);

    let mut stack_children = Vec::new();
    if let Some(title) = args.get("title").filter(|title| !title.is_empty()) {
        stack_children.push(json!({
            "id": next_id("callout_title"),
            "type": "text",
            "name": "Title",
            "role": "callout-title",
            "content": title,
            "fontSize": 14,
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": fg }],
        }));
    }
    stack_children.push(json!({
        "id": next_id("callout_body"),
        "type": "text",
        "name": "Body",
        "role": "callout-body",
        "content": body,
        "fontSize": 13,
        "fontWeight": 400,
        "lineHeight": 1.5,
        "fill": [{ "type": "solid", "color": fg }],
        "width": "fill_container",
        "textGrowth": "fixed-width",
    }));

    Ok(json!({
        "id": next_id("callout"),
        "type": "frame",
        "name": "Callout",
        "role": "callout",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "start",
        "gap": 12,
        "padding": [12, 16],
        "cornerRadius": 8,
        "fill": [{ "type": "solid", "color": bg }],
        "children": [
            icon_node("Tone Icon", Some("callout-icon"), icon, 18.0, 18.0, Some(fg)),
            {
                "id": next_id("callout_text"),
                "type": "frame",
                "name": "Text Stack",
                "role": "callout-text",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "gap": 4,
                "children": stack_children,
            },
        ],
    }))
}

fn build_toast(args: &BTreeMap<String, String>, theme_aware: bool) -> Result<Value, ToolOutcome> {
    let message = required(args, "message")?;
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let (pill_fill, fg, font_size, gap) = if theme_aware {
        match theme {
            "dark" => (json!("#F1F5F9"), json!("#0F172A"), json!(14), json!(8)),
            "system" => (
                json!("$color-text-primary"),
                json!("$color-surface"),
                json!("$type-body-size"),
                json!("$spacing-2"),
            ),
            _ => (json!("#111827"), json!("#FFFFFF"), json!(14), json!(8)),
        }
    } else {
        (json!("#111827"), json!("#FFFFFF"), json!(14), json!(8))
    };

    let mut children = Vec::new();
    if let Some(icon) = args.get("icon").filter(|icon| !icon.is_empty()) {
        children.push(icon_node(
            "Leading Icon",
            None,
            icon,
            18.0,
            18.0,
            fg.as_str(),
        ));
    }
    children.push(json!({
        "id": next_id("toast_message"),
        "type": "text",
        "name": "Message",
        "role": "toast-message",
        "content": message,
        "fontSize": font_size,
        "fontWeight": 500,
        "fill": [{ "type": "solid", "color": fg }],
    }));

    Ok(json!({
        "id": next_id("toast"),
        "type": "frame",
        "name": "Toast",
        "role": "toast",
        "width": "fit_content",
        "cornerRadius": 24,
        "padding": [12, 20],
        "fill": [{ "type": "solid", "color": pill_fill }],
        "layout": "horizontal",
        "alignItems": "center",
        "gap": gap,
        "children": children,
    }))
}

fn build_empty_state(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let title = required(args, "title")?;
    let mut children = Vec::new();
    if let Some(icon) = args.get("icon").filter(|icon| !icon.is_empty()) {
        children.push(icon_node(
            "Icon",
            Some("empty-state-icon"),
            icon,
            48.0,
            48.0,
            None,
        ));
    }
    children.push(json!({
        "id": next_id("empty_state_title"),
        "type": "text",
        "name": "Title",
        "role": "empty-state-title",
        "content": title,
        "fontSize": 18,
        "fontWeight": 600,
    }));
    if let Some(subtitle) = args.get("subtitle").filter(|subtitle| !subtitle.is_empty()) {
        children.push(json!({
            "id": next_id("empty_state_subtitle"),
            "type": "text",
            "name": "Subtitle",
            "role": "empty-state-subtitle",
            "content": subtitle,
            "fontSize": 14,
            "fontWeight": 400,
        }));
    }
    if let Some(cta) = args.get("cta_label").filter(|cta| !cta.is_empty()) {
        children.push(json!({
            "id": next_id("empty_state_cta"),
            "type": "frame",
            "name": "CTA",
            "role": "button",
            "cornerRadius": 24,
            "padding": [12, 24],
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": "center",
            "children": [{
                "id": next_id("empty_state_cta_label"),
                "type": "text",
                "name": "CTA Label",
                "role": "label",
                "content": cta,
                "fontSize": 14,
                "fontWeight": 500,
            }],
        }));
    }
    Ok(json!({
        "id": next_id("empty_state"),
        "type": "frame",
        "name": "Empty State",
        "role": "empty-state",
        "width": "fill_container",
        "layout": "vertical",
        "alignItems": "center",
        "justifyContent": "center",
        "gap": 16,
        "padding": [48, 24],
        "children": children,
    }))
}

fn build_image_placeholder(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let width = number_arg(args, "width", 200.0).floor().max(40.0);
    let height = number_arg(args, "height", 140.0).floor().max(40.0);
    let icon = args.get("icon").map(String::as_str).unwrap_or("image");
    let corner_radius = number_arg(args, "corner_radius", 8.0).floor().max(0.0);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let (frame_bg, icon_color, label_color) = image_placeholder_colors(theme, theme_aware);

    let mut children = vec![icon_node(
        "Placeholder Icon",
        Some("image-placeholder-icon"),
        icon,
        40.0,
        40.0,
        Some(icon_color),
    )];
    if let Some(label) = args.get("label").filter(|label| !label.is_empty()) {
        children.push(json!({
            "id": next_id("image_placeholder_label"),
            "type": "text",
            "name": "Label",
            "role": "image-placeholder-label",
            "content": label,
            "fontSize": 13,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": label_color }],
        }));
    }

    let mut frame = json!({
        "id": next_id("image_placeholder"),
        "type": "frame",
        "name": "Image Placeholder",
        "role": "image-placeholder",
        "width": width,
        "height": height,
        "cornerRadius": corner_radius,
        "layout": "vertical",
        "alignItems": "center",
        "justifyContent": "center",
        "gap": 8,
        "fill": [{ "type": "solid", "color": frame_bg }],
        "children": children,
    });
    if let Some(query) = args
        .get("image_search_query")
        .filter(|query| !query.is_empty())
    {
        frame["imageSearchQuery"] = json!(query);
    }
    Ok(frame)
}

fn build_video_placeholder(args: &BTreeMap<String, String>) -> Result<Value, ToolOutcome> {
    let width = number_arg(args, "width", 320.0).floor().max(80.0);
    let height = number_arg(args, "height", 180.0).floor().max(60.0);
    let corner_radius = number_arg(args, "corner_radius", 12.0).floor().max(0.0);
    let mut children = vec![icon_node(
        "Play Icon",
        Some("video-placeholder-icon"),
        "play",
        48.0,
        48.0,
        Some("#FFFFFF"),
    )];
    if let Some(label) = args.get("label").filter(|label| !label.is_empty()) {
        children.push(json!({
            "id": next_id("video_placeholder_label"),
            "type": "text",
            "name": "Label",
            "role": "video-placeholder-label",
            "content": label,
            "fontSize": 13,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": "#FFFFFFB3" }],
        }));
    }
    Ok(json!({
        "id": next_id("video_placeholder"),
        "type": "frame",
        "name": "Video Placeholder",
        "role": "video-placeholder",
        "width": width,
        "height": height,
        "cornerRadius": corner_radius,
        "layout": "vertical",
        "alignItems": "center",
        "justifyContent": "center",
        "gap": 8,
        "fill": [{ "type": "solid", "color": "#334155" }],
        "children": children,
    }))
}

#[derive(Clone, Copy)]
enum CalloutTone {
    Info,
    Success,
    Warning,
    Danger,
    Note,
}

fn callout_tone(raw: Option<&str>, coerce_invalid: bool) -> Result<CalloutTone, ToolOutcome> {
    match raw.unwrap_or("note") {
        "info" => Ok(CalloutTone::Info),
        "success" => Ok(CalloutTone::Success),
        "warning" => Ok(CalloutTone::Warning),
        "danger" => Ok(CalloutTone::Danger),
        "note" => Ok(CalloutTone::Note),
        other if coerce_invalid => {
            let _ = other;
            Ok(CalloutTone::Note)
        }
        other => Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!(
                "add_callout_v0: invalid tone {other:?}; expected one of: info, success, warning, danger, note"
            ),
        )),
    }
}

fn callout_colors(
    tone: CalloutTone,
    theme: &str,
    theme_aware: bool,
) -> (&'static str, &'static str, &'static str) {
    if !theme_aware || theme == "light" {
        return match tone {
            CalloutTone::Info => ("#DBEAFE", "#1E40AF", "info"),
            CalloutTone::Success => ("#DCFCE7", "#166534", "check-circle"),
            CalloutTone::Warning => ("#FEF3C7", "#92400E", "alert-triangle"),
            CalloutTone::Danger => ("#FEE2E2", "#991B1B", "alert-octagon"),
            CalloutTone::Note => ("#F1F5F9", "#0F172A", "sticky-note"),
        };
    }
    if theme == "system" {
        return match tone {
            CalloutTone::Info => ("$color-info-bg", "$color-info-text", "info"),
            CalloutTone::Success => ("$color-success-bg", "$color-success-text", "check-circle"),
            CalloutTone::Warning => ("$color-warning-bg", "$color-warning-text", "alert-triangle"),
            CalloutTone::Danger => ("$color-danger-bg", "$color-danger-text", "alert-octagon"),
            CalloutTone::Note => ("$color-surface-2", "$color-text-primary", "sticky-note"),
        };
    }
    match tone {
        CalloutTone::Info => ("#1E3A8A", "#BFDBFE", "info"),
        CalloutTone::Success => ("#14532D", "#BBF7D0", "check-circle"),
        CalloutTone::Warning => ("#78350F", "#FDE68A", "alert-triangle"),
        CalloutTone::Danger => ("#7F1D1D", "#FECACA", "alert-octagon"),
        CalloutTone::Note => ("#334155", "#F1F5F9", "sticky-note"),
    }
}

fn image_placeholder_colors(
    theme: &str,
    theme_aware: bool,
) -> (&'static str, &'static str, &'static str) {
    if !theme_aware || theme == "light" {
        return ("#F1F5F9", "#94A3B8", "#64748B");
    }
    if theme == "system" {
        return ("$color-bg-deep", "$color-text-muted", "$color-text-muted");
    }
    ("#0F172A", "#94A3B8", "#94A3B8")
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

fn bool_arg(args: &BTreeMap<String, String>, key: &str, default: bool) -> bool {
    args.get(key)
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(default)
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
    let n = NEXT_FEEDBACK_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_feedback_{prefix}_{n}")
}
