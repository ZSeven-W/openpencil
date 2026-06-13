use crate::widgets::{DesignMdPanel, PaintCx};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::{ButtonPressTarget, DesignMdButton, EditorState};

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

fn open_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.design_md_panel_open = true;
    state.doc.design_md = Some(op_editor_core::parse_design_md(
        "# Brief\n\n## Visual Theme\nWarm system",
    ));
    state
}

#[test]
fn for_editor_picks_up_pressed_design_md_button() {
    let mut state = open_state();
    state.editor_ui.pressed_button = Some(ButtonPressTarget::DesignMd(DesignMdButton::Import));

    let panel = DesignMdPanel::for_editor(&state).expect("open");

    assert_eq!(panel.pressed, Some(DesignMdButton::Import));
}

#[test]
fn pressed_section_header_paints_pressed_feedback() {
    let mut state = open_state();
    state.editor_ui.pressed_button = Some(ButtonPressTarget::DesignMd(
        DesignMdButton::ToggleSection(0),
    ));
    let panel = DesignMdPanel::for_editor(&state).expect("open");
    let rect = Rect::xywh(0.0, 0.0, 480.0, 560.0);
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
            .any(|(_, radius, color)| *radius == 7.0 && *color == expected),
        "pressed section header should paint shared pressed feedback"
    );
}
