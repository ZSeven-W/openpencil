//! Generation-workspace host wiring tests: the chat pin, the docked
//! canvas region, the chrome's press actions, and the deck strip.

use super::WidgetHostNative;
use op_editor_core::{EntrySurface, HomeFamily, Tool, WorkspacePhase, WorkspaceView};
use op_editor_ui::Point2D;

const W: f32 = 1440.0;
const H: f32 = 900.0;

/// A host whose active page holds `boards` frames (the shape a
/// generated document has). `install_imported_state` preserves the
/// live shell UI, so the workspace is opened AFTER the install.
fn host_with_boards(family: HomeFamily, boards: usize) -> WidgetHostNative {
    host_with_boards_preserving(family, boards, |_| {})
}

/// The same host, with a hook over the freshly installed state (used
/// to vary phase/view before the workspace opens).
fn host_with_boards_preserving(
    family: HomeFamily,
    boards: usize,
    tune: impl Fn(&mut op_editor_core::EditorState),
) -> WidgetHostNative {
    let mut children = String::new();
    for index in 0..boards {
        children.push_str(&format!(
            r#"{{ "type": "frame", "id": "board-{index}", "x": {x}, "y": 0,
                "width": 375, "height": 812, "children": [] }},"#,
            x = index * 420
        ));
    }
    let children = children.trim_end_matches(',');
    let source = format!(r#"{{ "version": "1.0.0", "children": [{children}] }}"#);
    let document = jian_ops_schema::load_str(&source)
        .expect("parse workspace fixture")
        .value;
    let state = op_editor_core::EditorState::from_document(document);
    let mut host = WidgetHostNative::new();
    host.install_imported_state(state);
    host.set_now_ms(2_000);
    {
        let editor = host.editor_state_mut();
        editor.tool = Tool::Hand;
        tune(editor);
        // The same entry the Home send takes: the workspace opens with
        // the LEFT PANEL open on the Chat tab, whose one-time bump takes
        // the column from the 240 default to 320.
        editor.editor_ui.open_workspace_for_generation(
            family,
            "取餐预约，3 个页面",
            op_editor_core::TaskDraft::default(),
            0,
            1_000,
            Some(Tool::Select),
        );
    }
    host
}

#[test]
fn the_chat_panel_pins_into_the_dock_rect() {
    let host = host_with_boards(HomeFamily::AppUi, 3);
    let rect = host.ai_chat_rect(W, H).expect("pinned chat rect");
    assert_eq!(rect.origin.x, 0.0);
    assert_eq!(rect.origin.y, 64.0);
    assert_eq!(
        rect.size.x, 320.0,
        "the dock width is the bumped rail width"
    );
    assert_eq!(rect.size.y, H - 64.0);

    // Collapsing the dock (the toolbar toggle) hides the chat entirely.
    let mut collapsed = host_with_boards(HomeFamily::AppUi, 3);
    collapsed.editor_state_mut().editor_ui.sidebar_open = false;
    assert_eq!(collapsed.ai_chat_rect(W, H), None);
    let (x, _, w, _) = collapsed.canvas_region(W, H);
    assert_eq!((x, w), (0.0, W), "the canvas takes the full width back");
}

/// The professional editor's Chat tab pins the very same chat into the
/// left rail, below the tab row — and a closed panel means NO chat, not
/// a floating one over the canvas.
#[test]
fn the_chat_tab_pins_the_chat_into_the_left_rail() {
    let mut host = host_with_boards(HomeFamily::AppUi, 3);
    host.editor_state_mut()
        .editor_ui
        .workspace
        .enter_professional();
    // 专业编辑 kept the panel open on the Chat tab at the bumped width.
    assert_eq!(
        host.editor_state().editor_ui.slides_panel.tab,
        op_editor_core::LeftPanelTab::Chat
    );
    assert_eq!(host.editor_state().editor_ui.layer_panel_width, 320.0);

    let rect = host.ai_chat_rect(W, H).expect("the rail owns the chat");
    let top_bar = op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let tab_row = op_editor_ui::widgets::SLIDES_TAB_ROW_HEIGHT;
    assert_eq!(rect.origin.x, 0.0);
    assert_eq!(rect.origin.y, top_bar + tab_row, "below the tab row");
    assert_eq!(rect.size.x, 320.0);
    assert_eq!(rect.size.y, H - top_bar - tab_row);

    // Closing the left panel no longer removes the chat — the composer
    // card takes over at the canvas floor (the floating branch does
    // not come back on desktop). Its height is resolved from the live
    // composer layout, not a second copy of the constant.
    host.editor_state_mut().editor_ui.sidebar_open = false;
    let card = host
        .ai_chat_rect(W, H)
        .expect("composer card, never a floating panel");
    let width = op_editor_ui::widgets::host_canvas_geometry::COMPOSER_CARD_W;
    let height = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state())
        .composer_only_height(width);
    assert_eq!(card.origin.x, 12.0, "canvas is full-width again");
    assert_eq!(card.origin.y + card.size.y, H - 12.0);
    assert_eq!(card.size, Point2D::new(width, height));

    // Presenting is the one desktop state with no chat at all.
    host.editor_state_mut().editor_ui.sidebar_open = true;
    host.editor_state_mut().editor_ui.preview.mode = true;
    assert_eq!(
        host.ai_chat_rect(W, H),
        None,
        "a presentation is no chat, never a floating panel"
    );
}

#[test]
fn the_canvas_region_starts_at_the_dock() {
    let host = host_with_boards(HomeFamily::AppUi, 3);
    let (x, y, w, h) = host.canvas_region(W, H);
    assert_eq!((x, y, w, h), (320.0, 108.0, 1120.0, 792.0));

    let deck = host_with_boards(HomeFamily::Presentation, 3);
    let (x, y, _, h) = deck.canvas_region(W, H);
    assert_eq!((x, y), (320.0, 108.0));
    assert_eq!(h, 682.0, "the deck strip reserves the bottom 110 px");

    let mut collapsed = host_with_boards(HomeFamily::AppUi, 3);
    collapsed.editor_state_mut().editor_ui.sidebar_open = false;
    let (x, _, w, _) = collapsed.canvas_region(W, H);
    assert_eq!(
        (x, w),
        (0.0, W),
        "a collapsed dock gives the canvas the full width"
    );
}

#[test]
fn pressing_professional_hides_the_workspace_and_restores_the_tool() {
    let mut host = host_with_boards(HomeFamily::AppUi, 3);
    let professional = {
        let surface =
            op_editor_ui::widgets::WorkspaceSurface::for_editor_at(host.editor_state(), 0)
                .expect("workspace");
        surface.layout(W, H).professional
    };
    assert!(host.apply_press(
        professional.origin.x + 4.0,
        professional.origin.y + 4.0,
        W,
        H
    ));
    let workspace = &host.editor_state().editor_ui.workspace;
    assert!(!workspace.visible);
    assert!(workspace.active, "active survives the pro-canvas visit");
    assert_eq!(
        host.editor_state().tool,
        Tool::Select,
        "the previous tool returns"
    );
}

#[test]
fn pressing_back_shows_home_and_keeps_the_workspace_active() {
    let mut host = host_with_boards(HomeFamily::AppUi, 3);
    let back = {
        let surface =
            op_editor_ui::widgets::WorkspaceSurface::for_editor_at(host.editor_state(), 0)
                .expect("workspace");
        surface.layout(W, H).back
    };
    assert!(host.apply_press(back.origin.x + 4.0, back.origin.y + 4.0, W, H));
    assert!(host.editor_state().editor_ui.home.visible);
    assert_eq!(
        host.editor_state().editor_ui.entry_surface,
        EntrySurface::Home
    );
    let workspace = &host.editor_state().editor_ui.workspace;
    assert!(
        workspace.active,
        "returning to Home keeps the workspace alive"
    );
    assert!(workspace.visible, "no generation was cancelled");
    assert_eq!(workspace.phase, WorkspacePhase::Generating);
}

#[test]
fn home_footer_returns_to_the_live_workspace() {
    let mut host = host_with_boards(HomeFamily::AppUi, 3);
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Done;
    host.editor_state_mut().editor_ui.home.visible = true;
    let link = {
        let home = op_editor_ui::widgets::HomeSurface::for_editor_at(host.editor_state(), 0)
            .expect("home");
        home.layout(W, H).use_example
    };
    assert!(host.apply_press(link.origin.x + 4.0, link.origin.y + 4.0, W, H));
    assert!(!host.editor_state().editor_ui.home.visible);
    let workspace = &host.editor_state().editor_ui.workspace;
    assert!(workspace.visible);
    assert_eq!(workspace.phase, WorkspacePhase::Done, "state was kept");
}

#[test]
fn a_deck_family_frames_board_two_from_its_thumbnail() {
    let mut host = host_with_boards(HomeFamily::Presentation, 3);
    host.editor_state_mut().editor_ui.workspace.view = WorkspaceView::Single { index: 0 };
    host.refresh_layout_scene();
    host.fit_content_to_viewport(W, H);
    let before = host.editor_state().viewport;
    let thumb = {
        let surface =
            op_editor_ui::widgets::WorkspaceSurface::for_editor_at(host.editor_state(), 0)
                .expect("workspace");
        let layout = surface.layout(W, H);
        layout.thumbs[2]
    };
    let point = Point2D::new(thumb.origin.x + 4.0, thumb.origin.y + 4.0);
    assert!(host.apply_press(point.x, point.y, W, H));
    let workspace = &host.editor_state().editor_ui.workspace;
    assert_eq!(workspace.selected, 2);
    assert_eq!(workspace.view, WorkspaceView::Single { index: 2 });
    assert_ne!(
        host.editor_state().viewport,
        before,
        "Thumb(2) reframes the camera onto board 2"
    );
}

#[test]
fn the_dock_handle_drag_clamps_the_width() {
    let mut host = host_with_boards(HomeFamily::AppUi, 3);
    let handle = {
        let surface =
            op_editor_ui::widgets::WorkspaceSurface::for_editor_at(host.editor_state(), 0)
                .expect("workspace");
        surface.layout(W, H).dock_handle.expect("handle")
    };
    // Press the handle, drag past the max, release. The dock drag writes
    // the shared layer_panel_width with the shared 240–440 clamp.
    assert!(host.apply_press(handle.origin.x + 2.0, 500.0, W, H));
    host.apply_cursor_move(700.0, 500.0);
    assert_eq!(host.editor_state().editor_ui.layer_panel_width, 440.0);
    host.apply_cursor_move(-2_000.0, 500.0);
    assert_eq!(host.editor_state().editor_ui.layer_panel_width, 240.0);
    host.apply_cursor_move(330.0, 500.0);
    // Press landed 2 px inside the 5 px handle: start_x = 317, so the
    // width tracks the press-relative delta (320 + 13).
    assert_eq!(host.editor_state().editor_ui.layer_panel_width, 333.0);
    host.apply_release();
    // After release, moves no longer resize.
    host.apply_cursor_move(700.0, 500.0);
    assert_eq!(host.editor_state().editor_ui.layer_panel_width, 333.0);
}

#[test]
fn generation_pump_refits_allboards_once_per_new_count() {
    let mut host = host_with_boards(HomeFamily::AppUi, 2);
    host.editor_state_mut().editor_ui.workspace.view = WorkspaceView::AllBoards;
    assert!(host.pump_workspace_generation(W, H, true));
    assert_eq!(
        host.editor_state().editor_ui.workspace.fitted_board_count,
        2
    );
    // Same count again: nothing to do.
    assert!(!host.pump_workspace_generation(W, H, true));
    // A new board lands (the orchestrator appends a top-level frame):
    // install a fresh 3-board document — the shell UI, including the
    // workspace and its fitted count, is preserved — then refit.
    let third = {
        let document = jian_ops_schema::load_str(
            r#"{ "version": "1.0.0", "children": [
                { "type": "frame", "id": "board-0", "x": 0, "y": 0, "width": 375, "height": 812, "children": [] },
                { "type": "frame", "id": "board-1", "x": 420, "y": 0, "width": 375, "height": 812, "children": [] },
                { "type": "frame", "id": "board-2", "x": 840, "y": 0, "width": 375, "height": 812, "children": [] }
            ] }"#,
        )
        .expect("parse 3-board fixture")
        .value;
        op_editor_core::EditorState::from_document(document)
    };
    host.install_imported_state(third);
    assert!(
        host.editor_state().editor_ui.workspace.active,
        "the shell UI survived the document swap"
    );
    assert!(host.pump_workspace_generation(W, H, true));
    assert_eq!(
        host.editor_state().editor_ui.workspace.fitted_board_count,
        3
    );
}

#[test]
fn a_second_home_brief_starts_on_a_fresh_page() {
    // Home is the "start something new" surface: a brief launched from
    // it must not append to whatever the previous run drew, or the two
    // deliverables share a canvas, a deck strip and a transcript.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.home.visible = true;
    host.editor_state_mut().editor_ui.home.task = HomeFamily::AppUi;
    host.editor_state_mut()
        .editor_ui
        .home
        .set_draft("做一个咖啡点单 App");
    assert!(host.queue_home_send(), "first brief sends");
    // Pretend the run drew a board, then come back to Home for a second
    // brief of a different family.
    host.editor_state_mut().editor_ui.workspace.visible = false;
    host.editor_state_mut().editor_ui.home.visible = true;
    host.editor_state_mut().editor_ui.home.task = HomeFamily::Presentation;
    host.editor_state_mut()
        .editor_ui
        .home
        .set_draft("做一份产品介绍 PPT");
    let before = host.editor_state().chat.messages.len();
    assert!(before > 0, "the first brief left a transcript");
    assert!(host.queue_home_send(), "second brief sends");
    assert_eq!(
        host.editor_state().editor_ui.workspace.family,
        HomeFamily::Presentation
    );
}
