//! Shared paint primitives for the property-panel sections —
//! section headers, dividers, the various input pills, and the
//! standalone `format_color_hex` helper.
//!
//! Pulled out of `property_panel_sections.rs` to keep that file
//! under the 800-line workspace ceiling. Every helper is callable
//! from any section paint module via `super::property_panel_inputs::*`.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

pub const PAD_X: f32 = 16.0;
pub const SECTION_GAP: f32 = 8.0;
pub const ROW_HEIGHT: f32 = 28.0;
pub const INPUT_HEIGHT: f32 = 30.0;
pub const INPUT_RADIUS: f32 = 8.0;
pub const COLOR_VARIABLE_BUTTON_W: f32 = 28.0;
pub const COLOR_VARIABLE_GAP: f32 = 6.0;
pub const SECTION_HEADER_HEIGHT: f32 = 24.0;
pub const TAB_HEIGHT: f32 = 36.0;
pub const HEADER_HEIGHT: f32 = 30.0;

/// "Create component" button metrics — shared by `paint_create_component`
/// and every layout walker so paint ↔ hit-test ↔ section offsets stay in
/// lockstep. The button + its icon are kept compact (icon size below).
pub const CREATE_COMPONENT_PAD_TOP: f32 = 8.0;
pub const CREATE_COMPONENT_BTN_H: f32 = 30.0;
pub const CREATE_COMPONENT_PAD_BOTTOM: f32 = 12.0;
pub const CREATE_COMPONENT_ICON: f32 = 14.0;
/// Total vertical space the create-component block consumes.
pub const CREATE_COMPONENT_BLOCK_H: f32 =
    CREATE_COMPONENT_PAD_TOP + CREATE_COMPONENT_BTN_H + CREATE_COMPONENT_PAD_BOTTOM;
/// Gap between the instance variant's two rows (Go to component /
/// Detach instance).
pub const CREATE_COMPONENT_ROW_GAP: f32 = 6.0;

/// Component / instance accent colours — TS `text-purple-400`
/// (#a855f7) for components, `#9281f7` for instances.
pub const COMPONENT_ACCENT: Color = Color {
    r: 0xA8 as f32 / 255.0,
    g: 0x55 as f32 / 255.0,
    b: 0xF7 as f32 / 255.0,
    a: 1.0,
};
pub const INSTANCE_ACCENT: Color = Color {
    r: 0x92 as f32 / 255.0,
    g: 0x81 as f32 / 255.0,
    b: 0xF7 as f32 / 255.0,
    a: 1.0,
};

/// Vertical space the create-component block consumes for the given
/// button variant — the instance variant stacks a second row. Paint
/// + BOTH layout walkers must read this so y-offsets never drift.
pub fn create_component_block_height(
    state: crate::widgets::property_panel_visibility::ComponentButtonState,
) -> f32 {
    use crate::widgets::property_panel_visibility::ComponentButtonState as S;
    match state {
        S::Create | S::DetachComponent => CREATE_COMPONENT_BLOCK_H,
        S::Instance => CREATE_COMPONENT_BLOCK_H + CREATE_COMPONENT_BTN_H + CREATE_COMPONENT_ROW_GAP,
    }
}

pub fn paint_section_label(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    x: f32,
    y: f32,
    _w: f32,
) -> f32 {
    let label_layout = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label_layout, Point2D::new(x + PAD_X, y + 16.0));
    y + SECTION_HEADER_HEIGHT
}

pub fn paint_section_label_with_add(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let next_y = paint_section_label(cx, theme, label, x, y, width);
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(x + width - PAD_X - 14.0, y + 6.0),
        14.0,
        theme.muted_foreground,
        1.4,
    );
    next_y
}

pub fn paint_section_divider(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32, width: f32) {
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(width, 1.0),
        },
        theme.border,
    );
}

pub fn paint_input_with_prefix(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    prefix: &str,
    value: &str,
) {
    paint_input_with_prefix_focused(cx, theme, rect, prefix, value, false, None, false);
}

/// `paint_input_with_prefix` with explicit focus + caret toggle.
#[allow(clippy::too_many_arguments)]
pub fn paint_input_with_prefix_focused(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    prefix: &str,
    value: &str,
    focused: bool,
    caret: Option<usize>,
    select_all: bool,
) {
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    if focused {
        cx.backend
            .stroke_round_rect(rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    let prefix_layout = TextLayout::single_run(
        prefix,
        "system-ui",
        12.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &prefix_layout,
        Point2D::new(
            rect.origin.x + 10.0,
            rect.origin.y + rect.size.y / 2.0 + 4.0,
        ),
    );
    let prefix_w = cx.backend.measure_text(prefix, 12.0);
    let value_x = rect.origin.x + 10.0 + prefix_w + 8.0;
    let baseline_y = rect.origin.y + rect.size.y / 2.0 + 4.0;
    if focused && select_all && !value.is_empty() {
        crate::widgets::text_selection::paint_single_line_selection(
            cx,
            theme,
            value,
            value_x,
            baseline_y,
            12.0,
            rect.origin.x + rect.size.x - 8.0,
        );
    }
    let value_layout = TextLayout::single_run(
        value,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&value_layout, Point2D::new(value_x, baseline_y));
    if let Some(pos) = caret {
        let value_w = cx
            .backend
            .measure_text(&value[..pos.min(value.len())], 12.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(value_x + value_w, rect.origin.y + 6.0),
                size: Point2D::new(1.5, rect.size.y - 12.0),
            },
            theme.foreground,
        );
    }
}

#[allow(dead_code)]
pub fn paint_input_with_suffix(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    value: &str,
    unit: &str,
) {
    paint_input_with_suffix_focused(cx, theme, rect, value, unit, false, None, false);
}

#[allow(clippy::too_many_arguments)]
pub fn paint_input_with_suffix_focused(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    value: &str,
    unit: &str,
    focused: bool,
    caret: Option<usize>,
    select_all: bool,
) {
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    if focused {
        cx.backend
            .stroke_round_rect(rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    let value_x = rect.origin.x + 10.0;
    // Centre the text vertically in the box: for the standard 30px input
    // this is +19 (unchanged); for the compact 20px gap row it becomes
    // +14 so the value isn't pinned to the bottom.
    let baseline_y = rect.origin.y + rect.size.y / 2.0 + 4.0;
    if focused && select_all && !value.is_empty() {
        crate::widgets::text_selection::paint_single_line_selection(
            cx,
            theme,
            value,
            value_x,
            baseline_y,
            12.0,
            rect.origin.x + rect.size.x - 18.0,
        );
    }
    let value_layout = TextLayout::single_run(
        value,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&value_layout, Point2D::new(value_x, baseline_y));
    if let Some(pos) = caret {
        let value_w = cx
            .backend
            .measure_text(&value[..pos.min(value.len())], 12.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(value_x + value_w, rect.origin.y + 6.0),
                size: Point2D::new(1.5, rect.size.y - 12.0),
            },
            theme.foreground,
        );
    }
    let unit_layout = TextLayout::single_run(
        unit,
        "system-ui",
        12.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &unit_layout,
        Point2D::new(rect.origin.x + rect.size.x - 14.0, baseline_y),
    );
}

#[allow(dead_code)]
pub fn paint_input_with_icon(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    icon: Icon,
    value: &str,
    unit: Option<&str>,
) {
    paint_input_with_icon_focused(cx, theme, rect, icon, value, unit, false, None, false);
}

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_input_with_icon_focused(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    icon: Icon,
    value: &str,
    unit: Option<&str>,
    focused: bool,
    caret: Option<usize>,
    select_all: bool,
) {
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    if focused {
        cx.backend
            .stroke_round_rect(rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    // Icon at 10 px from the left edge — matches the prefix-text
    // inputs (X / Y / W / H), so a row of mixed icon + prefix
    // inputs reads as a single column. Vertically centred in the
    // 30-tall row: `(30 - 14) / 2 == 8`.
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + 10.0, rect.origin.y + 8.0),
        14.0,
        theme.muted_foreground,
        1.4,
    );
    let value_x = rect.origin.x + 30.0;
    let baseline_y = rect.origin.y + 19.0;
    if focused && select_all && !value.is_empty() {
        crate::widgets::text_selection::paint_single_line_selection(
            cx,
            theme,
            value,
            value_x,
            baseline_y,
            12.0,
            rect.origin.x + rect.size.x - 18.0,
        );
    }
    let value_layout = TextLayout::single_run(
        value,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&value_layout, Point2D::new(value_x, baseline_y));
    let value_w = cx.backend.measure_text(value, 12.0);
    if let Some(pos) = caret {
        let caret_w = cx
            .backend
            .measure_text(&value[..pos.min(value.len())], 12.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(value_x + caret_w, rect.origin.y + 6.0),
                size: Point2D::new(1.5, rect.size.y - 12.0),
            },
            theme.foreground,
        );
    }
    let _ = value_w;
    if let Some(u) = unit {
        // Unit pinned to the box's right edge — keeps the icon
        // input visually consistent with the suffix-only inputs
        // ("100 %" etc.) where the unit also anchors right.
        let unit_layout = TextLayout::single_run(
            u,
            "system-ui",
            12.0,
            to_jian_color(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &unit_layout,
            Point2D::new(rect.origin.x + rect.size.x - 14.0, baseline_y),
        );
    }
}

#[allow(dead_code)]
pub fn paint_dropdown(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, value: &str) {
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    let value_layout = TextLayout::single_run(
        value,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &value_layout,
        Point2D::new(rect.origin.x + 12.0, rect.origin.y + 19.0),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(rect.origin.x + rect.size.x - 22.0, rect.origin.y + 5.0),
        16.0,
        theme.muted_foreground,
        1.4,
    );
}

pub fn format_color_hex(c: Color) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

pub fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
