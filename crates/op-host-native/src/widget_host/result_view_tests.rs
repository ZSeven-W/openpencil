//! Native-host wiring tests for the post-generation 成品视图.
//!
//! Windows-gated like the other tests that solve layout: building the
//! scene runs `jian_skia::SkiaMeasure`, which aborts the process under
//! Windows CI's DirectWrite.

#![cfg(all(test, not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::{EditorState, EntrySurface, HomeFamily, NodeId};
use op_editor_ui::widgets::ResultViewSurface;
use op_editor_ui::Point2D;
use std::sync::{LazyLock, Mutex, MutexGuard};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

const VW: f32 = 1440.0;
const VH: f32 = 900.0;
/// Past every entrance window, so hit rects and draw rects coincide.
const NOW: u64 = 90_000;

/// Two phone screens — the shape a Home App-UI generation produces.
const TWO_PHONES: &str = r##"{
    "version": "1.0.0",
    "children": [
        { "type": "frame", "id": "screen-1", "name": "首页", "x": 0, "y": 0,
          "width": 375, "height": 812,
          "fill": [{"type":"solid","color":"#ffffff"}], "children": [] },
        { "type": "frame", "id": "screen-2", "name": "预约", "x": 500, "y": 0,
          "width": 375, "height": 812,
          "fill": [{"type":"solid","color":"#eeeeee"}], "children": [] }
    ]
}"##;

fn host_with_result_view() -> WidgetHostNative {
    let document = jian_ops_schema::load_str(TWO_PHONES)
        .expect("parse phone fixture")
        .value;
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(document));
    host.set_now_ms(NOW);
    host.editor_state
        .editor_ui
        .result_view
        .open(vec!["screen-1".into(), "screen-2".into()], NOW - 1_000);
    host.editor_state.editor_ui.result_view.family = Some(HomeFamily::AppUi);
    host.editor_state.editor_ui.result_view.brief = "取餐预约".into();
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host
}

fn rect_center(rect: op_editor_ui::Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn button_point(host: &WidgetHostNative, index: usize) -> Point2D {
    let surface = ResultViewSurface::for_editor_at(&host.editor_state, host.now_ms())
        .expect("result view is visible");
    let layout = surface.layout(VW, VH);
    rect_center(layout.buttons[index])
}

#[test]
fn home_send_arms_the_result_view_with_family_and_brief() {
    let _guard = test_lock();
    let mut host = WidgetHostNative::new();
    host.editor_state.editor_ui.home.visible = true;
    host.editor_state.editor_ui.home.bind(HomeFamily::AppUi);
    for character in "取餐预约".chars() {
        assert!(host.apply_text(character));
    }
    assert!(host.apply_send());
    let view = &host.editor_state.editor_ui.result_view;
    assert!(view.awaiting_generation);
    assert!(!view.visible, "arming must not take over mid-generation");
    assert_eq!(view.family, Some(HomeFamily::AppUi));
    assert_eq!(view.brief, "取餐预约");
}

#[test]
fn edit_this_screen_hides_selects_and_preps_the_composer() {
    let _guard = test_lock();
    let mut host = host_with_result_view();
    let point = button_point(&host, 0);
    assert!(host.apply_press(point.x, point.y, VW, VH));
    assert!(!host.result_view_visible());
    assert_eq!(
        host.editor_state.selection.set,
        vec![NodeId::new("screen-1".to_string())],
        "改这一页 must select the chosen board"
    );
    assert_eq!(host.editor_state.chat.input.text(), "改这一页：");
    assert!(host.editor_state.chat.focused);
    assert!(host.editor_state.chat.pending_send.is_none());
}

#[test]
fn restyle_hides_rearms_and_sends_the_turn() {
    let _guard = test_lock();
    let mut host = host_with_result_view();
    let point = button_point(&host, 3);
    assert!(host.apply_press(point.x, point.y, VW, VH));
    assert!(!host.result_view_visible());
    let view = &host.editor_state.editor_ui.result_view;
    assert!(view.awaiting_generation, "the view reopens after the turn");
    assert_eq!(
        host.editor_state.chat.pending_send.as_deref(),
        Some("换一种视觉风格，保持结构、内容和交互不变")
    );
}

#[test]
fn escape_hides_the_view_back_to_the_canvas() {
    let _guard = test_lock();
    let mut host = host_with_result_view();
    assert!(host.apply_escape());
    assert!(!host.result_view_visible());
}

#[test]
fn pressing_empty_stage_paper_is_swallowed_by_the_takeover() {
    let _guard = test_lock();
    let mut host = host_with_result_view();
    host.editor_state
        .set_single_selection(NodeId::new("screen-1".to_string()));
    let selection = host.editor_state.selection.set.clone();
    let surface = ResultViewSurface::for_editor_at(&host.editor_state, host.now_ms()).unwrap();
    let stage = surface.layout(VW, VH).stage;
    let paper = Point2D::new(stage.origin.x + 6.0, stage.origin.y + 6.0);
    assert!(host.apply_press(paper.x, paper.y, VW, VH));
    assert!(host.result_view_visible(), "the takeover stays up");
    assert_eq!(
        host.editor_state.selection.set, selection,
        "a swallowed press never reaches the canvas selection"
    );
}

#[test]
fn pressing_a_board_selects_it_and_back_returns_home() {
    let _guard = test_lock();
    let mut host = host_with_result_view();
    let surface = ResultViewSurface::for_editor_at(&host.editor_state, host.now_ms()).unwrap();
    let layout = surface.layout(VW, VH);
    let second = rect_center(layout.screens[1]);
    assert!(host.apply_press(second.x, second.y, VW, VH));
    assert_eq!(host.editor_state.editor_ui.result_view.selected, 1);
    assert!(host.result_view_visible());

    let back = rect_center(layout.breadcrumb_back);
    assert!(host.apply_press(back.x, back.y, VW, VH));
    assert!(!host.result_view_visible());
    assert!(host.editor_state.editor_ui.home.visible);
    assert_eq!(
        host.editor_state.editor_ui.entry_surface,
        EntrySurface::Home
    );
}

#[test]
fn play_and_components_and_export_route_to_their_surfaces() {
    let _guard = test_lock();
    let mut host = host_with_result_view();

    let play = button_point(&host, 1);
    assert!(host.apply_press(play.x, play.y, VW, VH));
    assert!(!host.result_view_visible());
    assert!(host.preview_active(), "试点原型 enters the preview");

    host.apply_escape(); // leave preview, back to canvas
    host.editor_state.editor_ui.result_view.open(
        vec!["screen-1".into(), "screen-2".into()],
        host.now_ms().saturating_sub(2_000),
    );
    let components = button_point(&host, 2);
    assert!(host.apply_press(components.x, components.y, VW, VH));
    assert!(!host.result_view_visible());
    assert!(host.editor_state.editor_ui.variables_panel_open);

    host.editor_state.editor_ui.result_view.open(
        vec!["screen-1".into(), "screen-2".into()],
        host.now_ms().saturating_sub(2_000),
    );
    let export = button_point(&host, 4);
    assert!(host.apply_press(export.x, export.y, VW, VH));
    assert!(!host.result_view_visible());
    assert!(host.editor_state.editor_ui.export_dialog_open);
}
