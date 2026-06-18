use super::WidgetHostNative;
use op_editor_core::{NodeId, PathAnchorMenuState};
use op_editor_ui::widgets::design_md_panel::DesignMdPanel;
use op_editor_ui::widgets::path_anchor_context_menu::PathAnchorContextMenu;
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

#[test]
fn topmost_design_panel_cursor_move_does_not_hover_path_anchor_menu_underneath() {
    let mut host = WidgetHostNative::new();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    let point = Point2D::new(160.0, 120.0);
    host.editor_state_mut().ui.path_anchor_menu = Some(PathAnchorMenuState {
        node_id: NodeId::new("n1"),
        anchor_index: 0,
        x: 120.0,
        y: 100.0,
        menu: Default::default(),
    });
    let menu = PathAnchorContextMenu::for_state(
        host.editor_state(),
        host.editor_state()
            .ui
            .path_anchor_menu
            .clone()
            .expect("menu open"),
    );
    assert!(
        menu.hovered_row_at(point).is_some(),
        "fixture point should hover the lower path-anchor menu"
    );

    host.editor_state_mut().editor_ui.design_md_panel_open = true;
    host.editor_state_mut().editor_ui.design_md_panel_pos = Some((110.0, 80.0));
    let panel_rect = host
        .design_md_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("design-md panel rect");
    assert!(
        panel_rect.contains(point),
        "topmost panel should cover the lower menu point"
    );
    let design_hover = DesignMdPanel::for_editor(host.editor_state())
        .and_then(|panel| panel.hover_at(panel_rect, point));
    host.editor_state_mut().editor_ui.design_md_hover = design_hover;

    let _ = host.apply_cursor_move(point.x, point.y);

    let menu = host
        .editor_state()
        .ui
        .path_anchor_menu
        .as_ref()
        .expect("menu still open");
    assert_eq!(
        menu.menu.hover, None,
        "hover must not pass through the topmost Design-MD panel"
    );
}

#[test]
fn topmost_design_panel_cursor_move_clears_stale_path_anchor_menu_hover() {
    let mut host = WidgetHostNative::new();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    let point = Point2D::new(160.0, 120.0);
    host.editor_state_mut().ui.path_anchor_menu = Some(PathAnchorMenuState {
        node_id: NodeId::new("n1"),
        anchor_index: 0,
        x: 120.0,
        y: 100.0,
        menu: Default::default(),
    });
    host.editor_state_mut()
        .ui
        .path_anchor_menu
        .as_mut()
        .expect("menu open")
        .menu
        .hover = Some(0);
    host.editor_state_mut().editor_ui.design_md_panel_open = true;
    host.editor_state_mut().editor_ui.design_md_panel_pos = Some((110.0, 80.0));
    let panel_rect = host
        .design_md_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("design-md panel rect");
    assert!(panel_rect.contains(point));
    host.editor_state_mut().editor_ui.design_md_hover =
        DesignMdPanel::for_editor(host.editor_state())
            .and_then(|panel| panel.hover_at(panel_rect, point));

    let _ = host.apply_cursor_move(point.x, point.y);

    let menu = host
        .editor_state()
        .ui
        .path_anchor_menu
        .as_ref()
        .expect("menu still open");
    assert_eq!(
        menu.menu.hover, None,
        "stale lower-menu hover should clear under the topmost panel"
    );
}
