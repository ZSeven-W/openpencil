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

    host.editor_state_mut().editor_ui.design_md_panel.open = true;
    host.editor_state_mut().editor_ui.design_md_panel.pos = Some((110.0, 80.0));
    let panel_rect = host
        .design_md_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("design-md panel rect");
    assert!(
        panel_rect.contains(point),
        "topmost panel should cover the lower menu point"
    );
    let design_hover = DesignMdPanel::for_editor(host.editor_state())
        .and_then(|panel| panel.hover_at(panel_rect, point));
    host.editor_state_mut().editor_ui.design_md_panel.hover = design_hover;

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
    host.editor_state_mut().editor_ui.design_md_panel.open = true;
    host.editor_state_mut().editor_ui.design_md_panel.pos = Some((110.0, 80.0));
    let panel_rect = host
        .design_md_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("design-md panel rect");
    assert!(panel_rect.contains(point));
    host.editor_state_mut().editor_ui.design_md_panel.hover =
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

#[test]
fn multi_selection_layer_context_menu_tracks_the_hovered_row() {
    use op_editor_core::editor_ui_state::{LayerContextMenuState, LayerContextTarget};
    use op_editor_core::EditorState;
    use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;

    let mut host = WidgetHostNative::new();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    *host.editor_state_mut() = EditorState::sample();
    host.editor_state_mut().toggle_selection(NodeId::new("n13"));
    assert_eq!(host.editor_state().selection_count(), 2);
    host.editor_state_mut().editor_ui.layer_context_menu = Some(LayerContextMenuState {
        target: LayerContextTarget::Layer(NodeId::new("n11")),
        anchor_x: 220.0,
        anchor_y: 120.0,
        menu: Default::default(),
    });

    let menu_state = host
        .editor_state()
        .editor_ui
        .layer_context_menu
        .clone()
        .expect("menu open");
    let menu = LayerContextMenu::for_state(host.editor_state(), menu_state);
    let point = Point2D::new(menu.rect().origin.x + 40.0, menu.rect().origin.y + 22.0);
    assert_eq!(menu.hovered_row_at(point), Some(0));

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .layer_context_menu
            .as_ref()
            .expect("menu stays open")
            .menu
            .hover,
        Some(0),
        "the layer menu must preserve the row hover for a multi-selection"
    );
}

/// The colour-variable popup used to have no hover state at all, so a
/// cursor over its rows highlighted whatever inspector control sat
/// underneath. The popup now owns every point on its chrome: its own row
/// lights up and the rail's stale wash is dropped.
#[test]
fn color_variable_popup_owns_hover_instead_of_the_rail_underneath() {
    use jian_ops_schema::variable::{VariableKind, VariableScalar};
    use op_editor_core::{ColorTarget, EditorState};
    use op_editor_ui::widgets::{press_flow, PropertyPanel};

    let mut host = WidgetHostNative::new();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    *host.editor_state_mut() = EditorState::sample();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n13"));
    for i in 0..6 {
        assert!(host.editor_state_mut().create_variable(
            &format!("--card-{i:02}"),
            VariableKind::Color,
            VariableScalar::Str("#DBD8CB".into()),
        ));
    }
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.property_color_variable_picker_open = Some(ColorTarget::Fill);
    // A wash left over from the inspector row the popup now covers.
    ui.property_action_hover = Some(0);

    let rect = press_flow::property_panel_rect(host.editor_state(), VIEWPORT_W, VIEWPORT_H);
    let layout = PropertyPanel::for_selection(host.editor_state())
        .expect("rectangle panel")
        .color_variable_picker_layout(rect)
        .expect("open picker lays out");
    let row = layout.rows[1].1;
    let point = Point2D::new(
        row.origin.x + row.size.x / 2.0,
        row.origin.y + row.size.y / 2.0,
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .property_color_variable_picker_hover,
        Some(1),
        "the row under the cursor must light up"
    );
    assert_eq!(
        host.editor_state().editor_ui.property_action_hover,
        None,
        "hover must not pass through the popup to the rail underneath"
    );
}
