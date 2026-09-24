//! Host-level tests for the phone works reader: it replaces the
//! professional canvas after a Home send, Home does not cancel a run,
//! the pager / swipe / long-page scroll move the camera, 改这一页 binds
//! the current board, Stop / Retry reuse the real paths, 普通 ⇄ 专业
//! keeps one document, and the 作品 list reopens the live work.

use super::super::WidgetHostNative;
use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
use op_editor_core::{
    EntrySurface, FileAction, HomeFamily, NodeId, ReaderHit, TaskDraft, Tool, WorkspacePhase,
};
use op_editor_ui::widgets::{HomeSurface, MobileAppBar, WorksReader};
use op_editor_ui::{Point2D, Rect};

const W: f32 = 390.0;
const H: f32 = 844.0;

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn phone_state(document: jian_ops_schema::PenDocument) -> op_editor_core::EditorState {
    let mut state = op_editor_core::EditorState::from_document(document);
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state
}

/// `count` boards of `w`×`h` side by side.
fn boards_document(count: usize, w: u32, h: u32) -> jian_ops_schema::PenDocument {
    let children: Vec<String> = (0..count)
        .map(|i| {
            format!(
                r#"{{ "type": "frame", "id": "b{i}", "name": "第 {n} 页", "x": {x}, "y": 0,
                     "width": {w}, "height": {h}, "children": [
                       {{ "type": "rectangle", "id": "r{i}", "x": 10, "y": 10,
                          "width": 100, "height": 100 }} ] }}"#,
                n = i + 1,
                x = i as u32 * (w + 100)
            )
        })
        .collect();
    let source = format!(
        r#"{{ "version": "1.0.0", "children": [{}] }}"#,
        children.join(",")
    );
    jian_ops_schema::load_str(&source).expect("fixture").value
}

/// A phone host reading a finished `family` work of `count` boards.
fn reading_host(family: HomeFamily, count: usize, w: u32, h: u32) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(phone_state(boards_document(count, w, h))));
    host.set_now_ms(1_000);
    host.editor_state_mut()
        .editor_ui
        .open_workspace_for_generation(
            family,
            "为 OpenPencil 做一份产品介绍",
            TaskDraft::default(),
            0,
            1_000,
            Some(Tool::Select),
        );
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Done;
    host.frame_reader_board(W, H);
    host
}

fn layout(host: &WidgetHostNative) -> op_editor_ui::widgets::ReaderLayout {
    WorksReader::for_editor(host.editor_state())
        .expect("reader visible")
        .layout(W, H)
}

fn tap(host: &mut WidgetHostNative, rect: Rect) {
    let point = center(rect);
    host.apply_press(point.x, point.y, W, H);
    host.apply_release_with_viewport(W, H);
}

fn paint_once(host: &mut WidgetHostNative) {
    let mut backend = crate::backend::NativeBackend::with_dpi(1.0);
    let mut surface =
        skia_safe::surfaces::raster_n32_premul((W as i32, H as i32)).expect("raster surface");
    let mut frame = crate::backend::NativeFrameBackend::new(&mut backend, surface.canvas());
    host.paint(&mut frame, W, H);
}

#[test]
fn a_phone_home_send_opens_the_reader_not_the_professional_canvas() {
    let mut state = op_editor_core::EditorState::starter();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.home.visible = true;
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host.editor_state_mut()
        .editor_ui
        .home
        .set_draft("做一份 5 页产品介绍 PPT");
    assert!(host.queue_home_send());
    assert!(host.works_reader_visible(), "the reader owns the phone");
    assert!(!host.workspace_visible(), "never the desktop workspace");
    paint_once(&mut host);
    // The canvas paints into the reader's stage, not under an app bar.
    let stage = layout(&host).stage;
    let (x, y, w, h) = host.canvas_region(W, H);
    assert_eq!(
        (x, y, w, h),
        (stage.origin.x, stage.origin.y, stage.size.x, stage.size.y)
    );
}

#[test]
fn back_to_home_keeps_the_run_and_the_featured_card_returns_to_the_reader() {
    let mut host = reading_host(HomeFamily::Presentation, 3, 1920, 1080);
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Generating;
    host.editor_state_mut().chat.pending_send = Some("生成中的需求".into());
    let target = layout(&host).back;
    tap(&mut host, target);
    assert!(host.home_visible());
    assert!(!host.works_reader_visible(), "Home paints over the reader");
    let ui = &host.editor_state().editor_ui;
    assert!(ui.workspace.active, "the run is not cancelled");
    assert_eq!(ui.workspace.phase, WorkspacePhase::Generating);
    assert!(!host.editor_state().chat.pending_stop_chat);
    assert!(host.editor_state().chat.pending_send.is_some());

    let card = {
        let home = HomeSurface::for_editor(host.editor_state()).unwrap();
        home.layout(W, H).use_example
    };
    tap(&mut host, card);
    assert!(!host.home_visible());
    assert!(
        host.works_reader_visible(),
        "回到工作区 re-enters the reader"
    );
}

#[test]
fn the_pager_and_a_swipe_turn_pages_and_frame_each_board() {
    let mut host = reading_host(HomeFamily::Presentation, 3, 1920, 1080);
    let first = host.editor_state().viewport;
    let target = layout(&host).next.unwrap();
    tap(&mut host, target);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 1);
    let second = host.editor_state().viewport;
    assert!(second.pan_x < first.pan_x, "the camera moved to board 2");
    assert!(
        (second.zoom - first.zoom).abs() < 1e-4,
        "same-size boards, same fit"
    );

    // A left swipe on the stage is the next page too.
    let stage = layout(&host).stage;
    let start = center(stage);
    host.apply_press(start.x, start.y, W, H);
    host.apply_cursor_move(start.x - 60.0, start.y);
    host.apply_cursor_move(start.x - 120.0, start.y + 4.0);
    host.apply_release_with_viewport(W, H);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 2);

    // A short drag snaps back instead of leaving the board off-centre.
    let framed = host.editor_state().viewport;
    host.apply_press(start.x, start.y, W, H);
    host.apply_cursor_move(start.x + 20.0, start.y + 30.0);
    host.apply_release_with_viewport(W, H);
    assert_eq!(host.editor_state().viewport, framed);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 2);

    let target = layout(&host).prev.unwrap();
    tap(&mut host, target);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 1);
}

#[test]
fn a_long_page_scrolls_vertically_within_its_edges() {
    let mut host = reading_host(HomeFamily::Web, 1, 1440, 6000);
    assert!(layout(&host).pager.is_none());
    let top = host.editor_state().viewport;
    let stage = layout(&host).stage;
    let start = center(stage);
    // Scroll up past the top: clamped, nothing moves.
    host.apply_press(start.x, start.y, W, H);
    host.apply_cursor_move(start.x, start.y + 200.0);
    host.apply_release_with_viewport(W, H);
    assert_eq!(host.editor_state().viewport.pan_y, top.pan_y);
    // Scroll down: the page moves up, horizontally fixed.
    host.apply_press(start.x, start.y, W, H);
    host.apply_cursor_move(start.x + 30.0, start.y - 300.0);
    host.apply_release_with_viewport(W, H);
    let scrolled = host.editor_state().viewport;
    assert_eq!(scrolled.pan_y, top.pan_y - 300.0);
    assert_eq!(scrolled.pan_x, top.pan_x);
    // Fling far past the bottom: the page's bottom edge stops at the stage.
    host.apply_press(start.x, start.y, W, H);
    host.apply_cursor_move(start.x, start.y - 100_000.0);
    host.apply_release_with_viewport(W, H);
    let bottom = host.editor_state().viewport;
    let page_bottom = stage.origin.y + bottom.pan_y + 6000.0 * bottom.zoom;
    assert!((page_bottom - (stage.origin.y + stage.size.y - 16.0)).abs() < 0.5);
}

#[test]
fn edit_this_page_binds_the_board_on_show_and_opens_the_chat_sheet() {
    let mut host = reading_host(HomeFamily::Presentation, 3, 1920, 1080);
    let target = layout(&host).next.unwrap();
    tap(&mut host, target);
    let target = layout(&host).edit_page;
    tap(&mut host, target);
    let state = host.editor_state();
    assert_eq!(
        state.selection.anchor.as_str(),
        "b1",
        "the page is the scope"
    );
    let staged = state.editor_ui.workspace.page_edit.clone().expect("staged");
    assert_eq!((staged.board_id.as_str(), staged.page_number()), ("b1", 2));
    assert_eq!(state.editor_ui.mobile_sheet, Some(MobileSheetKind::Ai));
    assert!(
        state.chat.focused,
        "the keyboard comes up for the instruction"
    );
    // The reader stays underneath: paging did not drop the selection.
    assert!(host.works_reader_visible());
}

#[test]
fn continue_chat_opens_the_sheet_without_a_page_binding() {
    let mut host = reading_host(HomeFamily::Presentation, 2, 1920, 1080);
    host.editor_state_mut()
        .editor_ui
        .workspace
        .stage_page_edit("b0", 0);
    let target = layout(&host).continue_chat;
    tap(&mut host, target);
    assert_eq!(
        host.editor_state().editor_ui.mobile_sheet,
        Some(MobileSheetKind::Ai)
    );
    assert!(host.editor_state().editor_ui.workspace.page_edit.is_none());
}

#[test]
fn a_live_run_disables_edit_and_offers_stop_which_uses_the_real_stop() {
    let mut host = reading_host(HomeFamily::Presentation, 2, 1920, 1080);
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Generating;
    host.editor_state_mut().chat.set_input_text("继续");
    assert!(host.editor_state_mut().chat.begin_send());
    let target = layout(&host).edit_page;
    tap(&mut host, target);
    assert!(host.editor_state().editor_ui.workspace.page_edit.is_none());
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);

    let stop = layout(&host).status_action.expect("stop offered");
    tap(&mut host, stop);
    let state = host.editor_state();
    assert_eq!(state.editor_ui.workspace.phase, WorkspacePhase::Stopped);
    assert!(
        state.chat.pending_stop_chat,
        "the pump's stop drain retires the worker"
    );
    assert!(
        state.chat.pending_send.is_none(),
        "the queued send is cancelled"
    );
}

#[test]
fn retry_resends_the_stored_brief() {
    let mut host = reading_host(HomeFamily::Presentation, 2, 1920, 1080);
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Failed;
    let retry = layout(&host).status_action.expect("retry offered");
    tap(&mut host, retry);
    let state = host.editor_state();
    assert_eq!(state.editor_ui.workspace.phase, WorkspacePhase::Generating);
    let sent = state.chat.pending_send.as_deref().expect("brief re-sent");
    assert!(sent.contains("为 OpenPencil 做一份产品介绍"), "{sent}");
}

#[test]
fn normal_professional_round_trip_keeps_document_selection_and_history() {
    let mut host = reading_host(HomeFamily::Presentation, 3, 1920, 1080);
    let target = layout(&host).next.unwrap();
    tap(&mut host, target);
    // 专业: the full mobile canvas over the same editor.
    let target = layout(&host).mode_professional;
    tap(&mut host, target);
    assert!(!host.works_reader_visible());
    assert!(host.editor_state().editor_ui.workspace.active);
    assert_eq!(
        host.editor_state().tool,
        Tool::Select,
        "the tool comes back"
    );

    // Edit there with real history.
    host.editor_state_mut()
        .set_single_selection(NodeId::new("r1"));
    host.editor_state_mut().commit_history();
    assert!(host.editor_state_mut().translate_selected(40.0, 0.0));
    let depth = host.editor_state().history.past.len();
    let revision = host.editor_state().document_revision();

    // 普通 from the professional app bar returns to THIS work's reader.
    let app_bar =
        op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_rect(host.editor_state(), W);
    tap(&mut host, MobileAppBar::home_rect(app_bar));
    assert!(host.works_reader_visible(), "普通 is the reader, not Home");
    assert!(!host.home_visible());
    assert_eq!(
        host.editor_state().editor_ui.workspace.selected,
        1,
        "same page"
    );
    assert_eq!(host.editor_state().selection.anchor.as_str(), "r1");
    assert_eq!(host.editor_state().history.past.len(), depth);
    assert_eq!(
        host.editor_state().document_revision(),
        revision,
        "same document"
    );

    // And back to 专业 with the edit still undoable.
    let target = layout(&host).mode_professional;
    tap(&mut host, target);
    assert!(host.editor_state_mut().undo());
    assert_eq!(host.editor_state().history.past.len(), depth - 1);
}

#[test]
fn professional_from_home_over_a_reader_leaves_the_reader() {
    let mut host = reading_host(HomeFamily::AppUi, 2, 375, 812);
    let target = layout(&host).back;
    tap(&mut host, target);
    let professional = HomeSurface::for_editor(host.editor_state())
        .unwrap()
        .layout(W, H)
        .professional;
    tap(&mut host, professional);
    assert!(!host.home_visible());
    assert!(!host.works_reader_visible(), "专业 means the full canvas");
    assert_eq!(
        host.editor_state().editor_ui.entry_surface,
        EntrySurface::Canvas
    );
}

#[test]
fn the_works_list_reopens_a_canvas_drawn_in_professional_mode() {
    // A blank-canvas session: content drawn, no Home brief, no workspace.
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(phone_state(boards_document(2, 375, 812))));
    host.editor_state_mut().editor_ui.home.visible = true;
    let nav = HomeSurface::for_editor(host.editor_state())
        .unwrap()
        .layout(W, H)
        .nav_items[1];
    tap(&mut host, nav);
    assert!(host.editor_state().editor_ui.home.works_open);
    let current = {
        let home = HomeSurface::for_editor(host.editor_state()).unwrap();
        home.works_layout(W, H)
            .current
            .expect("the drawn canvas is a work")
    };
    tap(&mut host, current);
    assert!(host.works_reader_visible());
    let workspace = &host.editor_state().editor_ui.workspace;
    assert_eq!(workspace.family, HomeFamily::AppUi, "phone screens page");
    assert_eq!(workspace.phase, WorkspacePhase::Done);
    assert_eq!(
        WorksReader::for_editor(host.editor_state())
            .unwrap()
            .boards
            .len(),
        2
    );
}

#[test]
fn a_recent_work_opens_in_the_reader_once_the_shell_loads_it() {
    let mut host = WidgetHostNative::new();
    let mut state = phone_state(boards_document(1, 375, 812));
    state.editor_ui.recent_files = vec![op_editor_core::RecentFile {
        path: "/tmp/作品.op".into(),
        modified_at: 0,
    }];
    state.editor_ui.home.visible = true;
    state.editor_ui.home.works_open = true;
    assert!(host.replace_editor_state(state));
    let row = HomeSurface::for_editor(host.editor_state())
        .unwrap()
        .works_layout(W, H)
        .rows[0];
    tap(&mut host, row);
    assert_eq!(
        host.editor_state().editor_ui.pending_file_action,
        Some(FileAction::OpenRecent(0))
    );
    // The shell loads the file and swaps the document in.
    let mut loaded = phone_state(boards_document(3, 1920, 1080));
    loaded.editor_ui.home.hide();
    assert!(host.replace_editor_state(loaded));
    assert!(host.works_reader_visible(), "the tapped work opens to read");
    assert_eq!(
        host.editor_state().editor_ui.workspace.family,
        HomeFamily::Presentation
    );
    // The intent is one-shot: the next swap opens normally.
    assert!(host.replace_editor_state(phone_state(boards_document(1, 375, 812))));
    assert!(!host.works_reader_visible());
}

#[test]
fn a_new_board_takes_the_stage_while_a_run_streams() {
    let mut host = reading_host(HomeFamily::Presentation, 2, 1920, 1080);
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Generating;
    assert!(host.pump_workspace_generation(W, H, true));
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 1);
    // A page edit in flight never moves the reader off its page.
    host.editor_state_mut()
        .editor_ui
        .workspace
        .select_board(0, 2);
    host.editor_state_mut()
        .editor_ui
        .workspace
        .stage_page_edit("b0", 0);
    host.editor_state_mut()
        .editor_ui
        .workspace
        .begin_page_edit_turn();
    host.editor_state_mut()
        .editor_ui
        .workspace
        .fitted_board_count = 0;
    host.editor_state_mut().editor_ui.workspace.fitted_bounds = None;
    host.pump_workspace_generation(W, H, true);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 0);
}

#[test]
fn every_reader_press_is_swallowed_but_an_open_sheet_owns_its_presses() {
    let mut host = reading_host(HomeFamily::Presentation, 2, 1920, 1080);
    let title = layout(&host).title;
    let before = host.editor_state().selection.anchor.clone();
    tap(&mut host, title);
    assert_eq!(
        host.editor_state().selection.anchor,
        before,
        "no canvas select"
    );
    let target = layout(&host).continue_chat;
    tap(&mut host, target);
    // With the sheet open, a press on the (now covered) pager is the
    // sheet's: the reader tier stands aside.
    let next = layout(&host).next.unwrap();
    let point = center(next);
    let hit = WorksReader::for_editor(host.editor_state())
        .unwrap()
        .hit_test(W, H, point);
    assert_eq!(hit, Some(ReaderHit::Next));
    host.apply_press(point.x, point.y, W, H);
    host.apply_release_with_viewport(W, H);
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 0);
}
