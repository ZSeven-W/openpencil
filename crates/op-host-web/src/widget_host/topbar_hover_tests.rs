use super::WidgetHost;
use op_editor_ui::widgets::{TopBar, TopBarHit, TOP_BAR_HEIGHT};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

#[test]
fn web_cursor_move_ignores_topbar_traffic_cluster() {
    let mut host = WidgetHost::new();
    host.last_viewport_w = 1440.0;
    host.last_viewport_h = 900.0;
    let topbar = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(host.last_viewport_w, TOP_BAR_HEIGHT),
    };
    let traffic = TopBar::traffic_cluster_rect(topbar);
    let over = Point2D::new(
        traffic.origin.x + traffic.size.x / 2.0,
        traffic.origin.y + traffic.size.y / 2.0,
    );

    host.apply_cursor_move(over.x, over.y);

    assert!(!host.editor_state.editor_ui.topbar_traffic_hover);
}

#[test]
fn web_topbar_sidebar_button_has_no_traffic_gap() {
    let mut host = WidgetHost::new();
    host.last_viewport_w = 1440.0;
    host.last_viewport_h = 900.0;
    assert!(host.editor_state.editor_ui.sidebar_open);

    assert!(host.apply_press(26.0, TOP_BAR_HEIGHT / 2.0, 1440.0, 900.0));

    assert!(!host.editor_state.editor_ui.sidebar_open);
}

#[test]
fn web_topbar_preview_button_toggles_preview_mode_like_native() {
    let mut host = WidgetHost::new();
    host.last_viewport_w = 1440.0;
    host.last_viewport_h = 900.0;
    assert!(!host.editor_state.editor_ui.preview_mode);
    // The canvas Preview (Play) button is gated behind the experimental-features
    // opt-in (Settings → System); it is hidden by default, so enable the flag
    // before probing for its hit target.
    host.editor_state
        .editor_ui
        .agent_settings
        .experimental_features_enabled = true;

    let topbar_rect = host.top_bar_rect(host.last_viewport_w);
    let topbar = host.top_bar();
    let mut point = None;
    let mut x = topbar_rect.origin.x;
    while x < topbar_rect.origin.x + topbar_rect.size.x {
        let p = Point2D::new(x, TOP_BAR_HEIGHT / 2.0);
        if topbar.hit_test(topbar_rect, p) == Some(TopBarHit::TogglePreview) {
            point = Some(p);
            break;
        }
        x += 1.0;
    }
    let point = point.expect("preview button point");

    assert!(host.apply_press(point.x, point.y, 1440.0, 900.0));

    assert!(host.editor_state.editor_ui.preview_mode);

    assert!(host.apply_press(point.x, point.y, 1440.0, 900.0));

    assert!(!host.editor_state.editor_ui.preview_mode);
}

#[derive(Default)]
struct CaptureBackend {
    ovals: Vec<Rect>,
}

impl RenderBackend for CaptureBackend {
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
    fn fill_oval(&mut self, rect: Rect, _: Color) {
        self.ovals.push(rect);
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

#[test]
fn web_topbar_does_not_paint_traffic_dots() {
    let mut host = WidgetHost::new();
    let mut backend = CaptureBackend::default();

    host.paint_editor(&mut backend, 1440.0, 900.0);

    let left_topbar_ovals = backend
        .ovals
        .iter()
        .filter(|r| r.origin.x < 80.0 && r.origin.y < TOP_BAR_HEIGHT)
        .count();
    assert_eq!(left_topbar_ovals, 0);
}
