use super::WidgetHost;
use op_editor_core::{EditorState, NodeId};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
struct CaptureBackend {
    fills: Vec<(Rect, Color)>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.fills.push((rect, color));
    }
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

fn caret_fill_count(host: &mut WidgetHost, now_ms: u64) -> usize {
    host.set_now_ms(now_ms);
    let mut backend = CaptureBackend::default();
    host.paint_editor(&mut backend, 1200.0, 800.0);
    backend
        .fills
        .iter()
        .filter(|(rect, color)| {
            (rect.size.x - 1.5).abs() < 0.01
                && rect.size.y > 12.0
                && rect.size.y < 20.0
                && *color == host.theme.foreground
        })
        .count()
}

#[test]
fn layer_rename_caret_uses_web_host_time() {
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::sample();
    host.mark_dirty();
    assert!(host.editor_state.start_rename_layer(NodeId::new("n10")));

    assert_eq!(caret_fill_count(&mut host, 0), 1);
    assert_eq!(caret_fill_count(&mut host, 750), 0);
}

#[test]
fn caret_animation_active_tracks_focused_text_input() {
    let mut host = WidgetHost::new();
    assert!(!host.caret_animation_active());

    host.editor_state.chat.focused = true;
    assert!(host.caret_animation_active());

    host.editor_state.chat.focused = false;
    assert!(!host.caret_animation_active());
}
