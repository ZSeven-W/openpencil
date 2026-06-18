use super::WidgetHost;
use op_editor_core::editor_ui_state::RecentFile;
use op_editor_core::Locale;
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
struct TextCaptureBackend {
    texts: Vec<String>,
}

impl RenderBackend for TextCaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
        self.texts
            .extend(layout.runs().iter().map(|run| run.content.clone()));
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

#[test]
fn file_menu_recent_age_uses_web_host_time() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.locale = Locale::EnUs;
    host.editor_state.editor_ui.file_menu_open = true;
    host.editor_state.editor_ui.recent_files = vec![RecentFile {
        path: "/tmp/demo.op".into(),
        modified_at: 6_400,
    }];
    host.set_wall_now_secs(10_000);

    let mut backend = TextCaptureBackend::default();
    host.paint_editor(&mut backend, 1200.0, 800.0);

    assert!(
        backend.texts.iter().any(|text| text == "1h ago"),
        "expected web FileMenu to use host now_ms for recent age, got {:?}",
        backend.texts
    );
}
