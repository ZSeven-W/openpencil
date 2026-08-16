//! Focused touch-host regressions for presentation chrome.

use super::super::WidgetHostNative;
use op_editor_core::preview_slideshow::SlideshowToolbarButton;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{EditorState, LoginFlowStatus};
use op_editor_ui::widgets::{MobileAppBar, SlideshowToolbar};
use op_editor_ui::Point2D;

const DECK: &str = r#"{"version":"1.0.0","children":[
    {"type":"frame","id":"one","x":0,"y":0,"width":1920,"height":1080},
    {"type":"frame","id":"two","x":2100,"y":0,"width":1920,"height":1080}
]}"#;

fn presenting_touch_host(class: EditorSizeClass, width: f32, height: f32) -> WidgetHostNative {
    let document = jian_ops_schema::load_str(DECK).expect("parse deck").value;
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = Some(TemplateScene::Slides);
    let mut host = WidgetHostNative::new();
    host.install_imported_state(state);
    host.editor_state.editor_ui.touch = true;
    host.editor_state.editor_ui.size_class = class;
    host.editor_state.editor_ui.sidebar_open = class.is_expanded();
    host.last_viewport_w = width;
    host.last_viewport_h = height;
    host.enter_preview((width, height));
    assert!(host.preview_slideshow_active());
    host
}

fn exit_point(host: &WidgetHostNative, width: f32, height: f32) -> Point2D {
    let stage = host.preview_canvas_rect(width, height);
    let label = host
        .editor_state
        .preview_slideshow()
        .expect("presenting")
        .counter_label();
    let exit = SlideshowToolbar::button_rects(stage, &label)
        .into_iter()
        .find(|(button, _)| *button == SlideshowToolbarButton::Exit)
        .expect("exit button")
        .1;
    Point2D::new(
        exit.origin.x + exit.size.x / 2.0,
        exit.origin.y + exit.size.y / 2.0,
    )
}

#[test]
fn touch_presentations_use_the_complete_safe_local_stage() {
    for (class, width, height) in [
        (EditorSizeClass::Compact, 390.0, 763.0),
        (EditorSizeClass::Medium, 834.0, 1_078.0),
        (EditorSizeClass::Expanded, 1_194.0, 800.0),
    ] {
        let host = presenting_touch_host(class, width, height);
        let stage = host.preview_canvas_rect(width, height);
        assert_eq!(stage.origin, Point2D::new(0.0, 0.0), "class {class:?}");
        assert_eq!(stage.size, Point2D::new(width, height), "class {class:?}");
    }
}

#[test]
fn entering_a_touch_deck_releases_account_and_collaboration_overlays() {
    let (width, height) = (390.0, 763.0);
    let document = jian_ops_schema::load_str(DECK).expect("parse deck").value;
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = Some(TemplateScene::Slides);
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.mobile_sheet = Some(op_editor_core::size_class::MobileSheetKind::More);
    state.editor_ui.login_modal_open = true;
    state.editor_ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser);
    state.editor_ui.account_menu_open = true;
    state.editor_ui.agent_settings_open = true;
    state.editor_ui.collab.panel.open = true;
    state.editor_ui.collab.panel.join_address_focused = true;

    let mut host = WidgetHostNative::new();
    host.install_imported_state(state);
    // Import intentionally preserves the live shell chrome instead of the
    // incoming state's platform flags. Model the mobile shell after install
    // so deck entry exercises the touch-only overlay teardown path.
    host.editor_state.editor_ui.touch = true;
    host.editor_state.editor_ui.size_class = EditorSizeClass::Compact;
    host.auth_login_handle = Some(7);
    host.last_viewport_w = width;
    host.last_viewport_h = height;
    assert!(host.enter_preview((width, height)));

    let ui = &host.editor_state.editor_ui;
    assert_eq!(ui.mobile_sheet, None);
    assert!(!ui.login_modal_open);
    assert_eq!(ui.login_modal_status, None);
    assert!(!ui.account_menu_open);
    assert!(!ui.agent_settings_open);
    assert!(!ui.collab.panel.open);
    assert!(!ui.collab.panel.join_address_focused);
    assert_eq!(host.auth_login_handle, None);

    assert!(host.apply_press(width / 2.0, height / 2.0, width, height));
    assert!(
        host.slideshow_press_screen.is_some(),
        "the deck owns the press"
    );
}

#[test]
fn stationary_touch_tap_on_exit_needs_no_hover_move() {
    let (width, height) = (390.0, 763.0);
    let mut host = presenting_touch_host(EditorSizeClass::Compact, width, height);
    let point = exit_point(&host, width, height);

    assert!(host.apply_press(point.x, point.y, width, height));
    assert!(host.apply_release_with_viewport(width, height));

    assert!(!host.preview_slideshow_active());
    assert!(!host.editor_state.editor_ui.preview.mode);
}

#[test]
fn dragging_off_exit_still_cancels_the_action() {
    let (width, height) = (390.0, 763.0);
    let mut host = presenting_touch_host(EditorSizeClass::Compact, width, height);
    let point = exit_point(&host, width, height);

    assert!(host.apply_press(point.x, point.y, width, height));
    assert!(host.preview_slideshow_release_point(point.x - 100.0, point.y - 100.0, width, height,));
    assert!(host.apply_release_with_viewport(width, height));

    assert!(host.preview_slideshow_active());
}

#[test]
fn hidden_touch_app_bar_cannot_claim_undo_while_presenting() {
    let (width, height) = (390.0, 763.0);
    let mut host = presenting_touch_host(EditorSizeClass::Compact, width, height);
    let history_snapshot = host.editor_state.snapshot_for_history();
    host.editor_state.history_push_past(history_snapshot);
    let history_depth = host.editor_state.history.past.len();
    let document_before = host.editor_state.doc.clone();
    let app_bar =
        op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_rect(&host.editor_state, width);
    let undo = MobileAppBar::undo_rect(app_bar);
    let point = Point2D::new(
        undo.origin.x + undo.size.x / 2.0,
        undo.origin.y + undo.size.y / 2.0,
    );

    assert!(host.apply_press(point.x, point.y, width, height));
    assert_eq!(host.editor_state.doc, document_before, "Undo never ran");
    assert_eq!(host.editor_state.history.past.len(), history_depth);
    assert_eq!(host.editor_state.editor_ui.mobile_sheet, None);
    assert!(host
        .editor_state
        .editor_ui
        .preview
        .toolbar_pressed
        .is_none());
    assert!(
        host.slideshow_press_screen.is_some(),
        "the deck owns the press"
    );
}

#[test]
fn hidden_touch_app_bar_cannot_claim_fit_while_presenting() {
    let (width, height) = (390.0, 763.0);
    let mut host = presenting_touch_host(EditorSizeClass::Compact, width, height);
    host.editor_state.viewport.zoom = 2.75;
    host.editor_state.viewport.pan_x = -731.0;
    host.editor_state.viewport.pan_y = 419.0;
    let viewport_before = host.editor_state.viewport;
    let app_bar =
        op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_rect(&host.editor_state, width);
    let fit = MobileAppBar::fit_rect(app_bar);
    let point = Point2D::new(
        fit.origin.x + fit.size.x / 2.0,
        fit.origin.y + fit.size.y / 2.0,
    );

    assert!(host.apply_press(point.x, point.y, width, height));
    assert_eq!(host.editor_state.viewport, viewport_before, "Fit never ran");
    assert!(host
        .editor_state
        .editor_ui
        .preview
        .toolbar_pressed
        .is_none());
    assert!(
        host.slideshow_press_screen.is_some(),
        "the deck owns the press"
    );
}
