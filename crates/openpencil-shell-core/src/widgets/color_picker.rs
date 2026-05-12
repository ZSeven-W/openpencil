//! Floating HSV colour picker — opens from a Fill / Stroke swatch
//! click in the PropertyPanel. SV box + hue strip + R/G/B readout
//! + hex; mirrors the layout of common design-tool pickers.

use crate::document::{ColorPickerDrag, ColorPickerState, ColorTarget, Document};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

pub const PICKER_WIDTH: f32 = 240.0;
pub const PICKER_HEIGHT: f32 = 336.0;

const PAD: f32 = 8.0;
const HEADER_HEIGHT: f32 = 28.0;
const SV_HEIGHT: f32 = 140.0;
const ROW_GAP: f32 = 8.0;
const HUE_HEIGHT: f32 = 12.0;
const HUE_HANDLE_R: f32 = 7.0;
const SV_HANDLE_R: f32 = 6.0;
const SWATCH_SIZE: f32 = 28.0;
const RGB_BOX_W: f32 = 56.0;
const RGB_BOX_H: f32 = 32.0;
const HEX_HEIGHT: f32 = 28.0;

/// What a press inside the picker did. The host translates these
/// to a `ColorPickerDrag` it then keeps live until release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPickerHit {
    SvBox,
    HueSlider,
    Eyedropper,
    Close,
    /// Click inside the panel that doesn't hit a control — swallows
    /// to keep the picker open.
    Inside,
}

pub struct ColorPicker<'a> {
    pub id: WidgetId,
    pub state: ColorPickerState,
    pub theme: Theme,
    document: &'a Document,
}

impl<'a> ColorPicker<'a> {
    pub fn for_state(doc: &'a Document, state: ColorPickerState) -> Self {
        Self {
            id: WidgetId::new(5100),
            state,
            theme: doc.theme(),
            document: doc,
        }
    }

    /// Resolve the picker's anchor (top-left) in viewport coords —
    /// pinned to the right rail near the Fill section. Hosts use
    /// this for paint AND hit-test so they stay in sync.
    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let right_rail_left = viewport_w - self.document.ui.property_panel_width;
        let x = (right_rail_left - PICKER_WIDTH - 8.0).max(8.0);
        // Center vertically around the swatch the user clicked, then
        // clamp so the panel stays fully on-screen.
        let mut y = self.state.anchor_y - PICKER_HEIGHT / 2.0;
        let top_min = crate::widgets::TOP_BAR_HEIGHT + 8.0;
        let bottom_max = (viewport_h - PICKER_HEIGHT - 8.0).max(top_min);
        if y < top_min {
            y = top_min;
        }
        if y > bottom_max {
            y = bottom_max;
        }
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(PICKER_WIDTH, PICKER_HEIGHT),
        }
    }

    /// Map a press point inside the picker to a hit (or None if the
    /// point is outside the panel rect entirely).
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<ColorPickerHit> {
        if !rect_contains(panel, point) {
            return None;
        }
        // Close chip sits visually on top of the SV box's top-right
        // corner — check it first so its presses don't get swallowed
        // by the SV drag handler.
        let close = close_rect(panel);
        if rect_contains(close, point) {
            return Some(ColorPickerHit::Close);
        }
        let sv = sv_rect(panel);
        if rect_contains(sv, point) {
            return Some(ColorPickerHit::SvBox);
        }
        let hue = hue_rect(panel);
        if rect_contains(hue, point) {
            return Some(ColorPickerHit::HueSlider);
        }
        let eye = eyedropper_rect(panel);
        if rect_contains(eye, point) {
            return Some(ColorPickerHit::Eyedropper);
        }
        Some(ColorPickerHit::Inside)
    }

    /// Compute new HSV when dragging inside the SV box.
    pub fn sv_at(&self, panel: Rect, point: Point2D) -> (f32, f32) {
        let sv = sv_rect(panel);
        let x = ((point.x - sv.origin.x) / sv.size.x).clamp(0.0, 1.0);
        let y = ((point.y - sv.origin.y) / sv.size.y).clamp(0.0, 1.0);
        (x, 1.0 - y)
    }

    /// Compute new hue when dragging across the hue strip.
    pub fn hue_at(&self, panel: Rect, point: Point2D) -> f32 {
        let h = hue_rect(panel);
        let t = ((point.x - h.origin.x) / h.size.x).clamp(0.0, 1.0);
        t * 360.0
    }
}

impl<'a> Widget for ColorPicker<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &crate::widgets::LayoutCx) -> crate::widgets::LayoutBox {
        crate::widgets::LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(PICKER_WIDTH, PICKER_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        paint_picker(cx, &self.theme, &self.state, rect);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label("Color picker");
        node
    }
}

fn paint_picker(cx: &mut PaintCx<'_>, theme: &Theme, state: &ColorPickerState, panel: Rect) {
    // Card background + outline.
    cx.backend.fill_round_rect(panel, 8.0, theme.card);
    cx.backend.stroke_round_rect(panel, 8.0, theme.border, 1.0);

    // 0) Header row — picker title + close chip.
    let title = match state.target {
        ColorTarget::Fill => "Fill",
        ColorTarget::Stroke => "Stroke",
    };
    let title_layout = TextLayout::single_run(
        title,
        "system-ui",
        13.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title_layout,
        Point2D::new(panel.origin.x + PAD, header_top(panel) + 19.0),
    );

    // 1) SV box — saturation × value gradient under the current hue.
    //    Software-painted with a column-by-column gradient, then a
    //    vertical black overlay. Quick + faithful enough.
    paint_sv_box(cx, sv_rect(panel), state.hue, state.sat, state.val);

    // 2) Hue strip — 6-segment rainbow with a circular handle.
    paint_hue_strip(cx, hue_rect(panel), state.hue, theme);

    // 3) Eyedropper + current swatch.
    let eye = eyedropper_rect(panel);
    draw_icon(
        cx.backend,
        Icon::Pencil,
        Point2D::new(eye.origin.x, eye.origin.y + 4.0),
        16.0,
        theme.muted_foreground,
        1.5,
    );
    let swatch = swatch_rect(panel);
    let cur = hsv_to_rgb(state.hue, state.sat, state.val);
    cx.backend.fill_round_rect(swatch, 14.0, cur);
    cx.backend
        .stroke_round_rect(swatch, 14.0, theme.border, 1.0);

    // 4) R/G/B numeric boxes.
    let (r, g, b) = (
        (cur.r * 255.0).round() as u8,
        (cur.g * 255.0).round() as u8,
        (cur.b * 255.0).round() as u8,
    );
    let rgb_y = sv_rect(panel).origin.y + SV_HEIGHT + ROW_GAP + 36.0 + ROW_GAP;
    for (i, (label, value)) in [("R", r), ("G", g), ("B", b)].iter().enumerate() {
        let value = *value;
        let label = *label;
        let bx = panel.origin.x + PAD + (i as f32) * (RGB_BOX_W + 8.0);
        let box_rect = Rect {
            origin: Point2D::new(bx, rgb_y),
            size: Point2D::new(RGB_BOX_W, RGB_BOX_H),
        };
        cx.backend
            .stroke_round_rect(box_rect, 6.0, theme.border, 1.0);
        let val_str = value.to_string();
        let text_w = cx.backend.measure_text(&val_str, 14.0);
        let text = TextLayout::single_run(
            &val_str,
            "system-ui",
            14.0,
            to_jian(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &text,
            Point2D::new(bx + (RGB_BOX_W - text_w) / 2.0, rgb_y + 21.0),
        );
        let lab_w = cx.backend.measure_text(label, 12.0);
        let lab = TextLayout::single_run(
            label,
            "system-ui",
            12.0,
            to_jian(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &lab,
            Point2D::new(bx + (RGB_BOX_W - lab_w) / 2.0, rgb_y + RGB_BOX_H + 14.0),
        );
    }

    // 5) Hex bar at the bottom.
    let hex_rect = Rect {
        origin: Point2D::new(
            panel.origin.x + PAD,
            panel.origin.y + PICKER_HEIGHT - PAD - HEX_HEIGHT,
        ),
        size: Point2D::new(PICKER_WIDTH - PAD * 2.0, HEX_HEIGHT),
    };
    cx.backend.fill_round_rect(hex_rect, 6.0, theme.muted);
    cx.backend
        .stroke_round_rect(hex_rect, 6.0, theme.primary, 1.0);
    let mini = Rect {
        origin: Point2D::new(hex_rect.origin.x + 6.0, hex_rect.origin.y + 6.0),
        size: Point2D::new(16.0, 16.0),
    };
    cx.backend.fill_round_rect(mini, 3.0, cur);
    cx.backend.stroke_round_rect(mini, 3.0, theme.border, 1.0);
    let hex_str = format!("#{:02x}{:02x}{:02x}", r, g, b);
    let hex_layout = TextLayout::single_run(
        &hex_str,
        "system-ui",
        13.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &hex_layout,
        Point2D::new(hex_rect.origin.x + 32.0, hex_rect.origin.y + 19.0),
    );

    // 6) Close (×) chip — filled hover-card so it actually reads
    //    against the SV gradient behind it.
    let close = close_rect(panel);
    let chip_bg = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.45,
    };
    cx.backend.fill_round_rect(close, 11.0, chip_bg);
    cx.backend.stroke_round_rect(close, 11.0, theme.border, 1.0);
    draw_icon(
        cx.backend,
        Icon::Close,
        Point2D::new(close.origin.x + 4.0, close.origin.y + 4.0),
        14.0,
        Color::WHITE,
        1.8,
    );

    // SV handle and hue handle on top of everything.
    paint_sv_handle(cx, sv_rect(panel), state, theme);
    paint_hue_handle(cx, hue_rect(panel), state.hue, theme);
}

fn paint_sv_box(cx: &mut PaintCx<'_>, sv: Rect, hue: f32, _sat: f32, _val: f32) {
    // Column gradient: at value=1, the row interpolates from white
    // (sat=0) to the hue's pure colour (sat=1); a vertical overlay
    // darkens to black as value drops to 0. Approximate with N=18
    // column strips for cheap painting.
    const STEPS_X: u32 = 18;
    const STEPS_Y: u32 = 14;
    let pure = hsv_to_rgb(hue, 1.0, 1.0);
    let cell_w = sv.size.x / STEPS_X as f32;
    let cell_h = sv.size.y / STEPS_Y as f32;
    for ix in 0..STEPS_X {
        for iy in 0..STEPS_Y {
            let s = (ix as f32 + 0.5) / STEPS_X as f32;
            let v = 1.0 - (iy as f32 + 0.5) / STEPS_Y as f32;
            let r = (1.0 - s) * 1.0 + s * pure.r;
            let g = (1.0 - s) * 1.0 + s * pure.g;
            let b = (1.0 - s) * 1.0 + s * pure.b;
            let color = Color {
                r: r * v,
                g: g * v,
                b: b * v,
                a: 1.0,
            };
            let cell = Rect {
                origin: Point2D::new(
                    sv.origin.x + ix as f32 * cell_w,
                    sv.origin.y + iy as f32 * cell_h,
                ),
                size: Point2D::new(cell_w + 0.5, cell_h + 0.5),
            };
            cx.backend.fill_rect(cell, color);
        }
    }
    cx.backend.stroke_round_rect(
        sv,
        4.0,
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.25,
        },
        1.0,
    );
}

fn paint_hue_strip(cx: &mut PaintCx<'_>, hue: Rect, _current: f32, _theme: &Theme) {
    const STEPS: u32 = 32;
    let cell_w = hue.size.x / STEPS as f32;
    for i in 0..STEPS {
        let h = (i as f32 + 0.5) / STEPS as f32 * 360.0;
        let c = hsv_to_rgb(h, 1.0, 1.0);
        let cell = Rect {
            origin: Point2D::new(hue.origin.x + i as f32 * cell_w, hue.origin.y),
            size: Point2D::new(cell_w + 0.5, hue.size.y),
        };
        cx.backend.fill_round_rect(cell, hue.size.y / 2.0, c);
    }
}

fn paint_sv_handle(cx: &mut PaintCx<'_>, sv: Rect, state: &ColorPickerState, theme: &Theme) {
    let x = sv.origin.x + state.sat * sv.size.x;
    let y = sv.origin.y + (1.0 - state.val) * sv.size.y;
    let ring = Rect {
        origin: Point2D::new(x - SV_HANDLE_R, y - SV_HANDLE_R),
        size: Point2D::new(SV_HANDLE_R * 2.0, SV_HANDLE_R * 2.0),
    };
    cx.backend.stroke_oval(ring, theme.background, 3.0);
    cx.backend.stroke_oval(ring, Color::WHITE, 1.5);
}

fn paint_hue_handle(cx: &mut PaintCx<'_>, hue: Rect, current_hue: f32, theme: &Theme) {
    let cx0 = hue.origin.x + (current_hue / 360.0).clamp(0.0, 1.0) * hue.size.x;
    let cy0 = hue.origin.y + hue.size.y / 2.0;
    let ring = Rect {
        origin: Point2D::new(cx0 - HUE_HANDLE_R, cy0 - HUE_HANDLE_R),
        size: Point2D::new(HUE_HANDLE_R * 2.0, HUE_HANDLE_R * 2.0),
    };
    let center = Rect {
        origin: Point2D::new(cx0 - 5.0, cy0 - 5.0),
        size: Point2D::new(10.0, 10.0),
    };
    cx.backend
        .fill_oval(center, hsv_to_rgb(current_hue, 1.0, 1.0));
    cx.backend.stroke_oval(ring, theme.background, 1.0);
    cx.backend.stroke_oval(ring, Color::WHITE, 2.0);
}

fn header_top(panel: Rect) -> f32 {
    panel.origin.y + PAD
}

fn sv_rect(panel: Rect) -> Rect {
    Rect {
        origin: Point2D::new(panel.origin.x + PAD, header_top(panel) + HEADER_HEIGHT),
        size: Point2D::new(PICKER_WIDTH - PAD * 2.0, SV_HEIGHT),
    }
}

fn hue_rect(panel: Rect) -> Rect {
    let y = sv_rect(panel).origin.y + SV_HEIGHT + ROW_GAP + 8.0;
    let x = panel.origin.x + PAD + SWATCH_SIZE + 24.0 + 8.0;
    let w = PICKER_WIDTH - PAD * 2.0 - SWATCH_SIZE - 24.0 - 8.0;
    Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, HUE_HEIGHT),
    }
}

fn eyedropper_rect(panel: Rect) -> Rect {
    let y = sv_rect(panel).origin.y + SV_HEIGHT + ROW_GAP + 6.0;
    Rect {
        origin: Point2D::new(panel.origin.x + PAD, y),
        size: Point2D::new(20.0, 20.0),
    }
}

fn swatch_rect(panel: Rect) -> Rect {
    let y = sv_rect(panel).origin.y + SV_HEIGHT + ROW_GAP;
    Rect {
        origin: Point2D::new(panel.origin.x + PAD + 24.0, y),
        size: Point2D::new(SWATCH_SIZE, SWATCH_SIZE),
    }
}

/// Close chip — pinned to the header row above the SV box so it
/// doesn't steal clicks meant for the colour gradient.
fn close_rect(panel: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            panel.origin.x + PICKER_WIDTH - PAD - 22.0,
            header_top(panel) + (HEADER_HEIGHT - 22.0) / 2.0,
        ),
        size: Point2D::new(22.0, 22.0),
    }
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.y >= r.origin.y
        && p.x <= r.origin.x + r.size.x
        && p.y <= r.origin.y + r.size.y
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

/// HSV → RGB, h 0..360, s/v 0..1. Returns alpha 1.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let hh = h / 60.0;
    let x = c * (1.0 - (hh.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hh as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    Color {
        r: r1 + m,
        g: g1 + m,
        b: b1 + m,
        a: 1.0,
    }
}

/// RGB (0..1) → HSV (h 0..360, s 0..1, v 0..1).
pub fn rgb_to_hsv(c: Color) -> (f32, f32, f32) {
    let (r, g, b) = (c.r, c.g, c.b);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let v = max;
    let delta = max - min;
    let s = if max <= 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

/// Match the host's `apply_cursor_move` drag dispatch with the
/// document's stored kind — convenience the host calls when
/// translating a press into the persistent drag state.
pub fn drag_for_hit(hit: ColorPickerHit) -> Option<ColorPickerDrag> {
    match hit {
        ColorPickerHit::SvBox => Some(ColorPickerDrag::SvBox),
        ColorPickerHit::HueSlider => Some(ColorPickerDrag::HueSlider),
        _ => None,
    }
}

pub fn target_label(t: ColorTarget) -> &'static str {
    match t {
        ColorTarget::Fill => "Fill",
        ColorTarget::Stroke => "Stroke",
    }
}
