//! In-canvas File-menu routing for host-owned template persistence.

use super::WidgetHostNative;
use op_editor_ui::widgets::file_menu::{FileMenu, FileMenuChoice};
use op_editor_ui::widgets::{TopBar, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

const VIEWPORT_W: f32 = 1200.0;

fn save_as_template_point(host: &WidgetHostNative) -> Point2D {
    let top_bar_rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VIEWPORT_W, TOP_BAR_HEIGHT),
    };
    let top_bar = TopBar::for_editor_ui(&host.editor_state().editor_ui);
    let anchor = top_bar.file_menu_rect_for(top_bar_rect);
    let menu = FileMenu::from_editor_ui(&host.editor_state().editor_ui, 0);
    let panel = menu.rect_at(anchor);
    let x = panel.origin.x + 20.0;
    let mut y = panel.origin.y;
    while y < panel.origin.y + panel.size.y {
        let point = Point2D::new(x, y);
        if menu.hit_test(panel, point) == Some(FileMenuChoice::SaveAsTemplate) {
            return point;
        }
        y += 1.0;
    }
    panic!("Save As Template row was not present");
}

#[test]
fn save_as_template_row_queues_the_host_request() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .scene_template_center
        .save_current_supported = true;
    host.editor_state_mut().editor_ui.file_menu_open = true;
    let point = save_as_template_point(&host);

    host.dispatch_file_menu_press(point.x, point.y, VIEWPORT_W);

    assert!(
        host.editor_state()
            .editor_ui
            .scene_template_center
            .pending_save_current
    );
    assert!(!host.editor_state().editor_ui.file_menu_open);
}
