//! Static composite-widget visuals for the OP **design** canvas.
//!
//! Widget nodes (switch / checkbox / slider / progress / select /
//! radio_group / text_input / text_area / number_input / tabs) load
//! onto the canvas as degraded `rect` / `text` / `frame` scene nodes
//! (`op-pen-loader`'s adapter), but carry their real props in
//! [`SceneWidget`](crate::layout_scene::SceneWidget). This module paints
//! the recognizable static visual (track + knob, box + check, chevron,
//! bar, …) on the non-interactive design surface, mirroring jian-core's
//! `render/scene.rs::emit_widget_visual` (which the preview/runtime
//! path uses to draw the live widget).
//!
//! Everything paints in **world** coordinates: the per-kind painter in
//! `canvas_viewport_paint.rs` hands us the already-zoom-scaled
//! `world_rect`, and we scale every internal metric (track height,
//! knob diameter, stroke width, font size) by `zoom` so the visual
//! tracks the node across viewport zoom.

use crate::layout_scene::{SceneNode, SceneWidget};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use std::borrow::Cow;

/// Accent colour for "on" / filled portions (Tailwind blue-500).
const ACCENT: Color = Color::rgb_u8(0x3b, 0x82, 0xf6);
/// Off-state track / outline grey (Tailwind gray-300).
const TRACK_OFF: Color = Color::rgb_u8(0xd1, 0xd5, 0xdb);
/// Knob / check / inner-dot white.
const KNOB: Color = Color::WHITE;
/// Resolved-value text (near-black).
const TEXT_VALUE: Color = Color::rgb_u8(0x11, 0x11, 0x11);
/// Placeholder text (muted grey).
const TEXT_MUTED: Color = Color::rgb_u8(0x66, 0x66, 0x66);

/// Base horizontal text padding inside an input (doc px, pre-zoom).
pub(crate) const INPUT_PAD_X: f32 = 8.0;
/// Leading/trailing icon glyph box inside an input (doc px, pre-zoom).
pub(crate) const INPUT_ICON_BOX: f32 = 20.0;

/// Left inset for an input's text/caret (doc px, pre-zoom). Single
/// source of truth shared by the design canvas (`paint_text_field`) and
/// the preview caret (`op_host_native::preview::paint_focus_caret`), so
/// the caret always lands where the painted text starts. Mirrors jian's
/// `scene::input_left_inset`. A leading icon reserves `PAD + ICON + PAD`.
pub fn widget_text_inset_left(w: &SceneWidget) -> f32 {
    if w.leading_icon.is_some() {
        INPUT_PAD_X + INPUT_ICON_BOX + INPUT_PAD_X
    } else {
        INPUT_PAD_X
    }
}

/// Paint the static visual for a widget scene node, in world coords.
///
/// `world_rect` is the node's already-zoom-scaled screen rect; `zoom`
/// scales internal metrics. Returns `true` when the widget kind was
/// recognized + painted (so the caller skips the bare fill/stroke);
/// `false` for an unknown kind (caller falls back to the plain rect).
pub(crate) fn paint_widget_visual(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
) -> bool {
    let Some(w) = node.widget.as_ref() else {
        return false;
    };
    if world_rect.size.x <= 0.0 || world_rect.size.y <= 0.0 {
        return false;
    }
    match w.kind.as_str() {
        "switch" => paint_switch(cx, w, world_rect),
        "checkbox" => paint_checkbox(cx, node, w, world_rect, zoom),
        "slider" => paint_slider(cx, w, world_rect, zoom),
        "progress" => paint_progress(cx, w, world_rect),
        "select" => paint_select(cx, node, w, world_rect, zoom),
        "radio_group" => paint_radio_group(cx, w, world_rect, zoom),
        "text_input" | "text_area" | "number_input" => {
            paint_text_field(cx, node, w, world_rect, zoom)
        }
        "tabs" => paint_tabs(cx, node, w, world_rect, zoom),
        _ => return false,
    }
    true
}

/// Switch: rounded pill track (accent when on, grey when off) plus a
/// white knob circle slid to the right when on, left when off.
fn paint_switch(cx: &mut PaintCx<'_>, w: &SceneWidget, r: Rect) {
    let on = w.checked.unwrap_or(false);
    let (x, y, ww, h) = rect_parts(r);
    cx.backend
        .fill_round_rect(r, h / 2.0, if on { ACCENT } else { TRACK_OFF });
    let pad = 2.0;
    let d = (h - pad * 2.0).max(2.0);
    let kx = if on { x + ww - d - pad } else { x + pad };
    cx.backend
        .fill_round_rect(Rect::xywh(kx, y + pad, d, d), d / 2.0, KNOB);
}

/// Checkbox: rounded box — accent-filled with a white check polyline
/// when on, outlined when off.
fn paint_checkbox(cx: &mut PaintCx<'_>, node: &SceneNode, w: &SceneWidget, r: Rect, zoom: f32) {
    let on = w.checked.unwrap_or(false);
    let (x, y, ww, h) = rect_parts(r);
    let radius = (node.corner_radius * zoom).max(2.0);
    let stroke_w = node.stroke.map(|s| s.width).unwrap_or(1.5) * zoom;
    if on {
        cx.backend.fill_round_rect(r, radius, ACCENT);
    } else {
        cx.backend.fill_round_rect(r, radius, KNOB);
        cx.backend.stroke_round_rect(r, radius, TRACK_OFF, stroke_w);
    }
    if on {
        // White check (✓) as a 3-point polyline, fractions matching the
        // jian-core visual: (0.24,0.52) → (0.42,0.70) → (0.76,0.30).
        let p0 = Point2D::new(x + ww * 0.24, y + h * 0.52);
        let p1 = Point2D::new(x + ww * 0.42, y + h * 0.70);
        let p2 = Point2D::new(x + ww * 0.76, y + h * 0.30);
        let cw = (2.0 * zoom).max(1.0);
        cx.backend.stroke_line(p0, p1, KNOB, cw);
        cx.backend.stroke_line(p1, p2, KNOB, cw);
    }
}

/// Slider: thin grey track + accent filled portion (value within
/// min..max) + a white knob circle with a grey outline.
fn paint_slider(cx: &mut PaintCx<'_>, w: &SceneWidget, r: Rect, zoom: f32) {
    let (x, y, ww, h) = rect_parts(r);
    let frac = range_fraction(w.value_num, w.min.unwrap_or(0.0), w.max.unwrap_or(100.0));
    let track_h = 4.0 * zoom;
    let cy = y + h / 2.0;
    cx.backend.fill_round_rect(
        Rect::xywh(x, cy - track_h / 2.0, ww, track_h),
        track_h / 2.0,
        TRACK_OFF,
    );
    if frac > 0.0 {
        cx.backend.fill_round_rect(
            Rect::xywh(x, cy - track_h / 2.0, ww * frac, track_h),
            track_h / 2.0,
            ACCENT,
        );
    }
    let d = h.clamp(10.0 * zoom, 16.0 * zoom);
    let kx = (x + ww * frac - d / 2.0).clamp(x, x + ww - d);
    let knob = Rect::xywh(kx, cy - d / 2.0, d, d);
    cx.backend.fill_round_rect(knob, d / 2.0, KNOB);
    cx.backend
        .stroke_round_rect(knob, d / 2.0, TRACK_OFF, 1.0 * zoom);
}

/// Progress: rounded grey track + accent filled portion (value / max).
fn paint_progress(cx: &mut PaintCx<'_>, w: &SceneWidget, r: Rect) {
    let (x, y, ww, h) = rect_parts(r);
    let max = w.max.unwrap_or(100.0);
    let frac = if max > 0.0 {
        (w.value_num.unwrap_or(0.0) / max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let radius = h / 2.0;
    cx.backend.fill_round_rect(r, radius, TRACK_OFF);
    if frac > 0.0 {
        cx.backend
            .fill_round_rect(Rect::xywh(x, y, ww * frac, h), radius, ACCENT);
    }
}

/// Select: outlined box + current value / placeholder text + a down
/// chevron on the trailing edge.
fn paint_select(cx: &mut PaintCx<'_>, node: &SceneNode, w: &SceneWidget, r: Rect, zoom: f32) {
    let (x, y, ww, h) = rect_parts(r);
    let radius = (node.corner_radius * zoom).max(6.0 * zoom);
    if let Some(fill) = node.fill {
        cx.backend.fill_round_rect(r, radius, fill);
    } else {
        cx.backend.fill_round_rect(r, radius, KNOB);
    }
    let stroke_w = node.stroke.map(|s| s.width).unwrap_or(1.0) * zoom;
    cx.backend.stroke_round_rect(r, radius, TRACK_OFF, stroke_w);

    // Current selection (by value) wins; else the placeholder, muted.
    let selected = w
        .value_str
        .as_deref()
        .and_then(|v| option_label(w, v))
        .filter(|s| !s.is_empty());
    let label = match selected {
        Some(text) => Some((text, TEXT_VALUE)),
        None => w
            .placeholder
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|t| (t, TEXT_MUTED)),
    };
    if let Some((text, color)) = label {
        let fs = 14.0 * zoom;
        draw_label(cx, text, color, x + 8.0 * zoom, y + (h - fs) / 2.0, fs);
    }
    paint_chevron(cx, x + ww - 20.0 * zoom, y + h / 2.0, zoom);
}

/// Radio group: per option a circle (accent-filled with a white inner
/// dot when selected, outlined when not) plus its label to the right.
fn paint_radio_group(cx: &mut PaintCx<'_>, w: &SceneWidget, r: Rect, zoom: f32) {
    if w.options.is_empty() {
        return;
    }
    let (x, y, ww, h) = rect_parts(r);
    let selected = w.value_str.as_deref();
    let n = w.options.len().max(1);
    let row_h = (h / n as f32).clamp(0.0, 28.0 * zoom);
    let d = 14.0 * zoom;
    let fs = 14.0 * zoom;
    for (i, opt) in w.options.iter().enumerate() {
        let on = selected == Some(opt.value.as_str());
        let ry = y + i as f32 * row_h + (row_h - d) / 2.0;
        let circle = Rect::xywh(x + 2.0 * zoom, ry, d, d);
        cx.backend
            .fill_round_rect(circle, d / 2.0, if on { ACCENT } else { KNOB });
        cx.backend
            .stroke_round_rect(circle, d / 2.0, TRACK_OFF, 1.5 * zoom);
        if on {
            let inner = d * 0.4;
            cx.backend.fill_round_rect(
                Rect::xywh(
                    x + 2.0 * zoom + (d - inner) / 2.0,
                    ry + (d - inner) / 2.0,
                    inner,
                    inner,
                ),
                inner / 2.0,
                KNOB,
            );
        }
        let label = if opt.label.is_empty() {
            opt.value.as_str()
        } else {
            opt.label.as_str()
        };
        let lx = x + 2.0 * zoom + d + 8.0 * zoom;
        let _ = ww;
        draw_label(cx, label, TEXT_VALUE, lx, ry + (d - fs) / 2.0, fs);
    }
}

/// Text input / textarea / number input: outlined box + the value
/// (near-black) or, when empty, the placeholder (muted).
fn paint_text_field(cx: &mut PaintCx<'_>, node: &SceneNode, w: &SceneWidget, r: Rect, zoom: f32) {
    let (x, y, ww, h) = rect_parts(r);
    let radius = (node.corner_radius * zoom).max(6.0 * zoom);
    if let Some(fill) = node.fill {
        cx.backend.fill_round_rect(r, radius, fill);
    } else {
        cx.backend.fill_round_rect(r, radius, KNOB);
    }
    let stroke_w = node.stroke.map(|s| s.width).unwrap_or(1.0) * zoom;
    cx.backend.stroke_round_rect(r, radius, TRACK_OFF, stroke_w);

    // Leading / trailing lucide glyphs at the content edges, vertically
    // centred. The text inset (`widget_text_inset_left`) reserves room
    // for the leading icon so the value/placeholder never overlaps it.
    let icon = INPUT_ICON_BOX * zoom;
    let iy = y + (h - icon) / 2.0;
    if let Some(name) = w.leading_icon.as_deref() {
        crate::widgets::icons::paint_icon_font_node(
            cx.backend,
            "",
            name,
            Rect::xywh(x + INPUT_PAD_X * zoom, iy, icon, icon),
            Some(TEXT_MUTED),
        );
    }
    if let Some(name) = w.trailing_icon.as_deref() {
        crate::widgets::icons::paint_icon_font_node(
            cx.backend,
            "",
            name,
            Rect::xywh(
                x + ww - (INPUT_PAD_X + INPUT_ICON_BOX) * zoom,
                iy,
                icon,
                icon,
            ),
            Some(TEXT_MUTED),
        );
    }

    if let Some((text, color)) = text_field_display_text(w) {
        let fs = 14.0 * zoom;
        // text_area top-aligns; single-line inputs vertically centre.
        let ty = if w.kind == "text_area" {
            y + 8.0 * zoom
        } else {
            y + (h - fs) / 2.0
        };
        draw_label(
            cx,
            text.as_ref(),
            color,
            x + widget_text_inset_left(w) * zoom,
            ty,
            fs,
        );
    }
}

/// Tabs: a minimal tab-bar row of option labels with the active tab
/// underlined in accent. The panel area (children) is painted by the
/// caller's normal child recursion; we only add the bar.
fn paint_tabs(cx: &mut PaintCx<'_>, node: &SceneNode, w: &SceneWidget, r: Rect, zoom: f32) {
    let (x, y, ww, _h) = rect_parts(r);
    if w.options.is_empty() {
        return;
    }
    let bar_h = 32.0 * zoom;
    // Bottom border under the whole tab bar.
    let by = y + bar_h;
    cx.backend.stroke_line(
        Point2D::new(x, by),
        Point2D::new(x + ww, by),
        TRACK_OFF,
        1.0 * zoom,
    );
    let active = w.value_str.as_deref();
    let n = w.options.len().max(1);
    let tab_w = ww / n as f32;
    let fs = 14.0 * zoom;
    for (i, opt) in w.options.iter().enumerate() {
        let tx = x + i as f32 * tab_w;
        let on = active == Some(opt.value.as_str()) || (active.is_none() && i == 0);
        let label = if opt.label.is_empty() {
            opt.value.as_str()
        } else {
            opt.label.as_str()
        };
        let color = if on { TEXT_VALUE } else { TEXT_MUTED };
        draw_label(
            cx,
            label,
            color,
            tx + 8.0 * zoom,
            y + (bar_h - fs) / 2.0,
            fs,
        );
        if on {
            // Accent underline beneath the active tab.
            let uy = by - 1.0 * zoom;
            cx.backend.stroke_line(
                Point2D::new(tx, uy),
                Point2D::new(tx + tab_w, uy),
                ACCENT,
                2.0 * zoom,
            );
        }
    }
    let _ = node;
}

/// Draw a down chevron (`v`) centred at `(cx_px, cy_px)` on the leading
/// point — a 3-point polyline matching the jian-core select chevron.
fn paint_chevron(cx: &mut PaintCx<'_>, cx_px: f32, cy_px: f32, zoom: f32) {
    let cw = 9.0 * zoom;
    let p0 = Point2D::new(cx_px, cy_px - cw * 0.22);
    let p1 = Point2D::new(cx_px + cw / 2.0, cy_px + cw * 0.33);
    let p2 = Point2D::new(cx_px + cw, cy_px - cw * 0.22);
    let width = 1.5 * zoom;
    cx.backend.stroke_line(p0, p1, TEXT_MUTED, width);
    cx.backend.stroke_line(p1, p2, TEXT_MUTED, width);
}

/// Draw a single-run, left-aligned label at `(x, top_y)` in world
/// coords. `font_size` is already zoom-scaled; the run's origin is the
/// text top edge (TS canvas paint parity — see `paint_text_node`).
fn draw_label(cx: &mut PaintCx<'_>, text: &str, color: Color, x: f32, top_y: f32, font_size: f32) {
    let layout =
        TextLayout::single_run(text, "", font_size, color.to_jian(), Point2D::new(x, top_y));
    cx.backend.draw_text(&layout, Point2D::new(x, top_y));
}

/// `(x, y, w, h)` of a rect.
fn rect_parts(r: Rect) -> (f32, f32, f32, f32) {
    (r.origin.x, r.origin.y, r.size.x, r.size.y)
}

/// Fraction of `value` within `[min, max]`, clamped to `0.0..=1.0`.
/// An absent value or a degenerate range collapses to 0.0.
fn range_fraction(value: Option<f32>, min: f32, max: f32) -> f32 {
    let v = value.unwrap_or(min);
    if max > min {
        ((v - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Look up a select / radio option's display label by its `value`.
pub(crate) fn option_label<'a>(w: &'a SceneWidget, value: &str) -> Option<&'a str> {
    w.options.iter().find(|o| o.value == value).map(|o| {
        if o.label.is_empty() {
            o.value.as_str()
        } else {
            o.label.as_str()
        }
    })
}

pub(crate) fn text_field_display_text(w: &SceneWidget) -> Option<(Cow<'_, str>, Color)> {
    let value = match w.value_str.as_deref() {
        Some(text) => (!text.is_empty()).then_some(Cow::Borrowed(text)),
        None if w.kind == "number_input" => w
            .value_num
            .map(format_number)
            .filter(|text| !text.is_empty())
            .map(Cow::Owned),
        None => None,
    };
    match value {
        Some(text) => Some((text, TEXT_VALUE)),
        None => w
            .placeholder
            .as_deref()
            .filter(|text| !text.is_empty())
            .map(|text| (Cow::Borrowed(text), TEXT_MUTED)),
    }
}

/// Format a slider / number value without a trailing `.0` for integers.
fn format_number(v: f32) -> String {
    if v.fract().abs() < f32::EPSILON {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
#[path = "canvas_viewport_widget_tests.rs"]
mod tests;
