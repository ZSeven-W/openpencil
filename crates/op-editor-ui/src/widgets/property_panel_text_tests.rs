//! Text capture tests for `widgets::property_panel`.

use super::property_panel::PropertyPanel;
use super::property_panel_text_input::paint_text_input_view_value;
use crate::theme::Theme;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, TextLayout};
use jian_core::text_input::TextInputState;
use op_editor_core::{EditorState, Locale, PropertyFocus, PropertyTab};

#[derive(Default)]
struct TextCaptureBackend {
    texts: Vec<String>,
    origins: Vec<(String, Point2D)>,
}

#[derive(Default)]
struct RoundFillBackend {
    fills: Vec<(Rect, f32, Color)>,
}

impl crate::RenderBackend for TextCaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push(run.content.clone());
            self.origins.push((run.content.clone(), origin));
        }
    }
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

impl crate::RenderBackend for RoundFillBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.fills.push((rect, radius, color));
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

fn color_close(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 1e-6
        && (a.g - b.g).abs() < 1e-6
        && (a.b - b.b).abs() < 1e-6
        && (a.a - b.a).abs() < 1e-6
}

#[test]
fn focused_property_field_paints_text_input_state() {
    let mut state = EditorState::sample();
    state.ui.property_focus = Some(PropertyFocus::PositionX);
    state.ui.property_input.set_text("1234");
    state.ui.property_input.set_caret("1234".len(), 0);
    let panel = PropertyPanel::for_selection_at(&state, 0).expect("sample doc has a selection");
    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint(
        &mut cx,
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(280.0, 700.0),
        },
    );

    assert!(
        backend.texts.iter().any(|text| text == "1234"),
        "focused property field should paint from TextInputState"
    );
}

#[test]
fn focused_property_text_input_preserves_zero_horizontal_padding() {
    let input = TextInputState::with_text("1234");
    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    let rect = Rect {
        origin: Point2D::new(42.0, 10.0),
        size: Point2D::new(80.0, 28.0),
    };

    paint_text_input_view_value(&mut cx, &Theme::dark(), &input, rect, 12.0, 0.0, 24.0, 0);

    let (_, origin) = backend
        .origins
        .iter()
        .find(|(text, _)| text == "1234")
        .expect("input text should paint");
    assert!(
        (origin.x - rect.origin.x).abs() < 0.01,
        "zero-padding property input should paint at x={}, got {}",
        rect.origin.x,
        origin.x
    );
}

#[test]
fn code_tab_idle_body_uses_editor_locale() {
    let mut state = EditorState::sample();
    state.editor_ui.locale = Locale::ZhCn;
    state.editor_ui.property_tab = PropertyTab::Code;
    let panel = PropertyPanel::for_selection(&state).expect("sample doc has a selection");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(460.0, 700.0),
    };
    let mut backend = TextCaptureBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
    }
    let drawn = backend.texts.join("\n");
    assert!(drawn.contains("代码"));
    assert!(drawn.contains("1 个节点已选中"));
    assert!(drawn.contains("生成可用于生产的代码"));
    assert!(drawn.contains("生成 React"));
    assert!(drawn.contains("导出 AI Bundle"));
    assert!(!drawn.contains("1 node selected"));
    assert!(!drawn.contains("Generate production-ready code"));
}

#[test]
fn pressed_font_weight_picker_row_uses_shared_feedback() {
    let theme = Theme::dark();
    let panel_rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 700.0),
    };
    let visible = crate::widgets::property_panel_layout::VisibleSections {
        create_component: false,
        flex_layout: false,
        size_options: false,
        text: true,
        icon: false,
        ..crate::widgets::property_panel_layout::VisibleSections::ALL
    };
    let rows = crate::widgets::property_panel_text::font_weight_picker_action_rects(
        panel_rect.origin.x,
        crate::widgets::property_panel_text::text_section_top(panel_rect, visible).unwrap(),
        panel_rect.size.x - crate::widgets::property_panel_inputs::PAD_X * 2.0,
        visible.touch_controls,
    );
    let expected_row = rows[0].1;
    let expected = theme.button_hover.with_alpha(theme.button_hover.a * 1.8);
    let mut backend = RoundFillBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    crate::widgets::property_panel_text::paint_font_weight_picker(
        &mut cx,
        &theme,
        panel_rect,
        visible,
        Locale::EnUs,
        400,
        None,
        Some(0),
    );

    assert!(
        backend.fills.iter().any(|(fill, radius, color)| {
            *fill == expected_row && (*radius - 6.0).abs() < 0.01 && color_close(*color, expected)
        }),
        "pressed font-weight picker row should paint the shared pressed feedback token"
    );
}
