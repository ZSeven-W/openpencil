//! TopBar Home (制图台) button press — queues the same FileAction as
//! File ▸ 制图台.

use super::WidgetHostNative;
use op_editor_core::editor_ui_state::FileAction;
use op_editor_ui::widgets::{TopBar, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

const VIEWPORT_W: f32 = 1400.0;
const VIEWPORT_H: f32 = 900.0;

fn top_bar_rect() -> Rect {
    Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VIEWPORT_W, TOP_BAR_HEIGHT),
    }
}

#[test]
fn pressing_home_button_queues_file_action_home() {
    let mut host = WidgetHostNative::new();
    // Stay on the canvas so the Home surface does not swallow the press.
    host.editor_state.editor_ui.home.visible = false;

    let bar = TopBar::for_editor_ui(&host.editor_state.editor_ui);
    let home = bar.home_button_rect(top_bar_rect());
    let point = Point2D::new(
        home.origin.x + home.size.x / 2.0,
        home.origin.y + home.size.y / 2.0,
    );

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state.editor_ui.pending_file_action,
        Some(FileAction::Home)
    );
}
