//! Section paint helpers for [`crate::widgets::PropertyPanel`].
//! Split out of `property_panel.rs` to honor the 800-line file
//! ceiling. Each `paint_*_section` returns the y-coordinate just
//! below itself so the parent can chain them.

use crate::document::PropertyFocus;
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::NodeSnapshot;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

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
#[derive(Debug, Clone)]
pub struct PropertyLabels {
    pub tab_design: String,
    pub tab_code: String,
    pub create_component: String,
    pub position: String,
    pub flex_layout: String,
    pub size: String,
    pub layer: String,
    pub opacity: String,
    pub fill: String,
    pub stroke: String,
    pub effects: String,
    pub export: String,
    pub fill_width: String,
    pub fill_height: String,
    pub hug_width: String,
    pub hug_height: String,
    pub clip_content: String,
}

impl PropertyLabels {
    /// Builder — looks up every chrome string the property panel
    /// shows. Falls back to a hardcoded English literal when the
    /// translation layer returns the key itself (some keys aren't
    /// in the TS table because the TS panel hardcodes them).
    pub fn for_document(doc: &crate::document::Document) -> Self {
        // Some labels exist in TS, some don't. `pick` returns the
        // localised string, the EN fallback, OR a literal we
        // hardcode here so the panel never paints a raw dotted key.
        let pick = |key: &'static str, fallback: &'static str| -> String {
            let translated = doc.t(key);
            if translated == key {
                fallback.to_string()
            } else {
                translated.to_string()
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

const PAD_X: f32 = 16.0;
/// Vertical breathing room between a divider line and the next
/// section's label.
const SECTION_GAP: f32 = 8.0;
const ROW_HEIGHT: f32 = 28.0;
/// Input pill height — tuned to match TS Frame inspector
/// (apps/web/src/components/panels/size-section.tsx `Input`
/// renders at ~30 px). Bumped from 26 so values + prefix labels
/// breathe like the TS reference.
const INPUT_HEIGHT: f32 = 30.0;
const INPUT_RADIUS: f32 = 6.0;
/// Title strip at the top of a section ("位置" / "尺寸" / ...).
/// Matches TS `text-[11px] uppercase` headings.
const SECTION_HEADER_HEIGHT: f32 = 24.0;
const TAB_HEIGHT: f32 = 36.0;
const HEADER_HEIGHT: f32 = 30.0;

/// Compute the on-screen rects of every editable input the
/// PropertyPanel hit-tests. Same math as the per-section paint
/// helpers; factored out so paint + hit-test stay aligned.
///
/// Currently emits the X / Y / W / H number inputs. Rotation,
/// opacity, hex, etc. land here as their schema grows.
///
/// `panel_rect` is the rect the panel paints into (origin +
/// width = panel width).
pub fn editable_input_rects(panel_rect: Rect) -> Vec<(PropertyFocus, Rect)> {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;

    // Match the `PropertyPanel::paint` order:
    //   tab_strip (TAB_HEIGHT)
    // → node_header (HEADER_HEIGHT)
    // → create_component (8 + 36 + 12 = 56)
    // → position section header (SECTION_HEADER_HEIGHT) → X/Y row
    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += HEADER_HEIGHT;
    y += 8.0 + 36.0 + 12.0;
    // Position section.
    y += SECTION_HEADER_HEIGHT;
    let x_rect = Rect {
        origin: Point2D::new(x0 + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let y_rect = Rect {
        origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    // Position section: X/Y row + 6 px + Rotation/R row + 12 px + 8 (gap).
    y += INPUT_HEIGHT + 6.0;
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;
    // Flex section: header + 32 px button row + 12 px + section_gap.
    y += SECTION_HEADER_HEIGHT;
    y += 32.0 + 12.0;
    y += SECTION_GAP;
    // Size section: header + W/H row.
    y += SECTION_HEADER_HEIGHT;
    let w_rect = Rect {
        origin: Point2D::new(x0 + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let h_rect = Rect {
        origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    vec![
        (PropertyFocus::PositionX, x_rect),
        (PropertyFocus::PositionY, y_rect),
        (PropertyFocus::SizeW, w_rect),
        (PropertyFocus::SizeH, h_rect),
    ]
}

// ── Tab strip ─────────────────────────────────────────────────────

pub fn paint_tab_strip(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let pad = 14.0;
    let tab_y = y + 6.0;
    let active_w = (cx.backend.measure_text(&labels.tab_design, 13.0) + 24.0).max(48.0);
    let active_rect = Rect {
        origin: Point2D::new(x + pad, tab_y),
        size: Point2D::new(active_w, 26.0),
    };
    cx.backend.fill_round_rect(active_rect, 6.0, theme.muted);
    let active_label = TextLayout::single_run(
        &labels.tab_design,
        "system-ui",
        13.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &active_label,
        Point2D::new(active_rect.origin.x + 12.0, active_rect.origin.y + 18.0),
    );
    let inactive_label = TextLayout::single_run(
        &labels.tab_code,
        "system-ui",
        13.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &inactive_label,
        Point2D::new(
            active_rect.origin.x + active_rect.size.x + 14.0,
            tab_y + 18.0,
        ),
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
        &labels.create_component,
        "system-ui",
        13.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    let label_w = cx.backend.measure_text(&labels.create_component, 13.0);
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
    let mut y = paint_section_label(cx, theme, &labels.position, x, y, width);
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
    // TS uses RotateCw for the rotation input (layout-section.tsx).
    paint_input_with_icon(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        Icon::RotateCw,
        "0",
        Some("°"),
    );
    paint_input_with_prefix(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        "R",
        "0",
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
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, &labels.flex_layout, x, y, width);
    // TS layout-section.tsx uses Columns3 / Rows3 / LayoutGrid for
    // the three flex modes; LayoutGrid is the default-active mode
    // (Free / 自由布局).
    let btn_w = 56.0;
    let gap = 8.0;
    let row_x = x + PAD_X;
    let icons = [Icon::LayoutGrid, Icon::Rows3, Icon::Columns3];
    for (i, icon) in icons.iter().enumerate() {
        let bx = row_x + i as f32 * (btn_w + gap);
        let rect = Rect {
            origin: Point2D::new(bx, y),
            size: Point2D::new(btn_w, 32.0),
        };
        let is_active = i == 0;
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
    }
    y += 32.0 + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Size section ──────────────────────────────────────────────────

pub fn paint_size_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, &labels.size, x, y, width);
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
    paint_check_row(cx, theme, x + PAD_X, y, half_w, &labels.fill_width);
    paint_check_row(
        cx,
        theme,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
        &labels.fill_height,
    );
    y += row_h;
    paint_check_row(cx, theme, x + PAD_X, y, half_w, &labels.hug_width);
    paint_check_row(
        cx,
        theme,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
        &labels.hug_height,
    );
    y += row_h;
    paint_check_row(cx, theme, x + PAD_X, y, usable_w, &labels.clip_content);
    y += row_h + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

fn paint_check_row(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32, _w: f32, label: &str) {
    let box_rect = Rect {
        origin: Point2D::new(x, y + 3.0),
        size: Point2D::new(16.0, 16.0),
    };
    cx.backend
        .stroke_round_rect(box_rect, 4.0, theme.border, 1.0);
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
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, &labels.layer, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let row = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w / 2.0 - 4.0, INPUT_HEIGHT),
    };
    paint_input_with_suffix(cx, theme, row, "100", "%");
    y += INPUT_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Fill section ──────────────────────────────────────────────────

pub fn paint_fill_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label_with_add(cx, theme, &labels.fill, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let fill = snapshot.fill.unwrap_or(Color::WHITE);
    let swatch_rect = Rect {
        origin: Point2D::new(x + PAD_X, y + 2.0),
        size: Point2D::new(22.0, 22.0),
    };
    cx.backend.fill_round_rect(swatch_rect, 4.0, fill);
    cx.backend
        .stroke_round_rect(swatch_rect, 4.0, theme.border, 1.0);
    let dropdown_rect = Rect {
        origin: Point2D::new(swatch_rect.origin.x + swatch_rect.size.x + 6.0, y),
        size: Point2D::new(usable_w - 22.0 - 6.0 - 50.0 - 22.0 - 12.0, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(dropdown_rect, INPUT_RADIUS, theme.muted);
    let label = TextLayout::single_run(
        "纯色",
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(dropdown_rect.origin.x + 10.0, dropdown_rect.origin.y + 17.0),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(
            dropdown_rect.origin.x + dropdown_rect.size.x - 22.0,
            dropdown_rect.origin.y + 5.0,
        ),
        16.0,
        theme.muted_foreground,
        1.4,
    );
    let pct_rect = Rect {
        origin: Point2D::new(dropdown_rect.origin.x + dropdown_rect.size.x + 6.0, y),
        size: Point2D::new(50.0, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(pct_rect, INPUT_RADIUS, theme.muted);
    let pct = TextLayout::single_run(
        "100",
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &pct,
        Point2D::new(pct_rect.origin.x + 10.0, pct_rect.origin.y + 17.0),
    );
    let pct_unit = TextLayout::single_run(
        "%",
        "system-ui",
        12.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &pct_unit,
        Point2D::new(
            pct_rect.origin.x + pct_rect.size.x - 14.0,
            pct_rect.origin.y + 17.0,
        ),
    );
    draw_icon(
        cx.backend,
        Icon::Close,
        Point2D::new(
            pct_rect.origin.x + pct_rect.size.x + 8.0,
            y + (INPUT_HEIGHT - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.4,
    );
    y += INPUT_HEIGHT + 6.0;
    let hex_text = format_color_hex(fill);
    let hex_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(hex_rect, INPUT_RADIUS, theme.muted);
    cx.backend.fill_round_rect(
        Rect {
            origin: Point2D::new(hex_rect.origin.x + 6.0, hex_rect.origin.y + 5.0),
            size: Point2D::new(16.0, 16.0),
        },
        3.0,
        fill,
    );
    let hex = TextLayout::single_run(
        &hex_text,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &hex,
        Point2D::new(hex_rect.origin.x + 30.0, hex_rect.origin.y + 17.0),
    );
    y += INPUT_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Stroke section ────────────────────────────────────────────────

pub fn paint_stroke_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, &labels.stroke, x, y, width);
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
    cx.backend
        .fill_round_rect(hex_rect, INPUT_RADIUS, theme.muted);
    cx.backend.fill_round_rect(
        Rect {
            origin: Point2D::new(hex_rect.origin.x + 6.0, hex_rect.origin.y + 5.0),
            size: Point2D::new(16.0, 16.0),
        },
        3.0,
        stroke_color,
    );
    let hex_text = format_color_hex(stroke_color);
    let hex = TextLayout::single_run(
        &hex_text,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &hex,
        Point2D::new(hex_rect.origin.x + 30.0, hex_rect.origin.y + 17.0),
    );
    let w_rect = Rect {
        origin: Point2D::new(hex_rect.origin.x + hex_rect.size.x + 8.0, y),
        size: Point2D::new(width_w, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(w_rect, INPUT_RADIUS, theme.muted);
    let w_text = format!("{}", stroke_width.round() as i32);
    let w_label = TextLayout::single_run(
        &w_text,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &w_label,
        Point2D::new(w_rect.origin.x + 12.0, w_rect.origin.y + 17.0),
    );
    y += INPUT_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Effects section ───────────────────────────────────────────────

pub fn paint_effects_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let y = paint_section_label_with_add(cx, theme, &labels.effects, x, y, width);
    let after = y + 8.0;
    paint_section_divider(cx, theme, x, after, width);
    after + SECTION_GAP
}

// ── Export section ────────────────────────────────────────────────

pub fn paint_export_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, &labels.export, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    paint_dropdown(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        "1x",
    );
    paint_dropdown(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        "PNG",
    );
    y += INPUT_HEIGHT + 12.0;
    y
}

// ── Shared helpers ────────────────────────────────────────────────

pub fn paint_section_label(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    x: f32,
    y: f32,
    _w: f32,
) -> f32 {
    // 11.5 px (rendered at 12) muted-foreground header — matches
    // TS `text-[11px] tracking-wide text-muted-foreground`.
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

fn paint_section_divider(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32, width: f32) {
    // Full-width hairline (no PAD_X inset) — matches the TS
    // PropertyPanel where section dividers go edge-to-edge.
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(width, 1.0),
        },
        theme.border,
    );
}

fn paint_input_with_prefix(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    prefix: &str,
    value: &str,
) {
    paint_input_with_prefix_focused(cx, theme, rect, prefix, value, false, false);
}

/// Same as [`paint_input_with_prefix`] but with explicit focus +
/// caret-blink controls. The property panel uses this to render
/// the live edit-buffer of the focused row with a primary-tinted
/// border + a single-pixel blinking caret.
fn paint_input_with_prefix_focused(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    prefix: &str,
    value: &str,
    focused: bool,
    caret_visible: bool,
) {
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    if focused {
        cx.backend
            .stroke_round_rect(rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    // Prefix label hugs the left padding.
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
    // Value uses the real text-measure API so it stays anchored at
    // a consistent gap from the prefix regardless of digit count.
    let prefix_w = cx.backend.measure_text(prefix, 12.0);
    let value_x = rect.origin.x + 10.0 + prefix_w + 8.0;
    let baseline_y = rect.origin.y + rect.size.y / 2.0 + 4.0;
    let value_layout = TextLayout::single_run(
        value,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&value_layout, Point2D::new(value_x, baseline_y));
    if caret_visible {
        let value_w = cx.backend.measure_text(value, 12.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(value_x + value_w, rect.origin.y + 6.0),
                size: Point2D::new(1.5, rect.size.y - 12.0),
            },
            theme.foreground,
        );
    }
}

fn paint_input_with_suffix(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    value: &str,
    unit: &str,
) {
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
        Point2D::new(rect.origin.x + 10.0, rect.origin.y + 17.0),
    );
    let unit_layout = TextLayout::single_run(
        unit,
        "system-ui",
        12.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &unit_layout,
        Point2D::new(rect.origin.x + rect.size.x - 14.0, rect.origin.y + 17.0),
    );
}

fn paint_input_with_icon(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    icon: Icon,
    value: &str,
    unit: Option<&str>,
) {
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + 6.0, rect.origin.y + 5.0),
        14.0,
        theme.muted_foreground,
        1.4,
    );
    let value_layout = TextLayout::single_run(
        value,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &value_layout,
        Point2D::new(rect.origin.x + 26.0, rect.origin.y + 17.0),
    );
    if let Some(u) = unit {
        let unit_layout = TextLayout::single_run(
            u,
            "system-ui",
            12.0,
            to_jian_color(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &unit_layout,
            Point2D::new(rect.origin.x + rect.size.x - 14.0, rect.origin.y + 17.0),
        );
    }
}

fn paint_dropdown(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, value: &str) {
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
        Point2D::new(rect.origin.x + 12.0, rect.origin.y + 17.0),
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

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[allow(dead_code)]
const _ROW_HEIGHT_KEEP: f32 = ROW_HEIGHT;
