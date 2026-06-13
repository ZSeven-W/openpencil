use super::property_panel::PropertyPanel;
use super::property_panel_sections as sections;
use super::property_panel_test_support::visible_for;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::{ButtonPressTarget, EditorState};

#[derive(Default)]
struct CaptureBackend {
    round_fills: Vec<(Rect, f32, Color)>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}

    fn end_frame(&mut self) {}

    fn fill_rect(&mut self, _rect: Rect, _color: Color) {}

    fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {}

    fn draw_text(&mut self, _layout: &TextLayout, _origin: Point2D) {}

    fn clip_rect(&mut self, _rect: Rect) {}

    fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {}

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.round_fills.push((rect, radius, color));
    }

    fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {}

    fn stroke_svg_path(
        &mut self,
        _d: &str,
        _top_left: Point2D,
        _size: f32,
        _color: Color,
        _width: f32,
    ) {
    }

    fn save(&mut self) {}

    fn restore(&mut self) {}

    fn translate(&mut self, _offset: Point2D) {}

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

#[test]
fn property_action_pressed_uses_shared_feedback() {
    let mut state = EditorState::sample();
    state.editor_ui.pressed_button = Some(ButtonPressTarget::PropertyPanel(0));
    let panel = PropertyPanel::for_selection(&state).expect("sample doc has a selection");
    assert_eq!(panel.action_pressed, Some(0));

    let rect = Rect::xywh(0.0, 0.0, 280.0, 900.0);
    let action_rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&panel),
        &panel.snapshot.effects,
        panel.fill_type_picker.open,
        panel.font_picker.open,
        panel.font_weight_picker_open,
        panel.export_scale_picker_open,
        panel.export_format_picker_open,
        panel.padding_mode_popover_open,
    );
    assert!(
        !action_rects.is_empty(),
        "sample panel should expose actions"
    );

    let expected = panel
        .theme
        .button_hover
        .with_alpha(panel.theme.button_hover.a * 1.8);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(_, radius, color)| *radius == 6.0 && *color == expected),
        "pressed property action should paint shared pressed feedback"
    );
}
