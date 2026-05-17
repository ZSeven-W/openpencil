//! Section paint helpers for [`crate::widgets::PropertyPanel`].
//! Split out of `property_panel.rs` to honor the 800-line file
//! ceiling. Each `paint_*_section` returns the y-coordinate just
//! below itself so the parent can chain them.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::NodeSnapshot;
use crate::widgets::property_panel_inputs::{
    format_color_hex, paint_dropdown, paint_input_with_icon_focused,
    paint_input_with_prefix_focused, paint_input_with_suffix_focused, paint_section_divider,
    paint_section_label, to_jian_color, HEADER_HEIGHT, INPUT_HEIGHT, INPUT_RADIUS, PAD_X,
    SECTION_GAP, TAB_HEIGHT,
};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::PropertyFocus;

// Re-exports — fill paint moved to `property_panel_fill.rs`,
// layout walkers + visibility flags to `property_panel_layout.rs`.
pub use crate::widgets::property_panel_effects::paint_effects_section;
pub use crate::widgets::property_panel_fill::{
    fill_type_label, paint_fill_section, paint_fill_type_picker,
};
pub use crate::widgets::property_panel_inputs::format_color_hex as _format_color_hex_compat;

/// Hit-result for a click on the property panel — payload is the
/// input the click landed on (host stores in
/// `Document.ui.property_focus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyPanelHit {
    Input(PropertyFocus),
}

/// State plumbed from `PropertyPanel` down into the per-section
/// paint helpers so the focused input can render its primary
/// border + caret + draft text. Unused for non-editable rows.
pub struct EditContext<'a> {
    pub focus: Option<PropertyFocus>,
    pub draft: &'a str,
    pub caret_anchor_ms: u64,
    pub now_ms: u64,
}

/// Localised chrome strings for the PropertyPanel sections.
/// Resolved once at panel-construction time from `Document::t` so
/// every section's `paint_section_label` call gets the
/// locale-appropriate text without each helper hitting the
/// translation layer.
#[derive(Debug, Clone, Copy)]
pub struct PropertyLabels {
    pub tab_design: &'static str,
    pub tab_code: &'static str,
    pub create_component: &'static str,
    pub position: &'static str,
    pub flex_layout: &'static str,
    pub size: &'static str,
    pub layer: &'static str,
    pub opacity: &'static str,
    pub fill: &'static str,
    pub stroke: &'static str,
    pub effects: &'static str,
    pub export: &'static str,
    pub fill_width: &'static str,
    pub fill_height: &'static str,
    pub hug_width: &'static str,
    pub hug_height: &'static str,
    pub clip_content: &'static str,
}

impl PropertyLabels {
    /// Resolve every PropertyPanel chrome string against the editor's
    /// active locale (`EditorUiState.locale`).
    pub fn for_editor_ui(ui: &op_editor_core::editor_ui_state::EditorUiState) -> Self {
        // `pick` returns either the localised value or the English
        // fallback when the key isn't in the table. Both branches
        // are `&'static str` so the whole struct is `Copy` and
        // zero-allocation per build.
        let pick = |key: &'static str, fallback: &'static str| -> &'static str {
            let translated = crate::widgets::editor_state_ext::translate(ui, key);
            if translated == key {
                fallback
            } else {
                translated
            }
        };
        Self {
            tab_design: pick("rightPanel.design", "Design"),
            tab_code: pick("rightPanel.code", "Code"),
            create_component: pick("property.createComponent", "Create Component"),
            position: pick("size.position", "Position"),
            flex_layout: pick("layout.flexLayout", "Flex Layout"),
            size: pick("layout.dimensions", "Size"),
            layer: pick("appearance.layer", "Layer"),
            opacity: pick("appearance.opacity", "Opacity"),
            fill: pick("fill.title", "Fill"),
            stroke: pick("stroke.title", "Stroke"),
            effects: pick("effects.title", "Effects"),
            export: pick("export.title", "Export"),
            fill_width: pick("layout.fillWidth", "Fill Width"),
            fill_height: pick("layout.fillHeight", "Fill Height"),
            hug_width: pick("layout.hugWidth", "Hug Width"),
            hug_height: pick("layout.hugHeight", "Hug Height"),
            clip_content: pick("layout.clipContent", "Clip Content"),
        }
    }
}

impl<'a> EditContext<'a> {
    /// Convenience for paint helpers — returns the value to render
    /// for the given field: the live edit draft when this field is
    /// focused, or the snapshot fallback otherwise.
    pub fn value_for<'b>(&'b self, focus: PropertyFocus, fallback: &'b str) -> &'b str {
        if self.focus == Some(focus) {
            self.draft
        } else {
            fallback
        }
    }

    /// Whether this field currently shows the caret.
    pub fn caret_visible(&self, focus: PropertyFocus) -> bool {
        self.focus == Some(focus)
            && jian_core::anim::blink_visible(self.now_ms, self.caret_anchor_ms, 500)
    }
}

// Layout constants + shared paint helpers live in
// `property_panel_inputs.rs` and are imported via the pub-use
// block above.

/// Re-exports for back-compat — the layout walkers + the
/// `VisibleSections` / `SizeFlags` state live in
/// `property_panel_layout.rs` now.
pub use crate::widgets::property_panel_layout::{
    action_button_rects, action_button_rects_with_fill_picker, editable_input_rects,
    fill_body_height, SizeFlags, VisibleSections,
};

// ── Tab strip ─────────────────────────────────────────────────────

pub fn paint_tab_strip(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    active: op_editor_core::PropertyTab,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    use op_editor_core::PropertyTab;
    let pad = 14.0;
    let tab_y = y + 6.0;
    let design_w = (cx.backend.measure_text(labels.tab_design, 13.0) + 24.0).max(48.0);
    let design_rect = Rect {
        origin: Point2D::new(x + pad, tab_y),
        size: Point2D::new(design_w, 26.0),
    };
    let code_w = (cx.backend.measure_text(labels.tab_code, 13.0) + 24.0).max(48.0);
    let code_rect = Rect {
        origin: Point2D::new(design_rect.origin.x + design_rect.size.x + 6.0, tab_y),
        size: Point2D::new(code_w, 26.0),
    };
    if matches!(active, PropertyTab::Design) {
        cx.backend.fill_round_rect(design_rect, 6.0, theme.muted);
    } else {
        cx.backend.fill_round_rect(code_rect, 6.0, theme.muted);
    }
    let design_color = if matches!(active, PropertyTab::Design) {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    let code_color = if matches!(active, PropertyTab::Code) {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    let design_label = TextLayout::single_run(
        labels.tab_design,
        "system-ui",
        13.0,
        to_jian_color(design_color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &design_label,
        Point2D::new(design_rect.origin.x + 12.0, design_rect.origin.y + 18.0),
    );
    let code_label = TextLayout::single_run(
        labels.tab_code,
        "system-ui",
        13.0,
        to_jian_color(code_color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &code_label,
        Point2D::new(code_rect.origin.x + 12.0, code_rect.origin.y + 18.0),
    );
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x, y + TAB_HEIGHT - 1.0),
            size: Point2D::new(width, 1.0),
        },
        theme.border,
    );
    y + TAB_HEIGHT
}

// ── Header row ────────────────────────────────────────────────────

pub fn paint_node_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    x: f32,
    y: f32,
    _w: f32,
) -> f32 {
    let label = TextLayout::single_run(
        &snapshot.kind,
        "system-ui",
        14.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label, Point2D::new(x + PAD_X, y + 22.0));
    y + HEADER_HEIGHT
}

// ── Create-component button ───────────────────────────────────────

pub fn paint_create_component(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let pad_top = 8.0;
    let btn_h = 36.0;
    let btn_rect = Rect {
        origin: Point2D::new(x + PAD_X, y + pad_top),
        size: Point2D::new(width - PAD_X * 2.0, btn_h),
    };
    cx.backend.fill_round_rect(btn_rect, 8.0, theme.muted);
    cx.backend
        .stroke_round_rect(btn_rect, 8.0, theme.border, 1.0);
    // TS uses Component icon (cluster of 4 small diamonds) for
    // the "create component" affordance. Diamond is imported in
    // the same file but used elsewhere (instance indicator).
    draw_icon(
        cx.backend,
        Icon::Component,
        Point2D::new(btn_rect.origin.x + 12.0, btn_rect.origin.y + 9.0),
        18.0,
        theme.foreground,
        1.4,
    );
    let label = TextLayout::single_run(
        labels.create_component,
        "system-ui",
        13.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    let label_w = cx.backend.measure_text(labels.create_component, 13.0);
    cx.backend.draw_text(
        &label,
        Point2D::new(
            btn_rect.origin.x + (btn_rect.size.x - label_w) / 2.0 + 12.0,
            btn_rect.origin.y + 23.0,
        ),
    );
    y + pad_top + btn_h + 12.0
}

// ── Position section ──────────────────────────────────────────────

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_position_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.position, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let x_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let y_rect = Rect {
        origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let x_value = snapshot.x.to_string();
    paint_input_with_prefix_focused(
        cx,
        theme,
        x_rect,
        "X",
        edit.value_for(PropertyFocus::PositionX, &x_value),
        edit.focus == Some(PropertyFocus::PositionX),
        edit.caret_visible(PropertyFocus::PositionX),
    );
    let y_value = snapshot.y.to_string();
    paint_input_with_prefix_focused(
        cx,
        theme,
        y_rect,
        "Y",
        edit.value_for(PropertyFocus::PositionY, &y_value),
        edit.focus == Some(PropertyFocus::PositionY),
        edit.caret_visible(PropertyFocus::PositionY),
    );
    y += INPUT_HEIGHT + 6.0;
    // Rotation input — TS uses RotateCw glyph; we render with an
    // icon-prefixed pill that accepts editable degree values.
    let rotation_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let rotation_value = format!("{}", snapshot.rotation_deg.round() as i32);
    paint_input_with_icon_focused(
        cx,
        theme,
        rotation_rect,
        Icon::RotateCw,
        edit.value_for(PropertyFocus::Rotation, &rotation_value),
        Some("°"),
        edit.focus == Some(PropertyFocus::Rotation),
        edit.caret_visible(PropertyFocus::Rotation),
    );
    // Corner radius (R) — editable input bound to Node::corner_radius
    // via PropertyFocus::PositionR.
    let r_rect = Rect {
        origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let r_value = format!("{}", snapshot.corner_radius.round() as i32);
    paint_input_with_prefix_focused(
        cx,
        theme,
        r_rect,
        "R",
        edit.value_for(PropertyFocus::PositionR, &r_value),
        edit.focus == Some(PropertyFocus::PositionR),
        edit.caret_visible(PropertyFocus::PositionR),
    );
    y += INPUT_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Flex layout section ──────────────────────────────────────────

pub fn paint_flex_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    active: op_editor_core::FlexLayout,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.flex_layout, x, y, width);
    // TS layout-section.tsx uses Columns3 / Rows3 / LayoutGrid for
    // the three flex modes; LayoutGrid is the default-active mode
    // (Free layout).
    let btn_w = 56.0;
    let gap = 8.0;
    let row_x = x + PAD_X;
    use op_editor_core::FlexLayout;
    let modes = [
        (FlexLayout::Free, Icon::LayoutGrid),
        (FlexLayout::Vertical, Icon::Rows3),
        (FlexLayout::Horizontal, Icon::Columns3),
    ];
    for (i, (mode, icon)) in modes.iter().enumerate() {
        let bx = row_x + i as f32 * (btn_w + gap);
        let rect = Rect {
            origin: Point2D::new(bx, y),
            size: Point2D::new(btn_w, 32.0),
        };
        let is_active = *mode == active;
        if is_active {
            cx.backend.fill_round_rect(rect, 6.0, theme.primary);
        } else {
            cx.backend.fill_round_rect(rect, 6.0, theme.muted);
            cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
        }
        let icon_color = if is_active {
            theme.primary_foreground
        } else {
            theme.muted_foreground
        };
        draw_icon(
            cx.backend,
            *icon,
            Point2D::new(rect.origin.x + (btn_w - 18.0) / 2.0, rect.origin.y + 7.0),
            18.0,
            icon_color,
            1.4,
        );
        let _ = mode; // intentionally unused beyond the active-check
    }
    y += 32.0 + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Size section ──────────────────────────────────────────────────

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_size_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    flags: SizeFlags,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.size, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let w_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let h_rect = Rect {
        origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let w_value = snapshot.width.to_string();
    paint_input_with_prefix_focused(
        cx,
        theme,
        w_rect,
        "W",
        edit.value_for(PropertyFocus::SizeW, &w_value),
        edit.focus == Some(PropertyFocus::SizeW),
        edit.caret_visible(PropertyFocus::SizeW),
    );
    let h_value = snapshot.height.to_string();
    paint_input_with_prefix_focused(
        cx,
        theme,
        h_rect,
        "H",
        edit.value_for(PropertyFocus::SizeH, &h_value),
        edit.focus == Some(PropertyFocus::SizeH),
        edit.caret_visible(PropertyFocus::SizeH),
    );
    y += INPUT_HEIGHT + 10.0;
    let row_h = 22.0;
    paint_check_row(
        cx,
        theme,
        x + PAD_X,
        y,
        half_w,
        labels.fill_width,
        flags.fill_width,
    );
    paint_check_row(
        cx,
        theme,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
        labels.fill_height,
        flags.fill_height,
    );
    y += row_h;
    paint_check_row(
        cx,
        theme,
        x + PAD_X,
        y,
        half_w,
        labels.hug_width,
        flags.hug_width,
    );
    paint_check_row(
        cx,
        theme,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
        labels.hug_height,
        flags.hug_height,
    );
    y += row_h;
    paint_check_row(
        cx,
        theme,
        x + PAD_X,
        y,
        usable_w,
        labels.clip_content,
        flags.clip_content,
    );
    y += row_h + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

fn paint_check_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    _w: f32,
    label: &str,
    checked: bool,
) {
    let box_rect = Rect {
        origin: Point2D::new(x, y + 3.0),
        size: Point2D::new(16.0, 16.0),
    };
    if checked {
        cx.backend.fill_round_rect(box_rect, 4.0, theme.primary);
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(box_rect.origin.x + 1.0, box_rect.origin.y + 1.0),
            14.0,
            theme.primary_foreground,
            1.8,
        );
    } else {
        cx.backend
            .stroke_round_rect(box_rect, 4.0, theme.border, 1.0);
    }
    let lbl = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&lbl, Point2D::new(x + 22.0, y + 16.0));
}

// ── Layer (opacity) section ───────────────────────────────────────

pub fn paint_layer_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    edit: &EditContext<'_>,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.layer, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let row = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w / 2.0 - 4.0, INPUT_HEIGHT),
    };
    let focused = edit.focus == Some(PropertyFocus::Opacity);
    let value = edit.value_for(PropertyFocus::Opacity, "100");
    paint_input_with_suffix_focused(
        cx,
        theme,
        row,
        value,
        "%",
        focused,
        edit.caret_visible(PropertyFocus::Opacity),
    );
    y += INPUT_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Stroke section ────────────────────────────────────────────────

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_stroke_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.stroke, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let stroke_color = snapshot.stroke.map(|s| s.color).unwrap_or(Color {
        r: 0x37 as f32 / 255.0,
        g: 0x41 as f32 / 255.0,
        b: 0x51 as f32 / 255.0,
        a: 1.0,
    });
    let stroke_width = snapshot.stroke.map(|s| s.width).unwrap_or(0.0);
    let width_w = 60.0;
    let hex_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w - width_w - 8.0, INPUT_HEIGHT),
    };
    let hex_focused = edit.focus == Some(PropertyFocus::StrokeHex);
    cx.backend
        .fill_round_rect(hex_rect, INPUT_RADIUS, theme.muted);
    if hex_focused {
        cx.backend
            .stroke_round_rect(hex_rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    cx.backend.fill_round_rect(
        Rect {
            origin: Point2D::new(hex_rect.origin.x + 6.0, hex_rect.origin.y + 5.0),
            size: Point2D::new(16.0, 16.0),
        },
        3.0,
        stroke_color,
    );
    let hex_owned = format_color_hex(stroke_color);
    let hex_text = edit.value_for(PropertyFocus::StrokeHex, &hex_owned);
    let hex_layout = TextLayout::single_run(
        hex_text,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    let hex_x = hex_rect.origin.x + 30.0;
    cx.backend
        .draw_text(&hex_layout, Point2D::new(hex_x, hex_rect.origin.y + 17.0));
    if edit.caret_visible(PropertyFocus::StrokeHex) {
        let w = cx.backend.measure_text(hex_text, 12.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(hex_x + w, hex_rect.origin.y + 6.0),
                size: Point2D::new(1.5, hex_rect.size.y - 12.0),
            },
            theme.foreground,
        );
    }
    let w_rect = Rect {
        origin: Point2D::new(hex_rect.origin.x + hex_rect.size.x + 8.0, y),
        size: Point2D::new(width_w, INPUT_HEIGHT),
    };
    let w_focused = edit.focus == Some(PropertyFocus::StrokeWidth);
    cx.backend
        .fill_round_rect(w_rect, INPUT_RADIUS, theme.muted);
    if w_focused {
        cx.backend
            .stroke_round_rect(w_rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    let w_owned = format!("{}", stroke_width.round() as i32);
    let w_text = edit.value_for(PropertyFocus::StrokeWidth, &w_owned);
    let w_layout = TextLayout::single_run(
        w_text,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    let w_x = w_rect.origin.x + 12.0;
    cx.backend
        .draw_text(&w_layout, Point2D::new(w_x, w_rect.origin.y + 17.0));
    if edit.caret_visible(PropertyFocus::StrokeWidth) {
        let w = cx.backend.measure_text(w_text, 12.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(w_x + w, w_rect.origin.y + 6.0),
                size: Point2D::new(1.5, w_rect.size.y - 12.0),
            },
            theme.foreground,
        );
    }
    y += INPUT_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Effects section ───────────────────────────────────────────────
// `paint_effects_section` + its row helpers live in
// `property_panel_effects.rs` (split out at the 800-line cap);
// re-exported below so callers keep using `sections::*`.

// ── Export section ────────────────────────────────────────────────

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_export_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    format: crate::widgets::export_dialog::ExportFormat,
    scale: f32,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.export, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    // Left pill: current scale. Right pill: current format. Click
    // anywhere in the section opens the ExportDialog so the user
    // can change either (P3 wiring — full inline popovers will land
    // when a generic dropdown-popover infra ships).
    let scale_label = match crate::widgets::export_dialog::scale_index(scale) {
        1 => "1x",
        3 => "3x",
        _ => "2x",
    };
    paint_dropdown(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        scale_label,
    );
    paint_dropdown(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        format.label(),
    );
    y += INPUT_HEIGHT + 12.0;
    y
}

// NOTE: a future `export_section_rect` walker will live here once
// `hit_test_action` is extended to emit `OpenExportDialog` for
// clicks inside this section. Until then the section is preview-
// only — open the dialog via File menu / Cmd+Shift+P.

// All shared paint primitives + layout constants are imported
// from `property_panel_inputs` via the `pub use` block earlier in
// this file. Keep section-paint helpers here.
