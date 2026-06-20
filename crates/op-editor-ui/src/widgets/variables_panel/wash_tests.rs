//! Fit-content hover-wash tests for the VariablesPanel (#26 variant-header
//! value + #3 add-variable footer). Split into its own file so
//! `variables_panel/tests.rs` doesn't grow past the 800-line cap.
//!
//! Both the variant-header value (`Default v`) and the footer `+ <label> v`
//! button sit inside a much wider hit target, but their hover highlight should
//! hug just the visible content (+ small L/R padding) rather than washing the
//! whole cell / footer width — and must stay clamped to the hit rect so a long
//! localized value can't bleed into a neighbouring column.

use super::*;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

/// Captures `fill_round_rect` calls so the hover wash rect/colour can be
/// asserted without a real GPU.
#[derive(Default)]
struct RoundFillCapture {
    round_fills: Vec<(Rect, f32, Color)>,
}

impl crate::RenderBackend for RoundFillCapture {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &crate::TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.round_fills.push((rect, radius, color));
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn color_eq(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 0.001
        && (a.g - b.g).abs() < 0.001
        && (a.b - b.b).abs() < 0.001
        && (a.a - b.a).abs() < 0.001
}

fn hover_wash(s: &EditorState) -> Option<Rect> {
    let p = VariablesPanel::for_editor(s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, 480.0),
    };
    let mut backend = RoundFillCapture::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    p.paint(&mut cx, rect);
    let button_hover = p.theme.button_hover;
    backend
        .round_fills
        .iter()
        .find(|(_, _, color)| color_eq(*color, button_hover))
        .map(|(r, _, _)| *r)
}

#[test]
fn variant_header_hover_wash_hugs_value_not_full_column() {
    let mut s = EditorState::new();
    s.editor_ui.variables_panel_hover = Some(VariablesPanelButton::VariantHeader(0));
    let wash = hover_wash(&s).expect("hovered variant header should paint a button_hover wash");
    // "Default" ≈ 7 ASCII chars @13px ≈ 50px → wash hugs value + chevron +
    // padding (~80px), far under the >=156px value-column cell.
    assert!(
        wash.size.x < 110.0,
        "variant-header wash should hug `value v`, got {}",
        wash.size.x
    );
}

#[test]
fn variant_header_hover_wash_clamped_to_column_for_long_variant() {
    // A long variant name must clamp the wash to its hit column (`col_w.min(176)`)
    // rather than overflowing into the neighbouring variant column.
    let mut s = EditorState::new();
    let long = "a-very-long-variant-name-that-would-overflow-the-column";
    s.doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec![long.to_string()]);
    s.editor_ui.variables_panel_hover = Some(VariablesPanelButton::VariantHeader(0));
    let wash = hover_wash(&s).expect("hovered variant header should paint a button_hover wash");
    // Unclamped this would be ~315px (54 ASCII chars). The hit column is
    // `col_w.min(176)` (>= the 156px floor), so a clamped long-variant wash
    // lands in [156, 176] — a short "Default" fallback (~79px, if the long
    // variant failed to render) would fall below this range.
    assert!(
        (150.0..=177.0).contains(&wash.size.x),
        "long-variant wash should clamp to its [156,176]px column, got {}",
        wash.size.x
    );
}

#[test]
fn footer_add_variable_hover_wash_left_aligned_and_hugs_content() {
    let mut s = EditorState::new();
    // Short CJK label → fit-content pill is clearly narrower than the button.
    s.editor_ui.locale = Locale::ZhCn;
    s.editor_ui.variables_panel_hover = Some(VariablesPanelButton::AddVariable);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, 480.0),
    };
    let button = add_variable_rect(rect);
    let wash = hover_wash(&s).expect("hovered add-variable footer should paint a button_hover wash");
    // Pill is flush with the footer's (and rows') left edge — no centering gap.
    assert!(
        (wash.origin.x - button.origin.x).abs() < 0.01,
        "footer wash should be left-aligned with the button, got {} vs {}",
        wash.origin.x,
        button.origin.x
    );
    // …hugs the `+ label v` content, well under the full 164px button…
    assert!(
        wash.size.x < button.size.x - 20.0,
        "footer wash should hug content, got {} vs button {}",
        wash.size.x,
        button.size.x
    );
    // …and stays inside the footer button bounds.
    assert!(wash.origin.x + wash.size.x <= button.origin.x + button.size.x + 0.01);
}
