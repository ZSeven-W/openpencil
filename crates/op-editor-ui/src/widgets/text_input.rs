//! OP widget facade for Jian's reusable single-line text input view.

use super::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use jian_core::text_input::TextInputState;
use jian_widgets::components::text_area::TextArea;
use jian_widgets::components::text_input::TextInputView;
use jian_widgets::Tokens;

const DEFAULT_WIDTH: f32 = 160.0;
const DEFAULT_HEIGHT: f32 = 30.0;

pub struct TextInputWidget {
    pub id: WidgetId,
    pub state: TextInputState,
    pub placeholder: String,
    pub focused: bool,
    pub font_size: f32,
    pub now_ms: u64,
    pub pad_x: f32,
    pub tokens: Tokens,
}

impl TextInputWidget {
    pub fn new(id: u64, state: TextInputState) -> Self {
        Self {
            id: WidgetId::new(id),
            state,
            placeholder: String::new(),
            focused: false,
            font_size: 13.0,
            now_ms: 0,
            pad_x: 8.0,
            tokens: Tokens::dark(),
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }
}

impl Widget for TextInputWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width.max(DEFAULT_WIDTH), DEFAULT_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let view = TextInputView {
            state: &self.state,
            placeholder: &self.placeholder,
            focused: self.focused,
            font_size: self.font_size,
            now_ms: self.now_ms,
            pad_x: self.pad_x,
            baseline_delta_y: 0.0,
            mask: None,
        };
        view.paint(cx.backend, rect, &self.tokens);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::TextInput);
        let label = if self.placeholder.is_empty() {
            self.state.text()
        } else {
            &self.placeholder
        };
        node.set_label(label);
        node
    }
}

/// Geometry captured with the real paint backend for one single-line input.
/// Hosts retain the latest snapshot so pointer and IME placement use the same
/// font metrics as the glyphs on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct SingleLinePaintGeometry {
    source: String,
    caret: usize,
    composition: Option<(String, usize)>,
    rect: Rect,
    glyph_midpoints: Vec<(usize, f32)>,
    caret_rect: Rect,
}

impl SingleLinePaintGeometry {
    pub fn measure(
        state: &TextInputState,
        rect: Rect,
        font_size: f32,
        pad_x: f32,
        backend: &mut dyn RenderBackend,
    ) -> Self {
        let caret = jian_core::text_input::prev_char_boundary(state.text(), state.caret());
        let composition = state
            .composition()
            .filter(|value| !value.text.is_empty())
            .map(|value| {
                let cursor = jian_core::text_input::prev_char_boundary(&value.text, value.cursor);
                (value.text.clone(), cursor)
            });
        let mut caret_px = backend.measure_text_family(&state.text()[..caret], font_size, "Inter");
        if let Some((text, cursor)) = composition.as_ref() {
            caret_px += backend.measure_text_family(&text[..*cursor], font_size, "Inter");
        }
        let visible_w = (rect.size.x - pad_x * 2.0).max(0.0);
        let shift = (caret_px - visible_w).max(0.0);
        let base_x = rect.origin.x + pad_x - shift;
        let mut x = 0.0;
        let mut glyph_midpoints = Vec::with_capacity(state.text().chars().count());
        for (byte, ch) in state.text().char_indices() {
            let mut buf = [0; 4];
            let width = backend.measure_text_family(ch.encode_utf8(&mut buf), font_size, "Inter");
            glyph_midpoints.push((byte, base_x + x + width / 2.0));
            x += width;
        }
        let caret_h = font_size + 3.0;
        let caret_rect = Rect::xywh(
            base_x + caret_px,
            rect.origin.y + (rect.size.y - caret_h) / 2.0,
            1.5,
            caret_h,
        );
        Self {
            source: state.text().to_owned(),
            caret,
            composition,
            rect,
            glyph_midpoints,
            caret_rect,
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn byte_offset_at(
        &self,
        state: &TextInputState,
        point: Point2D,
        clamp_to_line: bool,
    ) -> Option<usize> {
        if state.text() != self.source || (!clamp_to_line && !self.rect.contains(point)) {
            return None;
        }
        for &(byte, midpoint) in &self.glyph_midpoints {
            if point.x < midpoint {
                return Some(byte);
            }
        }
        Some(self.source.len())
    }

    pub fn caret_rect(&self, state: &TextInputState) -> Option<Rect> {
        let caret = jian_core::text_input::prev_char_boundary(state.text(), state.caret());
        let composition = state
            .composition()
            .filter(|value| !value.text.is_empty())
            .map(|value| {
                let cursor = jian_core::text_input::prev_char_boundary(&value.text, value.cursor);
                (value.text.as_str(), cursor)
            });
        let composition_matches = match (&self.composition, composition) {
            (None, None) => true,
            (Some((stored, stored_cursor)), Some((text, cursor))) => {
                stored == text && *stored_cursor == cursor
            }
            _ => false,
        };
        (state.text() == self.source && caret == self.caret && composition_matches)
            .then_some(self.caret_rect)
    }
}

/// Resolve a click before the first real paint. This fallback follows the same
/// scroll and midpoint algorithm but uses the backend's conservative width
/// estimate; hosts prefer [`SingleLinePaintGeometry`] after paint.
pub fn single_line_byte_offset_at(
    state: &TextInputState,
    rect: Rect,
    point: Point2D,
    font_size: f32,
    pad_x: f32,
) -> Option<usize> {
    if !rect.contains(point) {
        return None;
    }
    let mut backend = MeasureOnlyBackend;
    SingleLinePaintGeometry::measure(state, rect, font_size, pad_x, &mut backend)
        .byte_offset_at(state, point, false)
}

/// Resolve a pointer position in a multiline text area before a real paint
/// geometry snapshot exists. The same Jian [`TextArea`] line wrapping,
/// visible-window, and glyph-midpoint algorithm used by paint performs the
/// inverse hit-test; the fallback backend supplies conservative advances.
pub fn multiline_byte_offset_at(
    state: &TextInputState,
    rect: Rect,
    point: Point2D,
    font_size: f32,
    pad_x: f32,
    max_visible_lines: usize,
) -> Option<usize> {
    if !rect.contains(point) {
        return None;
    }
    let area = TextArea {
        state,
        placeholder: "",
        focused: true,
        font_size,
        now_ms: 0,
        pad_x,
        max_visible_lines,
    };
    let mut backend = MeasureOnlyBackend;
    Some(area.byte_offset_at(&mut backend, rect, point, &jian_widgets::Tokens::dark()))
}

/// Estimated caret rectangle before a real paint geometry snapshot exists.
pub fn single_line_caret_rect(
    state: &TextInputState,
    rect: Rect,
    font_size: f32,
    pad_x: f32,
) -> Rect {
    let mut backend = MeasureOnlyBackend;
    SingleLinePaintGeometry::measure(state, rect, font_size, pad_x, &mut backend).caret_rect
}

struct MeasureOnlyBackend;

impl RenderBackend for MeasureOnlyBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
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

#[cfg(test)]
mod tests {
    use super::*;

    struct ProportionalBackend;

    impl RenderBackend for ProportionalBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
        fn measure_text_family(&mut self, text: &str, _: f32, _: &str) -> f32 {
            text.chars()
                .map(|ch| match ch {
                    'i' => 2.0,
                    'W' => 10.0,
                    ch if !ch.is_ascii() => 11.0,
                    _ => 6.0,
                })
                .sum()
        }
    }

    #[test]
    fn exposes_text_input_access_role() {
        let widget = TextInputWidget::new(401, TextInputState::with_text("hello"));

        let node = widget.access_node();

        assert_eq!(node.role(), accesskit::Role::TextInput);
    }

    #[test]
    fn shared_geometry_tracks_clicks_and_caret() {
        let rect = Rect::xywh(20.0, 30.0, 180.0, 28.0);
        let mut state = TextInputState::with_text("abcd");
        state.set_caret(0, 0);
        let start = single_line_caret_rect(&state, rect, 11.0, 8.0);
        state.set_caret(3, 0);
        let after_three = single_line_caret_rect(&state, rect, 11.0, 8.0);
        assert!(after_three.origin.x > start.origin.x);

        let hit = single_line_byte_offset_at(
            &state,
            rect,
            Point2D::new(start.origin.x + 1.0, rect.origin.y + 10.0),
            11.0,
            8.0,
        );
        assert_eq!(hit, Some(0));
    }

    #[test]
    fn shared_geometry_keeps_overflowing_ascii_and_cjk_caret_inside_view() {
        let rect = Rect::xywh(20.0, 30.0, 72.0, 28.0);
        let mut state = TextInputState::with_text("long search query 中文封面");
        state.set_caret(state.text().len(), 0);
        let caret = single_line_caret_rect(&state, rect, 11.0, 8.0);
        assert!(caret.origin.x >= rect.origin.x + 8.0);
        assert!(caret.origin.x <= rect.origin.x + rect.size.x - 8.0 + 0.01);

        let left = single_line_byte_offset_at(
            &state,
            rect,
            Point2D::new(rect.origin.x + 8.0, rect.origin.y + 12.0),
            11.0,
            8.0,
        )
        .expect("left overflow hit");
        let right = single_line_byte_offset_at(
            &state,
            rect,
            Point2D::new(rect.origin.x + rect.size.x - 8.0, rect.origin.y + 12.0),
            11.0,
            8.0,
        )
        .expect("right overflow hit");
        assert!(left > 0, "horizontal scroll should hide the prefix");
        assert_eq!(right, state.text().len());
        assert!(state.text().is_char_boundary(left));
    }

    #[test]
    fn painted_geometry_uses_real_proportional_and_utf8_metrics() {
        let rect = Rect::xywh(20.0, 30.0, 200.0, 28.0);
        let mut state = TextInputState::with_text("iiiiWWWW中文");
        state.set_caret(0, 0);
        let mut backend = ProportionalBackend;
        let geometry = SingleLinePaintGeometry::measure(&state, rect, 11.0, 8.0, &mut backend);

        assert_eq!(
            geometry.byte_offset_at(&state, Point2D::new(36.0, 42.0), false),
            Some(4),
            "the click is after all four narrow i glyphs"
        );
        assert_eq!(
            geometry.byte_offset_at(&state, Point2D::new(87.0, 42.0), false),
            Some("iiiiWWWW中".len())
        );
    }

    #[test]
    fn painted_geometry_keeps_scrolled_caret_and_hits_inside_narrow_line() {
        let rect = Rect::xywh(20.0, 30.0, 40.0, 28.0);
        let mut state = TextInputState::with_text("iiiiWWWW中文");
        state.set_caret(state.text().len(), 0);
        let mut backend = ProportionalBackend;
        let geometry = SingleLinePaintGeometry::measure(&state, rect, 11.0, 8.0, &mut backend);
        let caret = geometry.caret_rect(&state).expect("matching caret");

        assert!((caret.origin.x - 52.0).abs() < 0.01);
        assert_eq!(
            geometry.byte_offset_at(&state, Point2D::new(28.0, 42.0), false),
            Some("iiiiWWWW".len())
        );
        assert_eq!(
            geometry.byte_offset_at(&state, Point2D::new(52.0, 42.0), false),
            Some(state.text().len())
        );
    }

    #[test]
    fn painted_geometry_rejects_stale_composition_caret() {
        let rect = Rect::xywh(20.0, 30.0, 200.0, 28.0);
        let mut state = TextInputState::with_text("ab");
        state.set_caret(1, 0);
        state.set_composition("中文", "中文".len(), 0);
        let mut backend = ProportionalBackend;
        let geometry = SingleLinePaintGeometry::measure(&state, rect, 11.0, 8.0, &mut backend);
        assert!(geometry.caret_rect(&state).is_some());

        state.set_composition("中文", 0, 1);
        assert!(geometry.caret_rect(&state).is_none());
    }

    #[test]
    fn multiline_hit_maps_the_first_visible_touch_row_to_its_source_line() {
        let rect = Rect::xywh(20.0, 30.0, 300.0, 104.0);
        let mut state = TextInputState::with_text("model-a\nmodel-b\nmodel-c\nmodel-d\nmodel-e");
        state.set_caret(state.text().len(), 0);

        let hit = multiline_byte_offset_at(
            &state,
            rect,
            Point2D::new(rect.origin.x + 12.0, rect.origin.y + 6.0),
            15.0,
            12.0,
            4,
        );

        assert_eq!(
            hit,
            Some("model-a\n".len()),
            "five source lines with a four-line touch viewport show model-b first"
        );
    }
}
