//! Host-level tests for the tablet works reader: a tablet send lands in
//! the reader (never the professional canvas with its rails), the strip /
//! swipe page it, the chat opens into the landscape panel or the portrait
//! sheet without freezing the board, a rotation reframes, and the Home
//! works grid reopens the live work in it.

use super::super::WidgetHostNative;
use op_editor_core::size_class::{size_class, MobileSheetKind};
use op_editor_core::{HomeFamily, HomeHit, NodeId, TaskDraft, Tool, WorkspacePhase};
use op_editor_ui::widgets::{HomeSurface, ReaderLayout, WorksReader};
use op_editor_ui::{Point2D, Rect};

const LANDSCAPE: (f32, f32) = (1366.0, 1024.0);
const PORTRAIT: (f32, f32) = (1024.0, 1366.0);
const ANDROID_PORTRAIT: (f32, f32) = (800.0, 1280.0);

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner.origin.x >= outer.origin.x - 0.5
        && inner.origin.y >= outer.origin.y - 0.5
        && inner.origin.x + inner.size.x <= outer.origin.x + outer.size.x + 0.5
        && inner.origin.y + inner.size.y <= outer.origin.y + outer.size.y + 0.5
}

fn deck(count: usize) -> jian_ops_schema::PenDocument {
    let children: Vec<String> = (0..count)
        .map(|i| {
            format!(
                r#"{{ "type": "frame", "id": "b{i}", "name": "Slide {n}", "x": {x}, "y": 0,
                     "width": 1920, "height": 1080, "children": [
                       {{ "type": "rectangle", "id": "r{i}", "x": 40, "y": 40,
                          "width": 400, "height": 200 }} ] }}"#,
                n = i + 1,
                x = i * 2100
            )
        })
        .collect();
    let source = format!(
        r#"{{ "version": "1.0.0", "children": [{}] }}"#,
        children.join(",")
    );
    jian_ops_schema::load_str(&source).expect("fixture").value
}

/// A touch tablet at `(w, h)` (size class as the shell computes it, rails
/// open on Expanded) reading a finished `count`-slide deck.
fn tablet_host((w, h): (f32, f32), count: usize) -> WidgetHostNative {
    let mut state = op_editor_core::EditorState::from_document(deck(count));
    state.editor_ui.touch = true;
    state.editor_ui.size_class = size_class(w, h);
    state.editor_ui.sidebar_open = state.editor_ui.size_class.is_rail_layout();
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host.set_now_ms(1_000);
    host.editor_state_mut()
        .editor_ui
        .open_workspace_for_generation(
            HomeFamily::Presentation,
            "A product intro deck",
            TaskDraft::default(),
            0,
            1_000,
            Some(Tool::Select),
        );
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Done;
    host.frame_reader_board(w, h);
    host
}

fn layout(host: &WidgetHostNative, (w, h): (f32, f32)) -> ReaderLayout {
    WorksReader::for_editor(host.editor_state())
        .expect("reader visible")
        .layout(w, h)
}

fn tap(host: &mut WidgetHostNative, point: Point2D, (w, h): (f32, f32)) {
    host.apply_press(point.x, point.y, w, h);
    host.apply_release_with_viewport(w, h);
}

fn paint_once(host: &mut WidgetHostNative, (w, h): (f32, f32)) {
    let mut backend = crate::backend::NativeBackend::with_dpi(1.0);
    let mut surface =
        skia_safe::surfaces::raster_n32_premul((w as i32, h as i32)).expect("raster surface");
    let mut frame = crate::backend::NativeFrameBackend::new(&mut backend, surface.canvas());
    host.paint(&mut frame, w, h);
}

/// The current board's on-screen rect through the host's camera.
fn board_on_screen(host: &WidgetHostNative, viewport: (f32, f32)) -> Rect {
    host.reader_board_screen_rect(viewport.0, viewport.1)
        .expect("board framed")
}

#[test]
fn a_tablet_run_reads_in_the_reader_without_editing_rails() {
    for viewport in [LANDSCAPE, PORTRAIT, ANDROID_PORTRAIT] {
        let mut host = tablet_host(viewport, 4);
        assert!(host.works_reader_visible(), "{viewport:?}");
        assert!(!host.workspace_visible(), "never the desktop workspace");
        let ui = &host.editor_state().editor_ui;
        assert!(!ui.expanded_touch_layout(), "no rails beside the reader");
        assert!(!host.layers_panel_visible(), "{viewport:?}");
        paint_once(&mut host, viewport);
        let stage = layout(&host, viewport).stage;
        let (x, y, w, h) = host.canvas_region(viewport.0, viewport.1);
        assert_eq!(
            (x, y, w, h),
            (stage.origin.x, stage.origin.y, stage.size.x, stage.size.y)
        );
        assert!(
            contains(stage, board_on_screen(&host, viewport)),
            "{viewport:?}"
        );
    }
}

#[test]
fn the_strip_and_a_swipe_page_through_the_deck() {
    let mut host = tablet_host(LANDSCAPE, 5);
    let (_, third) = layout(&host, LANDSCAPE).thumbs[2];
    tap(&mut host, center(third), LANDSCAPE);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 2);
    assert!(contains(
        layout(&host, LANDSCAPE).stage,
        board_on_screen(&host, LANDSCAPE)
    ));
    // A left swipe across the stage turns to the next slide.
    let stage = layout(&host, LANDSCAPE).stage;
    let start = center(stage);
    host.apply_press(start.x, start.y, LANDSCAPE.0, LANDSCAPE.1);
    host.apply_cursor_move(start.x - 120.0, start.y + 4.0);
    host.apply_cursor_move(start.x - 240.0, start.y + 6.0);
    host.apply_release_with_viewport(LANDSCAPE.0, LANDSCAPE.1);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 3);
    let next = layout(&host, LANDSCAPE).next.unwrap();
    tap(&mut host, center(next), LANDSCAPE);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 4);
    tap(&mut host, center(next), LANDSCAPE);
    assert_eq!(
        host.editor_state().editor_ui.workspace.selected,
        4,
        "clamped"
    );
}

#[test]
fn landscape_edit_opens_the_chat_in_the_panel_and_the_binding_follows_the_page() {
    let mut host = tablet_host(LANDSCAPE, 4);
    let reader = layout(&host, LANDSCAPE);
    let panel = reader.side_panel.expect("side panel");
    tap(&mut host, center(reader.edit_page), LANDSCAPE);
    let ui = &host.editor_state().editor_ui;
    assert_eq!(ui.mobile_sheet, Some(MobileSheetKind::Ai));
    assert_eq!(ui.workspace.page_edit.as_ref().unwrap().board_id, "b0");
    assert!(!host.mobile_sheet_is_modal(), "the board stays interactive");
    let chat = host.ai_chat_rect(LANDSCAPE.0, LANDSCAPE.1).expect("chat");
    assert_eq!(chat, panel, "the chat opens INTO the side panel");

    // Paging with the chat open re-binds the staged edit to the page on show.
    let (_, tile) = layout(&host, LANDSCAPE).thumbs[2];
    tap(&mut host, center(tile), LANDSCAPE);
    let state = host.editor_state();
    assert_eq!(state.editor_ui.workspace.selected, 2);
    let staged = state.editor_ui.workspace.page_edit.as_ref().unwrap();
    assert_eq!((staged.board_id.as_str(), staged.index), ("b2", 2));
    assert_eq!(state.selection.anchor, NodeId::new("b2"));
    assert_eq!(state.editor_ui.mobile_sheet, Some(MobileSheetKind::Ai));

    // A press inside the chat belongs to the chat, not the reader.
    host.apply_press(center(chat).x, center(chat).y, LANDSCAPE.0, LANDSCAPE.1);
    assert_eq!(host.editor_state().editor_ui.workspace.reader_pressed, None);
    host.apply_release_with_viewport(LANDSCAPE.0, LANDSCAPE.1);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 2);
}

#[test]
fn continue_chat_drops_a_staged_page_binding_on_a_tablet() {
    let mut host = tablet_host(LANDSCAPE, 3);
    host.editor_state_mut()
        .editor_ui
        .workspace
        .stage_page_edit("b1", 1);
    let target = layout(&host, LANDSCAPE).continue_chat;
    tap(&mut host, center(target), LANDSCAPE);
    let ui = &host.editor_state().editor_ui;
    assert!(ui.workspace.page_edit.is_none());
    assert_eq!(ui.mobile_sheet, Some(MobileSheetKind::Ai));
}

#[test]
fn portrait_chat_sheet_shrinks_the_stage_and_keeps_the_page_in_view() {
    let mut host = tablet_host(PORTRAIT, 3);
    paint_once(&mut host, PORTRAIT);
    let before = layout(&host, PORTRAIT).stage;
    let edit = layout(&host, PORTRAIT).edit_page;
    tap(&mut host, center(edit), PORTRAIT);
    let sheet = host.ai_chat_rect(PORTRAIT.0, PORTRAIT.1).expect("sheet");
    paint_once(&mut host, PORTRAIT);
    let stage = layout(&host, PORTRAIT).stage;
    assert!(stage.size.y < before.size.y);
    assert!(stage.origin.y + stage.size.y <= sheet.origin.y);
    let board = board_on_screen(&host, PORTRAIT);
    assert!(
        contains(stage, board),
        "reframed above the sheet: {board:?}"
    );
    // The stage above the sheet still swipes.
    let start = center(stage);
    host.apply_press(start.x, start.y, PORTRAIT.0, PORTRAIT.1);
    host.apply_cursor_move(start.x - 200.0, start.y);
    host.apply_release_with_viewport(PORTRAIT.0, PORTRAIT.1);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 1);
}

#[test]
fn a_rotation_reframes_the_board_for_the_new_stage() {
    let mut host = tablet_host(LANDSCAPE, 3);
    paint_once(&mut host, LANDSCAPE);
    // iPad Pro 12.9" is Expanded both ways; only the form changes.
    assert_eq!(
        size_class(PORTRAIT.0, PORTRAIT.1),
        size_class(LANDSCAPE.0, LANDSCAPE.1)
    );
    paint_once(&mut host, PORTRAIT);
    let stage = layout(&host, PORTRAIT).stage;
    assert!(layout(&host, PORTRAIT).side_panel.is_none());
    assert!(contains(stage, board_on_screen(&host, PORTRAIT)));
}

#[test]
fn professional_brings_the_rails_back_and_the_header_is_not_the_ghost_app_bar() {
    let mut host = tablet_host(LANDSCAPE, 2);
    let switch = layout(&host, LANDSCAPE).mode_professional;
    assert_eq!(
        host.collab_history_action_at(center(switch).x, center(switch).y, LANDSCAPE.0, LANDSCAPE.1),
        None,
        "the reader's 专业 segment is not the hidden app bar's undo / redo"
    );
    tap(&mut host, center(switch), LANDSCAPE);
    assert!(!host.works_reader_visible());
    assert!(host.editor_state().editor_ui.expanded_touch_layout());
}

#[test]
fn the_home_works_grid_reopens_the_live_work_in_the_reader() {
    let mut host = tablet_host(PORTRAIT, 3);
    let back = layout(&host, PORTRAIT).back;
    tap(&mut host, center(back), PORTRAIT);
    assert!(host.home_visible());
    let card = {
        let home = HomeSurface::for_editor(host.editor_state()).expect("home");
        let page = home.layout(PORTRAIT.0, PORTRAIT.1);
        let grid = home.works_grid(&page).expect("tablet grid");
        let (hit, rect) = grid.cards[0];
        assert_eq!(hit, HomeHit::WorksCurrent);
        rect
    };
    let max = HomeSurface::for_editor(host.editor_state())
        .unwrap()
        .tablet_max_scroll(PORTRAIT.0, PORTRAIT.1);
    assert!(card.origin.y + card.size.y <= PORTRAIT.1 + max);
    host.editor_state_mut().editor_ui.home.scroll_y = max;
    let card = card.origin.y - max;
    let point = Point2D::new(200.0, card + 40.0);
    tap(&mut host, point, PORTRAIT);
    assert!(!host.home_visible());
    assert!(
        host.works_reader_visible(),
        "the work reopens in the reader"
    );
}
