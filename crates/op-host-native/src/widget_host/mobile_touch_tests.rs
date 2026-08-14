use super::WidgetHostNative;
#[path = "mobile_auth_collab_tests.rs"]
mod mobile_auth_collab_tests;
#[path = "mobile_layer_drag_tests.rs"]
mod mobile_layer_drag_tests;
use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
use op_editor_core::{
    agent_settings::AgentSettingsTab,
    editor_ui_state::{EffectParamFocus, FileAction},
    AccountState, AssetCenterTab, AuthenticatedCollabSession, CollabAvailability,
    CollabConnectionPhase, CollabUiRole, EffectField, FontPickerPurpose, NodeId, PropertyFocus,
    Tool,
};
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
use op_editor_ui::widgets::{host_canvas_geometry, CollabPanel, MobileAppBar, MobileMoreEntry};
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
    let index = MobileMoreEntry::visible(host.editor_state())
        .into_iter()
        .position(|candidate| candidate == entry)
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
fn app_bar_fit_matches_canonical_fit_without_editing_state_at_touch_breakpoints() {
    for (class, width, height) in [
        (EditorSizeClass::Compact, 320.0, 568.0),
        (EditorSizeClass::Medium, 834.0, 1_112.0),
        (EditorSizeClass::Expanded, 1_194.0, 834.0),
    ] {
        let mut host = touch_host(class);
        host.editor_state_mut().viewport.zoom = 2.75;
        host.editor_state_mut().viewport.pan_x = -731.0;
        host.editor_state_mut().viewport.pan_y = 419.0;
        host.editor_state_mut().tool = Tool::Pen;
        host.editor_state_mut()
            .set_single_selection(NodeId::new("n10"));
        let snapshot = host.editor_state().snapshot_for_history();
        host.editor_state_mut().history_push_past(snapshot);

        let mut canonical = WidgetHostNative::new();
        *canonical.editor_state_mut() = host.editor_state().clone();
        canonical.fit_content_to_viewport(width, height);
        let expected_viewport = canonical.editor_state().viewport;

        let document_before = host.editor_state().doc.clone();
        let active_page_before = host.editor_state().ui.active_page_index;
        let selection_before = host.editor_state().selection.clone();
        let history_past_before = host.editor_state().history.past.clone();
        let history_future_before = host.editor_state().history.future.clone();
        let tool_before = host.editor_state().tool;
        let bar = host_canvas_geometry::touch_app_bar_rect(host.editor_state(), width);
        let point = center(MobileAppBar::fit_rect(bar));

        assert!(host.apply_press(point.x, point.y, width, height));
        assert_eq!(host.editor_state().viewport, expected_viewport);
        assert_eq!(host.editor_state().doc, document_before);
        assert_eq!(host.editor_state().ui.active_page_index, active_page_before);
        assert_eq!(host.editor_state().selection, selection_before);
        assert_eq!(host.editor_state().history.past, history_past_before);
        assert_eq!(host.editor_state().history.future, history_future_before);
        assert_eq!(host.editor_state().tool, tool_before);
    }
}

#[test]
fn open_touch_surface_blocks_the_app_bar_fit_target() {
    let mut host = touch_host(EditorSizeClass::Medium);
    let (width, height) = (834.0, 1_112.0);
    host.editor_state_mut().viewport.zoom = 2.75;
    host.editor_state_mut().viewport.pan_x = -731.0;
    host.editor_state_mut().viewport.pan_y = 419.0;
    host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::More);
    let before = host.editor_state().viewport;
    let bar = host_canvas_geometry::touch_app_bar_rect(host.editor_state(), width);
    let point = center(MobileAppBar::fit_rect(bar));

    assert!(host.apply_press(point.x, point.y, width, height));
    assert_eq!(host.editor_state().viewport, before);
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

    let panel = host.mobile_sheet_rect(width, height, MobileSheetKind::More);
    let panel_point = center(panel);
    let scrim_point = Point2D::new(200.0, 400.0);
    assert!(!panel.contains(scrim_point));

    assert!(host.apply_wheel(200.0, 400.0, -80.0, width, height));
    assert_eq!(host.editor_state().viewport, before);
    assert!(host.apply_pan_gesture(200.0, 400.0, 25.0, -30.0, width, height));
    assert_eq!(host.editor_state().viewport, before);
    for point in [panel_point, scrim_point] {
        assert!(host.apply_pinch_gesture(point.x, point.y, 120.0, width, height));
        assert_eq!(
            host.editor_state().viewport,
            before,
            "the open sheet and its scrim both own pinch input"
        );
    }
}

#[test]
fn closed_touch_surface_allows_canvas_pinch_zoom() {
    let mut host = touch_host(EditorSizeClass::Compact);
    let (width, height) = (390.0, 844.0);
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
    let (canvas_x, canvas_y, canvas_w, canvas_h) = host.canvas_region(width, height);
    let point = Point2D::new(canvas_x + canvas_w * 0.6, canvas_y + canvas_h * 0.4);
    let before = host.editor_state().viewport;

    assert!(host.apply_pinch_gesture(point.x, point.y, 120.0, width, height));

    let after = host.editor_state().viewport;
    assert!(after.zoom > before.zoom, "pinch must change canvas zoom");
    assert_ne!(after.pan_x, before.pan_x, "zoom anchor must update pan x");
    assert_ne!(after.pan_y, before.pan_y, "zoom anchor must update pan y");
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

#[derive(Clone, Copy)]
enum PropertyKeyboardOwner {
    Property,
    Effect,
    Font,
    Image,
}

fn focus_property_owner(host: &mut WidgetHostNative, owner: PropertyKeyboardOwner) {
    let state = host.editor_state_mut();
    match owner {
        PropertyKeyboardOwner::Property => {
            state.ui.property_focus = Some(PropertyFocus::PositionX);
            state.ui.property_input.set_text("123");
        }
        PropertyKeyboardOwner::Effect => {
            state.editor_ui.effect_param_focus = Some(EffectParamFocus {
                effect: 0,
                field: EffectField::Radius,
            });
            state.ui.property_input.set_text("12");
        }
        PropertyKeyboardOwner::Font => {
            state.editor_ui.font_picker.open = true;
            state.editor_ui.font_picker_purpose = Some(FontPickerPurpose::PropertyText);
        }
        PropertyKeyboardOwner::Image => state.editor_ui.image_panel.search_open = true,
    }
}

fn assert_property_owners_released(host: &WidgetHostNative) {
    let state = host.editor_state();
    assert!(state.ui.property_focus.is_none());
    assert!(state.editor_ui.effect_param_focus.is_none());
    assert!(!state.editor_ui.font_picker.open);
    assert_ne!(
        state.editor_ui.font_picker_purpose,
        Some(FontPickerPurpose::PropertyText)
    );
    assert!(!state.editor_ui.image_panel.search_open);
    assert!(!state.editor_ui.image_panel.generate_open);
}

#[test]
fn closing_properties_sheet_releases_every_property_keyboard_owner() {
    let (width, height) = (834.0, 1112.0);
    for owner in [
        PropertyKeyboardOwner::Property,
        PropertyKeyboardOwner::Effect,
        PropertyKeyboardOwner::Font,
        PropertyKeyboardOwner::Image,
    ] {
        let mut host = touch_host(EditorSizeClass::Medium);
        host.editor_state_mut()
            .set_single_selection(NodeId::new("n10"));
        host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::Properties);
        focus_property_owner(&mut host, owner);
        let sheet = host.mobile_sheet_rect(width, height, MobileSheetKind::Properties);
        let close = center(op_editor_ui::widgets::mobile_chrome::sheet_close_rect(
            sheet,
        ));

        assert!(host.apply_press(close.x, close.y, width, height));
        assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
        assert_property_owners_released(&host);
        assert!(!host.text_input_focus_active());
    }
}

#[test]
fn switching_properties_to_ai_routes_typing_to_chat() {
    let mut host = touch_host(EditorSizeClass::Medium);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::Properties);
    focus_property_owner(&mut host, PropertyKeyboardOwner::Property);
    host.editor_state_mut().editor_ui.effect_param_focus = Some(EffectParamFocus {
        effect: 0,
        field: EffectField::Radius,
    });
    host.editor_state_mut().editor_ui.font_picker.open = true;
    host.editor_state_mut().editor_ui.font_picker_purpose = Some(FontPickerPurpose::PropertyText);
    host.editor_state_mut().editor_ui.image_panel.search_open = true;

    host.toggle_mobile_sheet(MobileSheetKind::Ai);

    assert_eq!(
        host.editor_state().editor_ui.mobile_sheet,
        Some(MobileSheetKind::Ai)
    );
    assert_property_owners_released(&host);
    assert!(host.editor_state().chat.focused);
    assert!(host.apply_text('x'));
    assert_eq!(host.editor_state().chat.input.text(), "x");
}

#[test]
fn switching_properties_to_layers_releases_hidden_property_focus() {
    let mut host = touch_host(EditorSizeClass::Medium);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::Properties);
    focus_property_owner(&mut host, PropertyKeyboardOwner::Property);

    host.toggle_mobile_sheet(MobileSheetKind::Layers);

    assert_eq!(
        host.editor_state().editor_ui.mobile_sheet,
        Some(MobileSheetKind::Layers)
    );
    assert_property_owners_released(&host);
    assert!(!host.text_input_focus_active());
}

#[test]
fn expanded_property_to_ai_switch_releases_property_and_routes_chat() {
    let mut host = touch_host(EditorSizeClass::Expanded);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    focus_property_owner(&mut host, PropertyKeyboardOwner::Property);
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);

    host.toggle_mobile_sheet(MobileSheetKind::Ai);

    assert_property_owners_released(&host);
    assert!(host.editor_state().chat.focused);
    assert!(host.apply_text('z'));
    assert_eq!(host.editor_state().chat.input.text(), "z");
}

#[test]
fn closing_more_over_expanded_layout_keeps_visible_property_focus() {
    let mut host = touch_host(EditorSizeClass::Expanded);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::More);
    focus_property_owner(&mut host, PropertyKeyboardOwner::Property);

    host.dismiss_mobile_surface();

    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
    assert_eq!(
        host.editor_state().ui.property_focus,
        Some(PropertyFocus::PositionX)
    );
    assert!(host.text_input_focus_active());
}

const TOUCH_CANVAS_FIXTURE: &str = r#"{"version":"1.0.0","children":[
  {"type":"rectangle","id":"touch-rect","name":"Touch rect","x":120,"y":120,"width":80,"height":60}
]}"#;

const TOUCH_VIEWPORTS: [(EditorSizeClass, f32, f32); 3] = [
    (EditorSizeClass::Compact, 390.0, 844.0),
    (EditorSizeClass::Medium, 834.0, 1112.0),
    (EditorSizeClass::Expanded, 1194.0, 834.0),
];

fn touch_canvas_host(size_class: EditorSizeClass) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(TOUCH_CANVAS_FIXTURE)
        .expect("touch canvas fixture parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = true;
    ui.size_class = size_class;
    ui.sidebar_open = size_class.is_expanded();
    host.mark_paint_dirty_for_test();
    host
}

fn touch_canvas_screen_point(
    host: &WidgetHostNative,
    viewport_w: f32,
    viewport_h: f32,
    doc_x: f32,
    doc_y: f32,
) -> Point2D {
    let (canvas_x, canvas_y, _, _) = host.canvas_region(viewport_w, viewport_h);
    let viewport = host.editor_state().viewport;
    Point2D::new(
        canvas_x + viewport.pan_x + doc_x * viewport.zoom,
        canvas_y + viewport.pan_y + doc_y * viewport.zoom,
    )
}

#[test]
fn touch_blank_canvas_drag_pans_and_keeps_capture_for_every_size_class() {
    for (size_class, viewport_w, viewport_h) in TOUCH_VIEWPORTS {
        let mut host = touch_canvas_host(size_class);
        host.editor_state_mut()
            .set_single_selection(NodeId::new("touch-rect"));
        let start = touch_canvas_screen_point(&host, viewport_w, viewport_h, 20.0, 300.0);
        let before = host.editor_state().viewport;

        let _ = host.apply_press(start.x, start.y, viewport_w, viewport_h);
        assert!(
            host.touch_panel_gesture.is_some(),
            "{size_class:?} arms deferred canvas gesture"
        );
        assert!(host.drag.is_none(), "{size_class:?} does not use mouse pan");
        assert!(host.marquee_drag.is_none(), "{size_class:?} skips marquee");
        assert!(
            !host.editor_state().selection.is_empty(),
            "drag intent must not clear selection on down"
        );

        let tiny_move = Point2D::new(start.x + 0.75, start.y - 0.5);
        let _ = host.apply_cursor_move(tiny_move.x, tiny_move.y);
        let after_tiny_move = host.editor_state().viewport;
        assert_eq!(
            after_tiny_move, before,
            "{size_class:?} ignores sub-slop finger jitter"
        );

        // Pointer capture must keep the pan alive after the finger leaves the
        // canvas and even the viewport bounds.
        let outside = Point2D::new(-16.0, start.y + 24.0);
        assert!(host.apply_cursor_move(outside.x, outside.y));
        let after = host.editor_state().viewport;
        assert!((after.pan_x - before.pan_x - (outside.x - start.x)).abs() < 0.001);
        assert!((after.pan_y - before.pan_y - (outside.y - start.y)).abs() < 0.001);
        assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
        assert!(host.touch_panel_gesture.is_none());
        assert_eq!(
            host.editor_state().selection.anchor,
            NodeId::new("touch-rect"),
            "{size_class:?} pan preserves selection"
        );
    }
}

#[test]
fn touch_blank_canvas_tap_still_clears_selection_without_panning() {
    for (size_class, viewport_w, viewport_h) in TOUCH_VIEWPORTS {
        let mut host = touch_canvas_host(size_class);
        host.editor_state_mut()
            .set_single_selection(NodeId::new("touch-rect"));
        let point = touch_canvas_screen_point(&host, viewport_w, viewport_h, 20.0, 300.0);
        let before = host.editor_state().viewport;

        let _ = host.apply_press(point.x, point.y, viewport_w, viewport_h);
        assert!(
            !host.editor_state().selection.is_empty(),
            "selection clearing waits for a confirmed tap"
        );
        assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
        assert!(host.editor_state().selection.is_empty());
        assert_eq!(host.editor_state().viewport, before, "{size_class:?}");
    }
}

#[test]
fn touch_blank_canvas_down_breaks_stale_node_double_tap_continuity() {
    let (size_class, viewport_w, viewport_h) = TOUCH_VIEWPORTS[1];
    let mut host = touch_canvas_host(size_class);
    host.editor_state_mut().editor_ui.last_canvas_click = Some((NodeId::new("touch-rect"), 1_000));
    let point = touch_canvas_screen_point(&host, viewport_w, viewport_h, 20.0, 300.0);

    let _ = host.apply_press(point.x, point.y, viewport_w, viewport_h);

    assert!(host.editor_state().editor_ui.last_canvas_click.is_none());
}

#[test]
fn cancelling_a_pending_touch_canvas_gesture_preserves_selection_and_camera() {
    let (size_class, viewport_w, viewport_h) = TOUCH_VIEWPORTS[1];
    let mut host = touch_canvas_host(size_class);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("touch-rect"));
    let point = touch_canvas_screen_point(&host, viewport_w, viewport_h, 20.0, 300.0);
    let before = host.editor_state().viewport;

    let _ = host.apply_press(point.x, point.y, viewport_w, viewport_h);
    assert!(host.cancel_native_touch_gestures());

    assert!(host.touch_panel_gesture.is_none());
    assert_eq!(
        host.editor_state().selection.anchor,
        NodeId::new("touch-rect")
    );
    assert_eq!(host.editor_state().viewport, before);
    assert!(!host.apply_release_with_viewport(viewport_w, viewport_h));
    assert_eq!(
        host.editor_state().selection.anchor,
        NodeId::new("touch-rect"),
        "release after cancel must not replay the deferred blank tap"
    );
}

#[test]
fn two_finger_pan_cancels_pending_blank_tap_without_clearing_selection() {
    let (size_class, viewport_w, viewport_h) = TOUCH_VIEWPORTS[2];
    let mut host = touch_canvas_host(size_class);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("touch-rect"));
    let point = touch_canvas_screen_point(&host, viewport_w, viewport_h, 20.0, 300.0);
    let before = host.editor_state().viewport;

    let _ = host.apply_press(point.x, point.y, viewport_w, viewport_h);
    assert!(host.apply_pan_gesture(point.x, point.y, 16.0, -12.0, viewport_w, viewport_h));

    assert!(host.touch_panel_gesture.is_none());
    assert_eq!(
        host.editor_state().selection.anchor,
        NodeId::new("touch-rect")
    );
    let after = host.editor_state().viewport;
    assert!((after.pan_x - before.pan_x - 16.0).abs() < 0.001);
    assert!((after.pan_y - before.pan_y + 12.0).abs() < 0.001);
}

#[test]
fn touch_node_drag_moves_the_node_without_panning_for_every_size_class() {
    for (size_class, viewport_w, viewport_h) in TOUCH_VIEWPORTS {
        let mut host = touch_canvas_host(size_class);
        let point = touch_canvas_screen_point(&host, viewport_w, viewport_h, 150.0, 145.0);
        let viewport_before = host.editor_state().viewport;
        let bounds_before = op_editor_core::own_bounds(
            op_editor_core::walkers::find_node(
                host.editor_state().active_children(),
                &NodeId::new("touch-rect"),
            )
            .expect("fixture node exists"),
        );

        host.apply_press(point.x, point.y, viewport_w, viewport_h);
        assert!(host.node_drag.is_some(), "{size_class:?} arms node drag");
        assert!(
            host.drag.is_none(),
            "{size_class:?} does not arm canvas pan"
        );
        assert!(host.apply_cursor_move(point.x + 18.0, point.y + 14.0));
        assert!(host.apply_release_with_viewport(viewport_w, viewport_h));

        let bounds_after = op_editor_core::own_bounds(
            op_editor_core::walkers::find_node(
                host.editor_state().active_children(),
                &NodeId::new("touch-rect"),
            )
            .expect("dragged fixture node remains"),
        );
        assert_eq!(
            host.editor_state().viewport,
            viewport_before,
            "{size_class:?}"
        );
        assert!((bounds_after.x - bounds_before.x - 18.0).abs() < 0.001);
        assert!((bounds_after.y - bounds_before.y - 14.0).abs() < 0.001);
    }
}
