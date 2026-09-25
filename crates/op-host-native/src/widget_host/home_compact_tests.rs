//! Host-level tests for the compact (phone) Studio Home: the 普通 /
//! 专业 switch on ONE EditorState, the canvas-side way back, the bottom
//! nav, and the one-finger page scroll.

use super::WidgetHostNative;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{EntrySurface, FileAction, HomeFamily, NodeId};
use op_editor_ui::widgets::host_canvas_geometry;
use op_editor_ui::widgets::{HomeSurface, MobileAppBar};
use op_editor_ui::Point2D;

const W: f32 = 390.0;
const H: f32 = 844.0;

fn compact_host() -> WidgetHostNative {
    compact_host_with(|_| {})
}

/// A compact phone host over the starter document (the n10 frame).
fn compact_host_with(configure: impl FnOnce(&mut op_editor_core::EditorState)) -> WidgetHostNative {
    let mut state = op_editor_core::EditorState::starter();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.home.visible = true;
    configure(&mut state);
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host
}

fn center(rect: op_editor_ui::Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn compact_layout(host: &WidgetHostNative) -> op_editor_ui::widgets::HomeLayout {
    let home = HomeSurface::for_editor(host.editor_state()).expect("home visible");
    home.layout(W, H)
}

/// The starter frame's doc x, wherever the page migration put it
/// (`add_page` moves root children into "Page 1").
fn starter_frame_x(host: &WidgetHostNative) -> f64 {
    fn in_children(children: &[jian_ops_schema::node::PenNode]) -> Option<f64> {
        children.iter().find_map(|node| match node {
            jian_ops_schema::node::PenNode::Frame(frame) if frame.base.id == "n10" => {
                Some(frame.base.x.unwrap_or(0.0))
            }
            _ => None,
        })
    }
    let state = host.editor_state();
    state
        .doc
        .pages
        .as_ref()
        .and_then(|pages| pages.iter().find_map(|page| in_children(&page.children)))
        .or_else(|| in_children(&state.doc.children))
        .expect("starter frame n10")
}

#[test]
fn compact_professional_segment_hides_home_and_sets_canvas_preference() {
    let mut host = compact_host();
    let rect = compact_layout(&host).professional;
    assert!(host.apply_press(rect.origin.x + 4.0, rect.origin.y + 4.0, W, H));
    assert!(!host.home_visible());
    assert_eq!(
        host.editor_state().editor_ui.entry_surface,
        EntrySurface::Canvas
    );
}

#[test]
fn compact_mode_round_trip_preserves_document_selection_and_undo() {
    let mut host = compact_host();
    // 普通 → 专业: the compact segmented control's 专业 half.
    let professional = compact_layout(&host).professional;
    assert!(host.apply_press(
        professional.origin.x + 4.0,
        professional.origin.y + 4.0,
        W,
        H
    ));
    assert!(!host.home_visible());

    // Edit in professional mode on the REAL document: move the starter
    // frame with a committed history entry underneath.
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().commit_history();
    assert!(host.editor_state_mut().translate_selected(30.0, 0.0));
    let undo_depth = host.editor_state().history.past.len();
    assert!(undo_depth >= 1, "the edit must sit on the undo stack");
    assert_eq!(starter_frame_x(&host), 30.0);
    // A second page, selected — page selection must ride through too.
    host.editor_state_mut().add_page();
    host.editor_state_mut().ui.active_page_index = 1;
    let page_count = host
        .editor_state()
        .doc
        .pages
        .as_ref()
        .map_or(1, |p| p.len());

    // 专业 → 普通: the compact app bar's Home target on the touch chrome.
    let app_bar = host_canvas_geometry::touch_app_bar_rect(host.editor_state(), W);
    let home_button = center(MobileAppBar::home_rect(app_bar));
    assert!(host.apply_press(home_button.x, home_button.y, W, H));
    assert!(host.home_visible());
    assert_eq!(
        host.editor_state().editor_ui.entry_surface,
        EntrySurface::Home
    );
    // Same document: the mutation, the page count, the page selection,
    // and the undo depth all survive the round trip.
    assert_eq!(starter_frame_x(&host), 30.0, "the edit survives");
    assert_eq!(
        host.editor_state()
            .doc
            .pages
            .as_ref()
            .map_or(1, |p| p.len()),
        page_count
    );
    assert_eq!(host.editor_state().ui.active_page_index, 1);
    assert_eq!(host.editor_state().history.past.len(), undo_depth);

    // 普通 → 专业 again, still the same editor.
    let professional = compact_layout(&host).professional;
    assert!(host.apply_press(
        professional.origin.x + 4.0,
        professional.origin.y + 4.0,
        W,
        H
    ));
    assert!(!host.home_visible());
    assert_eq!(starter_frame_x(&host), 30.0);
    assert_eq!(host.editor_state().history.past.len(), undo_depth);

    // And undo is still correct after the double crossing.
    assert!(host.apply_undo());
    assert_eq!(starter_frame_x(&host), 0.0, "undo restores the pre-edit x");
}

#[test]
fn compact_nav_settings_opens_the_agent_settings_modal() {
    let mut host = compact_host();
    let nav = compact_layout(&host).nav_items[2];
    let point = center(nav);
    assert!(host.apply_press(point.x, point.y, W, H));
    assert!(host.editor_state().editor_ui.agent_settings_open);
}

#[test]
fn compact_settings_gear_shares_the_nav_destination() {
    let mut host = compact_host();
    let gear = compact_layout(&host).settings;
    let point = center(gear);
    assert!(host.apply_press(point.x, point.y, W, H));
    assert!(host.editor_state().editor_ui.agent_settings_open);
}

#[test]
fn compact_blank_tile_requests_a_new_document() {
    let mut host = compact_host();
    let blank = compact_layout(&host).new_canvas;
    let point = center(blank);
    assert!(host.apply_press(point.x, point.y, W, H));
    assert!(
        host.apply_release_with_viewport(W, H),
        "replay the deferred tap"
    );
    assert_eq!(
        host.editor_state().editor_ui.pending_file_action,
        Some(FileAction::New)
    );
}

#[test]
fn compact_mode_normal_segment_confirms_without_leaving_home() {
    let mut host = compact_host();
    let normal = compact_layout(&host).mode_normal;
    let point = center(normal);
    assert!(host.apply_press(point.x, point.y, W, H));
    assert!(
        host.home_visible(),
        "the 普通 half only confirms the selected segment"
    );
    assert_eq!(
        host.editor_state().editor_ui.entry_surface,
        EntrySurface::Home
    );
}

#[test]
fn compact_home_touch_drag_scrolls_the_page_column() {
    let mut host = compact_host();
    // A short viewport so the page overflows and scrolling has room.
    let (w, h) = (430.0, 360.0);
    let home = HomeSurface::for_editor(host.editor_state()).expect("home visible");
    let layout = home.layout(w, h);
    // Start on the task grid, mid page column (between the pinned bars).
    let start = center(layout.tabs_row);
    assert!(host.apply_press(start.x, start.y, w, h));
    assert!(
        host.touch_panel_gesture.is_some(),
        "a touch down on the compact page arms the scroll gesture"
    );
    // Drag up 60: content tracks the finger, scroll_y advances.
    assert!(host.apply_cursor_move(start.x, start.y - 60.0));
    assert!(host.editor_state().editor_ui.home.scroll_y > 0.0);
    // A stationary tap instead replays the press ladder exactly once:
    // the input box takes the caret at the text's end.
    let mut host = compact_host();
    host.editor_state_mut().editor_ui.home.set_draft("abc");
    host.editor_state_mut().editor_ui.home.set_caret(0, 1_000);
    let input = HomeSurface::for_editor(host.editor_state())
        .expect("home visible")
        .layout(W, H)
        .input_box;
    let input = center(input);
    assert!(host.apply_press(input.x, input.y, W, H));
    assert!(
        host.touch_panel_gesture.is_some(),
        "the tap defers until release"
    );
    assert!(host.apply_release_with_viewport(W, H));
    assert_eq!(
        host.editor_state().editor_ui.home.input.caret(),
        3,
        "the replayed press placed the caret in the Home input"
    );
    assert!(host.touch_panel_gesture.is_none());
}

#[test]
fn compact_wheel_scroll_stops_at_the_featured_card_bottom() {
    let mut host = compact_host();
    let (w, h) = (430.0, 360.0);
    // A wheel-style scroll below the top bar and above the nav.
    let scrolled = host.try_scroll_home(w, 200.0, -400.0, w, h);
    assert!(scrolled);
    let max = op_editor_ui::widgets::home_surface::max_scroll_for_mode(
        w,
        h,
        host.editor_state().editor_ui.home.task,
        120.0,
        true,
        host.editor_state().editor_ui.locale,
    );
    assert!(host.editor_state().editor_ui.home.scroll_y <= max);
    // Above the nav the wheel belongs to the pinned chrome, not the page.
    assert!(!host.try_scroll_home(w, h - 10.0, -40.0, w, h));
}

/// Paint one compact Home frame into a raster surface so draw-time
/// panics in the compact painter surface here, not on a phone.
#[test]
fn compact_home_paints_a_full_frame_without_panicking() {
    let mut host = compact_host();
    host.editor_state_mut()
        .editor_ui
        .home
        .set_draft("画一份周末咖啡店的促销海报");
    host.set_now_ms(5_000);
    let mut backend = crate::backend::NativeBackend::with_dpi(1.0);
    let mut surface =
        skia_safe::surfaces::raster_n32_premul((390, 844)).expect("raster surface allocated");
    surface.canvas().clear(skia_safe::Color::WHITE);
    let mut frame = crate::backend::NativeFrameBackend::new(&mut backend, surface.canvas());
    host.paint(&mut frame, 390.0, 844.0);
    assert_eq!(
        host.editor_state().editor_ui.home.shown_at_ms,
        5_000,
        "the takeover stamped the entrance clock"
    );
}

#[test]
fn compact_task_grid_selects_the_task() {
    let mut host = compact_host();
    let layout = compact_layout(&host);
    let poster = center(layout.tabs[6]);
    assert!(host.apply_press(poster.x, poster.y, W, H));
    assert!(
        host.apply_release_with_viewport(W, H),
        "replay the deferred tap"
    );
    assert_eq!(
        host.editor_state().editor_ui.home.task,
        HomeFamily::EventPoster
    );
}
