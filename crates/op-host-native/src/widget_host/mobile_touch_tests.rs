use super::WidgetHostNative;
use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
use op_editor_core::{
    editor_ui_state::FileAction, AssetCenterTab, AuthenticatedCollabSession, CollabConnectionPhase,
    CollabUiRole, NodeId,
};
use op_editor_ui::widgets::{host_canvas_geometry, MobileAppBar, MobileMoreEntry};
use op_editor_ui::{Point2D, Rect};

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn touch_host(size_class: EditorSizeClass) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = true;
    ui.size_class = size_class;
    ui.sidebar_open = size_class.is_expanded();
    host
}

fn press_more_entry(
    host: &mut WidgetHostNative,
    entry: MobileMoreEntry,
    width: f32,
    height: f32,
) -> bool {
    host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::More);
    let panel = host.mobile_sheet_rect(width, height, MobileSheetKind::More);
    let index = MobileMoreEntry::ALL
        .iter()
        .position(|candidate| *candidate == entry)
        .expect("entry belongs to the mobile More grid");
    let point = center(op_editor_ui::widgets::mobile_chrome::more_entry_rect(
        host.editor_state(),
        panel,
        index,
    ));
    host.apply_press(point.x, point.y, width, height)
}

#[test]
fn medium_layers_button_opens_bounded_side_surface() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    let bar = host_canvas_geometry::touch_app_bar_rect(host.editor_state(), width);
    let point = center(MobileAppBar::layers_rect(bar));

    assert!(host.apply_press(point.x, point.y, width, height));
    assert_eq!(
        host.editor_state().editor_ui.mobile_sheet,
        Some(MobileSheetKind::Layers)
    );
    let panel = host.mobile_sheet_rect(width, height, MobileSheetKind::Layers);
    assert!(panel.size.x < width / 2.0);
    assert!(panel.origin.y >= bar.size.y);
}

#[test]
fn expanded_layers_button_toggles_persistent_rail() {
    let mut host = touch_host(EditorSizeClass::Expanded);
    let (width, height) = (1194.0, 834.0);
    let bar = host_canvas_geometry::touch_app_bar_rect(host.editor_state(), width);
    let point = center(MobileAppBar::layers_rect(bar));

    assert!(host.editor_state().editor_ui.sidebar_open);
    assert!(host.apply_press(point.x, point.y, width, height));
    assert!(!host.editor_state().editor_ui.sidebar_open);
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
}

#[test]
fn outside_more_tap_closes_surface_without_reaching_canvas() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::More);
    let before = host.editor_state().viewport;

    assert!(host.apply_press(40.0, 300.0, width, height));
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
    assert_eq!(host.editor_state().viewport, before);
}

#[test]
fn open_touch_surface_blocks_canvas_zoom_and_pan() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::More);
    let before = host.editor_state().viewport;

    assert!(host.apply_wheel(200.0, 400.0, -80.0, width, height));
    assert_eq!(host.editor_state().viewport, before);
    assert!(host.apply_pan_gesture(200.0, 400.0, 25.0, -30.0, width, height));
    assert_eq!(host.editor_state().viewport, before);
}

#[test]
fn more_open_file_queues_the_shared_file_action_and_closes() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::More);
    let panel = host.mobile_sheet_rect(width, height, MobileSheetKind::More);
    let point = center(op_editor_ui::widgets::mobile_chrome::more_entry_rect(
        host.editor_state(),
        panel,
        0,
    ));

    assert!(host.apply_press(point.x, point.y, width, height));
    assert_eq!(
        host.editor_state().editor_ui.pending_file_action,
        Some(FileAction::Open)
    );
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
}

#[test]
fn more_open_file_does_not_queue_when_collaboration_blocks_replacement() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    assert!(host
        .editor_state_mut()
        .editor_ui
        .collab
        .set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Test session".to_string(),
                role: CollabUiRole::Viewer,
                share_endpoint: None,
            },
            Vec::new(),
        ));
    host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::More);
    let panel = host.mobile_sheet_rect(width, height, MobileSheetKind::More);
    let point = center(op_editor_ui::widgets::mobile_chrome::more_entry_rect(
        host.editor_state(),
        panel,
        0,
    ));

    assert!(host.apply_press(point.x, point.y, width, height));
    assert_eq!(host.editor_state().editor_ui.pending_file_action, None);
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
}

#[test]
fn more_templates_opens_the_asset_center_on_templates_and_closes_more() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    host.editor_state_mut().editor_ui.scene_template_center.tab = AssetCenterTab::Styles;
    host.editor_state_mut().chat.focused = true;

    assert!(press_more_entry(
        &mut host,
        MobileMoreEntry::Templates,
        width,
        height
    ));
    let state = host.editor_state();
    assert_eq!(state.editor_ui.mobile_sheet, None);
    assert!(state.editor_ui.scene_template_center.open);
    assert_eq!(
        state.editor_ui.scene_template_center.tab,
        AssetCenterTab::Templates
    );
    assert!(!state.chat.focused);
}

#[test]
fn more_assets_opens_the_asset_center_on_styles_and_closes_more() {
    let mut host = touch_host(EditorSizeClass::Compact);
    let (width, height) = (390.0, 844.0);
    host.editor_state_mut().editor_ui.scene_template_center.tab = AssetCenterTab::Templates;
    host.editor_state_mut().chat.focused = true;

    assert!(press_more_entry(
        &mut host,
        MobileMoreEntry::Assets,
        width,
        height
    ));
    let state = host.editor_state();
    assert_eq!(state.editor_ui.mobile_sheet, None);
    assert!(state.editor_ui.scene_template_center.open);
    assert_eq!(
        state.editor_ui.scene_template_center.tab,
        AssetCenterTab::Styles
    );
    assert!(!state.chat.focused);
}

#[test]
fn closed_ai_surface_has_no_hidden_hit_region() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    host.editor_state_mut().chat.collapsed = false;
    host.editor_state_mut().editor_ui.mobile_sheet = None;

    assert_eq!(host.ai_chat_rect(width, height), None);
}

#[test]
fn touch_variables_surface_owns_the_dock_overlap() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    let before = host.editor_state().tool;
    let dock = host_canvas_geometry::touch_dock_rect(host.editor_state(), width, height);
    let point = center(dock);

    assert!(host.apply_press(point.x, point.y, width, height));
    assert!(!host.editor_state().editor_ui.variables_panel_open);
    assert_eq!(host.editor_state().tool, before);
}

#[test]
fn selection_properties_action_opens_touch_inspector() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    let actions = op_editor_ui::widgets::mobile_chrome::selection_actions_rect_for(
        host.editor_state(),
        width,
        height,
    );
    let point = Point2D::new(actions.origin.x + 36.0, actions.origin.y + 22.0);

    assert!(host.apply_press(point.x, point.y, width, height));
    assert_eq!(
        host.editor_state().editor_ui.mobile_sheet,
        Some(MobileSheetKind::Properties)
    );
}

#[test]
fn selection_delete_action_records_undo_history() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1112.0);
    let selected = NodeId::new("n10");
    host.editor_state_mut()
        .set_single_selection(selected.clone());
    assert!(
        op_editor_core::walkers::find_node(host.editor_state().active_children(), &selected,)
            .is_some()
    );
    let actions = op_editor_ui::widgets::mobile_chrome::selection_actions_rect_for(
        host.editor_state(),
        width,
        height,
    );
    let point = Point2D::new(
        actions.origin.x + actions.size.x - 22.0,
        actions.origin.y + 22.0,
    );

    assert!(host.apply_press(point.x, point.y, width, height));
    assert!(
        op_editor_core::walkers::find_node(host.editor_state().active_children(), &selected,)
            .is_none()
    );
    assert!(host.editor_state().history.can_undo());
    assert!(host.apply_undo());
    assert!(
        op_editor_core::walkers::find_node(host.editor_state().active_children(), &selected,)
            .is_some()
    );
}
