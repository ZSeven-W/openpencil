//! Ported TS element builders (inputs). Real role-tagged subtrees
//! for kinds that previously fell to the generic kit placeholder.
#![allow(clippy::result_large_err)]

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::element_ported_helpers::*;
use crate::ToolOutcome;

pub(crate) fn ported_inputs_alias_node_value(
    tool: &str,
    args: &std::collections::BTreeMap<String, String>,
) -> Result<Option<serde_json::Value>, crate::ToolOutcome> {
    let value = match tool {
        "add_phone_input_v0" => build_phone_input(args, false)?,
        "add_phone_input_v1" => build_phone_input(args, true)?,
        "add_range_slider_v0" => build_range_slider(args, false)?,
        "add_range_slider_v1" => build_range_slider(args, true)?,
        "add_search_bar_v0" => build_search_bar(args, false)?,
        "add_search_bar_v1" => build_search_bar(args, true)?,
        "add_select_v0" => build_select(args, false)?,
        "add_select_v1" => build_select(args, true)?,
        "add_upload_dropzone_v0" => build_upload_dropzone(args, false)?,
        "add_upload_dropzone_v1" => build_upload_dropzone(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

// ===== add_phone_input =====

/// `add_phone_input_v0` / `_v1` — phone-number input with leading country-code
/// selector. Ports `phone-input.ts` (v0 == v1 light) + `phone-input-v1.ts`.
fn build_phone_input(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let width = number_arg(args, "width", 320.0, 240.0).floor();
    let code = opt(args, "country_code").unwrap_or("+1");
    let placeholder = opt(args, "placeholder").unwrap_or("(555) 555-5555");
    // TS: isFilled = value !== undefined && value !== ''
    let value = opt(args, "value").filter(|v| !v.is_empty());
    let is_filled = value.is_some();
    let digits_content = value.unwrap_or(placeholder);

    let theme = opt(args, "theme").unwrap_or("light");
    let is_light = !theme_aware || theme == "light";
    let c = ported_theme(theme_aware, theme);

    let field_bg = if is_light { "#FFFFFF" } else { c.surface };
    let stroke_color = if is_light { "#CBD5E1" } else { c.border };
    let divider_color = if is_light { "#E2E8F0" } else { c.border };
    let label_color = if is_light { "#334155" } else { c.text_body };
    let code_color = if is_light { "#0F172A" } else { c.text_primary };
    let chevron_color = if is_light { "#64748B" } else { c.text_muted };
    let digits_color = if is_filled {
        if is_light {
            "#0F172A"
        } else {
            c.text_primary
        }
    } else if is_light {
        "#94A3B8"
    } else {
        c.text_subtle
    };

    let mut country_children = Vec::new();
    if let Some(flag) = opt(args, "country_flag").filter(|f| !f.is_empty()) {
        country_children.push(json!({
            "id": next_id("phone_input_flag"),
            "type": "text",
            "name": "Flag",
            "role": "phone-input-flag",
            "content": flag,
            "fontSize": 16,
            "fontWeight": 400,
        }));
    }
    country_children.push(json!({
        "id": next_id("phone_input_code"),
        "type": "text",
        "name": "Code",
        "role": "phone-input-code",
        "content": code,
        "fontSize": 14,
        "fontWeight": 500,
        "fill": [{ "type": "solid", "color": code_color }],
    }));
    country_children.push(json!({
        "id": next_id("phone_input_chevron"),
        "type": "icon_font",
        "name": "Chevron",
        "role": "phone-input-chevron",
        "iconFontName": "chevron-down",
        "iconFontFamily": "lucide",
        "width": 14,
        "height": 14,
        "fill": [{ "type": "solid", "color": chevron_color }],
    }));

    let input_row = json!({
        "id": next_id("phone_input_row"),
        "type": "frame",
        "name": "Input Row",
        "role": "phone-input-row",
        "width": "fill_container",
        "height": 44,
        "cornerRadius": 10,
        "layout": "horizontal",
        "alignItems": "center",
        "fill": [{ "type": "solid", "color": field_bg }],
        "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": stroke_color }] },
        "children": [
            {
                "id": next_id("phone_input_country"),
                "type": "frame",
                "name": "Country",
                "role": "phone-input-country",
                "width": "fit_content",
                "height": "fill_container",
                "layout": "horizontal",
                "alignItems": "center",
                "gap": 6,
                "padding": [0, 12, 0, 14],
                "children": country_children,
            },
            {
                "id": next_id("phone_input_divider"),
                "type": "rectangle",
                "name": "Divider",
                "role": "phone-input-divider",
                "width": 1,
                "height": 28,
                "fill": [{ "type": "solid", "color": divider_color }],
            },
            {
                "id": next_id("phone_input_digits"),
                "type": "frame",
                "name": "Digits",
                "role": "phone-input-digits",
                "width": "fill_container",
                "height": "fill_container",
                "layout": "horizontal",
                "alignItems": "center",
                "padding": [0, 14],
                "children": [{
                    "id": next_id("phone_input_digits_text"),
                    "type": "text",
                    "name": "Digits Text",
                    "role": "phone-input-digits-text",
                    "content": digits_content,
                    "fontSize": 14,
                    "fontWeight": 400,
                    "fill": [{ "type": "solid", "color": digits_color }],
                }],
            },
        ],
    });

    let mut children = Vec::new();
    if let Some(label) = opt(args, "label").filter(|l| !l.is_empty()) {
        let label_text = if bool_arg(args, "required") {
            format!("{label} *")
        } else {
            label.to_string()
        };
        children.push(json!({
            "id": next_id("phone_input_label"),
            "type": "text",
            "name": "Label",
            "role": "form-label",
            "content": label_text,
            "fontSize": 13,
            "fontWeight": 500,
            "fill": [{ "type": "solid", "color": label_color }],
        }));
    }
    children.push(input_row);

    Ok(json!({
        "id": next_id("phone_input_field_root"),
        "type": "frame",
        "name": "Phone Input Field",
        "role": "phone-input-field",
        "width": width,
        "height": "fit_content",
        "layout": "vertical",
        "gap": 6,
        "children": children,
    }))
}

/// Shared v1 theme-resolution mirroring `resolve-theme.ts`. `light` (or
/// `theme_aware == false`) callers should use v0 hex literals directly and
/// never read these fields.
struct PortedThemeColors {
    surface: &'static str,
    surface2: &'static str,
    bg_deep: &'static str,
    border: &'static str,
    text_primary: &'static str,
    text_body: &'static str,
    text_muted: &'static str,
    text_subtle: &'static str,
}

fn ported_theme(theme_aware: bool, theme: &str) -> PortedThemeColors {
    if theme_aware && theme == "system" {
        return PortedThemeColors {
            surface: "$color-surface",
            surface2: "$color-surface-2",
            bg_deep: "$color-bg-deep",
            border: "$color-border",
            text_primary: "$color-text-primary",
            text_body: "$color-text-body",
            text_muted: "$color-text-muted",
            text_subtle: "$color-text-subtle",
        };
    }
    if theme_aware && theme == "dark" {
        return PortedThemeColors {
            surface: "#1E293B",
            surface2: "#334155",
            bg_deep: "#0F172A",
            border: "#334155",
            text_primary: "#F1F5F9",
            text_body: "#CBD5E1",
            text_muted: "#94A3B8",
            text_subtle: "#64748B",
        };
    }
    // light fallback (v0-parity hex literals)
    PortedThemeColors {
        surface: "#FFFFFF",
        surface2: "#F1F5F9",
        bg_deep: "#F8FAFC",
        border: "#E2E8F0",
        text_primary: "#0F172A",
        text_body: "#334155",
        text_muted: "#64748B",
        text_subtle: "#94A3B8",
    }
}

// ===== add_range_slider =====

/// `add_range_slider_v0` / `_v1` — single-thumb range slider (static visual).
/// Ports `range-slider.ts` + `range-slider-v1.ts`.
fn build_range_slider(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    const TRACK_HEIGHT: f64 = 6.0;
    const THUMB_SIZE: f64 = 20.0;

    let width = number_arg(args, "width", 320.0, 160.0).floor();
    // min/max/value parsed directly so a negative `min` survives (number_arg
    // clamps to its `min` floor, which would reject negatives).
    let min = parse_f64(args, "min").unwrap_or(0.0);
    let max = parse_f64(args, "max").unwrap_or(100.0);
    let span = (max - min).max(1.0);
    let raw = parse_f64(args, "value").unwrap_or((min + max) / 2.0);
    let value = raw.max(min).min(max);
    let pct = (value - min) / span;

    let track_width = width;
    let left_width = (((track_width - THUMB_SIZE) * pct).round()).max(0.0);
    let right_width = (track_width - THUMB_SIZE - left_width).max(0.0);

    let theme = opt(args, "theme").unwrap_or("light");
    let is_light = !theme_aware || theme == "light";
    let c = ported_theme(theme_aware, theme);

    let accent_color = "#2563EB"; // brand-invariant
    let thumb_bg = if is_light { "#FFFFFF" } else { c.surface };
    let remaining_color = if is_light { "#E2E8F0" } else { c.border };
    let label_color = if is_light { "#0F172A" } else { c.text_primary };
    let value_color = if is_light { "#64748B" } else { c.text_muted };

    let mut track_children = Vec::new();
    if left_width > 0.0 {
        track_children.push(json!({
            "id": next_id("range_slider_fill"),
            "type": "rectangle",
            "name": "Fill",
            "role": "range-slider-fill",
            "width": left_width,
            "height": TRACK_HEIGHT,
            "cornerRadius": TRACK_HEIGHT / 2.0,
            "fill": [{ "type": "solid", "color": accent_color }],
        }));
    }
    track_children.push(json!({
        "id": next_id("range_slider_thumb"),
        "type": "frame",
        "name": "Thumb",
        "role": "range-slider-thumb",
        "width": THUMB_SIZE,
        "height": THUMB_SIZE,
        "cornerRadius": THUMB_SIZE / 2.0,
        "fill": [{ "type": "solid", "color": thumb_bg }],
        "stroke": { "thickness": 2, "fill": [{ "type": "solid", "color": accent_color }] },
        "effects": [{
            "type": "shadow",
            "offsetX": 0,
            "offsetY": 2,
            "blur": 4,
            "spread": 0,
            "color": "#0F172A1F",
        }],
    }));
    if right_width > 0.0 {
        track_children.push(json!({
            "id": next_id("range_slider_remaining"),
            "type": "rectangle",
            "name": "Remaining",
            "role": "range-slider-remaining",
            "width": right_width,
            "height": TRACK_HEIGHT,
            "cornerRadius": TRACK_HEIGHT / 2.0,
            "fill": [{ "type": "solid", "color": remaining_color }],
        }));
    }

    let mut children = Vec::new();
    let label = opt(args, "label").filter(|l| !l.is_empty());
    let show_value = bool_arg(args, "show_value");
    if label.is_some() || show_value {
        let mut header_children = Vec::new();
        if let Some(label) = label {
            header_children.push(json!({
                "id": next_id("range_slider_label"),
                "type": "text",
                "name": "Label",
                "role": "range-slider-label",
                "content": label,
                "fontSize": 13,
                "fontWeight": 500,
                "fill": [{ "type": "solid", "color": label_color }],
            }));
        }
        if show_value {
            let suffix = opt(args, "value_suffix").unwrap_or("");
            // TS: Math.round(value * 100) / 100 (drops trailing .0 for ints)
            let rounded = (value * 100.0).round() / 100.0;
            let rendered = format!("{}{}", format_number(rounded), suffix);
            header_children.push(json!({
                "id": next_id("range_slider_value"),
                "type": "text",
                "name": "Value",
                "role": "range-slider-value",
                "content": rendered,
                "fontSize": 13,
                "fontWeight": 500,
                "fill": [{ "type": "solid", "color": value_color }],
            }));
        }
        let justify = if label.is_some() && show_value {
            "space-between"
        } else {
            "flex-start"
        };
        children.push(json!({
            "id": next_id("range_slider_header"),
            "type": "frame",
            "name": "Header",
            "role": "range-slider-header",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "alignItems": "center",
            "justifyContent": justify,
            "children": header_children,
        }));
    }

    children.push(json!({
        "id": next_id("range_slider_track"),
        "type": "frame",
        "name": "Track",
        "role": "range-slider-track",
        "width": track_width,
        "height": THUMB_SIZE,
        "layout": "horizontal",
        "alignItems": "center",
        "children": track_children,
    }));

    Ok(json!({
        "id": next_id("range_slider_root"),
        "type": "frame",
        "name": "Range Slider",
        "role": "range-slider",
        "width": width,
        "height": "fit_content",
        "layout": "vertical",
        "gap": 8,
        "children": children,
    }))
}

/// Parse an arg as f64 (allows negatives / decimals), returning None if absent
/// or unparseable. Used where `number_arg`'s `min` clamp would be wrong.
fn parse_f64(args: &BTreeMap<String, String>, key: &str) -> Option<f64> {
    args.get(key)
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

/// Format an f64 the way JS `String(number)` does for these values: integral
/// values drop the decimal (`50` not `50.0`), fractional values keep up to 2
/// digits (already rounded by the caller).
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        // Trim trailing zeros from a 2-dp render (round-to-2 done by caller).
        let s = format!("{n:.2}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        s.to_string()
    }
}

// ===== add_search_bar =====

/// `add_search_bar_v0` / `_v1` — iOS-HIG search bar (h=44, cr=22).
/// Ports `search-bar.ts` + `search-bar-v1.ts`.
fn build_search_bar(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let icon = opt(args, "leading_icon").unwrap_or("search");
    let placeholder = opt(args, "placeholder").unwrap_or("Search...");
    let theme = opt(args, "theme").unwrap_or("light");
    let is_light = !theme_aware || theme == "light";
    let c = ported_theme(theme_aware, theme);

    let mut node = json!({
        "id": next_id("search_bar_root"),
        "type": "frame",
        "name": "Search Bar",
        "role": "search-bar",
        "width": "fill_container",
        "height": 44,
        "cornerRadius": 22,
        "layout": "horizontal",
        "alignItems": "center",
        "gap": 8,
        "padding": [0, 16],
        "children": [
            {
                "id": next_id("search_bar_icon"),
                "type": "icon_font",
                "name": "Leading Icon",
                "iconFontName": icon,
                "iconFontFamily": "lucide",
                "width": 20,
                "height": 20,
            },
            {
                "id": next_id("search_bar_placeholder"),
                "type": "text",
                "name": "Placeholder",
                "content": placeholder,
                "fontSize": 14,
                "fontWeight": 400,
            },
        ],
    });
    // v0 (and v1-light) omit the fill entirely; dark/system add surface2.
    if !is_light {
        node["fill"] = json!([{ "type": "solid", "color": c.surface2 }]);
    }
    Ok(node)
}

// ===== add_select =====

/// `add_select_v0` / `_v1` — closed-state dropdown select (label-above-input).
/// Ports `select.ts` + `select-v1.ts`.
fn build_select(args: &BTreeMap<String, String>, theme_aware: bool) -> Result<Value, ToolOutcome> {
    let label = required(args, "label")?;
    let label_text = if bool_arg(args, "required") {
        format!("{label} *")
    } else {
        label.to_string()
    };
    let placeholder = opt(args, "placeholder").unwrap_or("Select\u{2026}");
    let trailing_icon = opt(args, "trailing_icon").unwrap_or("chevron-down");
    // TS distinguishes on `value === undefined` (absence), not emptiness.
    let is_empty = args.get("value").is_none();
    let display_text = opt(args, "value").unwrap_or(placeholder);

    let theme = opt(args, "theme").unwrap_or("light");
    let is_light = !theme_aware || theme == "light";
    let c = ported_theme(theme_aware, theme);
    let placeholder_color = if is_light { "#94A3B8" } else { c.text_subtle };

    let mut text_node = json!({
        "id": next_id("select_text"),
        "type": "text",
        "name": if is_empty { "Placeholder" } else { "Selected Value" },
        "content": display_text,
        "fontSize": 14,
        "fontWeight": 400,
    });
    if is_empty {
        text_node["fill"] = json!([{ "type": "solid", "color": placeholder_color }]);
    }

    Ok(json!({
        "id": next_id("select_root"),
        "type": "frame",
        "name": "Select",
        "role": "select",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 6,
        "children": [
            {
                "id": next_id("select_label"),
                "type": "text",
                "name": "Label",
                "role": "label",
                "content": label_text,
                "fontSize": 14,
                "fontWeight": 500,
            },
            {
                "id": next_id("select_input"),
                "type": "frame",
                "name": "Input",
                "role": "select-input",
                "width": "fill_container",
                "height": 48,
                "cornerRadius": 8,
                "layout": "horizontal",
                "alignItems": "center",
                "justifyContent": "space_between",
                "gap": 8,
                "padding": [12, 16],
                "children": [
                    text_node,
                    {
                        "id": next_id("select_trailing_icon"),
                        "type": "icon_font",
                        "name": "Trailing Icon",
                        "iconFontName": trailing_icon,
                        "iconFontFamily": "lucide",
                        "width": 20,
                        "height": 20,
                    },
                ],
            },
        ],
    }))
}

// ===== add_upload_dropzone =====

/// `add_upload_dropzone_v0` / `_v1` — dashed-border file dropzone tile.
/// Ports `upload-dropzone.ts` + `upload-dropzone-v1.ts`.
fn build_upload_dropzone(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let width = number_arg(args, "width", 480.0, 200.0).floor();
    let height = number_arg(args, "height", 200.0, 120.0).floor();
    let corner_radius = number_arg(args, "corner_radius", 12.0, 0.0).floor();
    let title = opt(args, "title").unwrap_or("Drop files to upload");
    let subtitle = opt(args, "subtitle").unwrap_or("or click to browse");
    let icon = opt(args, "icon").unwrap_or("upload-cloud");

    let theme = opt(args, "theme").unwrap_or("light");
    let is_light = !theme_aware || theme == "light";
    let c = ported_theme(theme_aware, theme);

    let bg = if is_light { "#F8FAFC" } else { c.bg_deep };
    let stroke_color = if is_light { "#CBD5E1" } else { c.border };
    let icon_color = if is_light { "#64748B" } else { c.text_muted };
    let title_color = if is_light { "#334155" } else { c.text_body };
    let subtitle_color = if is_light { "#64748B" } else { c.text_muted };

    Ok(json!({
        "id": next_id("upload_dropzone_root"),
        "type": "frame",
        "name": "Upload Dropzone",
        "role": "upload-dropzone",
        "width": width,
        "height": height,
        "cornerRadius": corner_radius,
        "layout": "vertical",
        "alignItems": "center",
        "justifyContent": "center",
        "gap": 12,
        "padding": 24,
        "fill": [{ "type": "solid", "color": bg }],
        "stroke": {
            "thickness": 1.5,
            "fill": [{ "type": "solid", "color": stroke_color }],
            "strokeDashArray": [6, 4],
        },
        "children": [
            {
                "id": next_id("upload_dropzone_icon"),
                "type": "icon_font",
                "name": "Icon",
                "role": "upload-dropzone-icon",
                "iconFontName": icon,
                "iconFontFamily": "lucide",
                "width": 40,
                "height": 40,
                "fill": [{ "type": "solid", "color": icon_color }],
            },
            {
                "id": next_id("upload_dropzone_title"),
                "type": "text",
                "name": "Title",
                "role": "upload-dropzone-title",
                "content": title,
                "fontSize": 14,
                "fontWeight": 600,
                "fill": [{ "type": "solid", "color": title_color }],
            },
            {
                "id": next_id("upload_dropzone_subtitle"),
                "type": "text",
                "name": "Subtitle",
                "role": "upload-dropzone-subtitle",
                "content": subtitle,
                "fontSize": 12,
                "fontWeight": 400,
                "fill": [{ "type": "solid", "color": subtitle_color }],
            },
        ],
    }))
}
