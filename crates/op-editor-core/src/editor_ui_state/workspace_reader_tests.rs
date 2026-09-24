//! Tests for the phone works reader's state: reading postures, the
//! page-edit binding lifecycle, and opening a finished document.

use super::super::super::home::TaskDraft;
use super::*;
use crate::size_class::EditorSizeClass;

fn compact_ui() -> EditorUiState {
    let mut ui = EditorUiState::new();
    ui.touch = true;
    ui.size_class = EditorSizeClass::Compact;
    ui
}

#[test]
fn reader_shows_only_on_a_compact_touch_host() {
    let mut ui = EditorUiState::new();
    ui.workspace.visible = true;
    assert!(!ui.works_reader_visible(), "desktop keeps its workspace");
    ui.touch = true;
    ui.size_class = EditorSizeClass::Medium;
    assert!(!ui.works_reader_visible(), "tablets keep their composition");
    ui.size_class = EditorSizeClass::Compact;
    assert!(ui.works_reader_visible());
    ui.workspace.visible = false;
    assert!(!ui.works_reader_visible(), "professional mode hides it");
}

#[test]
fn long_page_families_scroll_and_the_rest_page() {
    assert!(reads_as_long_page(HomeFamily::Web));
    assert!(reads_as_long_page(HomeFamily::Infographic));
    for family in [
        HomeFamily::AppUi,
        HomeFamily::Presentation,
        HomeFamily::KnowledgeCards,
        HomeFamily::ScreenshotTutorial,
        HomeFamily::EventPoster,
    ] {
        assert!(!reads_as_long_page(family), "{family:?}");
        assert!(reader_is_paged(family, 1), "{family:?} reserves its pager");
        assert_eq!(
            WorkspaceView::default_for_reader(family),
            WorkspaceView::Single { index: 0 }
        );
    }
    assert!(!reader_is_paged(HomeFamily::Web, 1));
    assert!(reader_is_paged(HomeFamily::Web, 2));
    assert_eq!(
        WorkspaceView::default_for_reader(HomeFamily::Infographic),
        WorkspaceView::LongPage
    );
}

#[test]
fn a_phone_generation_opens_on_one_board_not_all_boards() {
    let mut ui = compact_ui();
    ui.open_workspace_for_generation(HomeFamily::AppUi, "取餐", TaskDraft::default(), 0, 10, None);
    assert_eq!(ui.workspace.view, WorkspaceView::Single { index: 0 });
    let mut desktop = EditorUiState::new();
    desktop.open_workspace_for_generation(
        HomeFamily::AppUi,
        "取餐",
        TaskDraft::default(),
        0,
        10,
        None,
    );
    assert_eq!(desktop.workspace.view, WorkspaceView::AllBoards);
}

#[test]
fn a_staged_page_edit_binds_exactly_the_next_run() {
    let mut workspace = WorkspaceState::default();
    workspace.open_for_reading(HomeFamily::Presentation, 5);
    workspace.stage_page_edit("slide-3", 2);
    assert_eq!(workspace.page_edit.as_ref().unwrap().page_number(), 3);

    let target = workspace.begin_page_edit_turn().expect("staged target");
    assert_eq!(target.board_id, "slide-3");
    assert!(workspace.page_edit.is_none(), "consumed by one send");
    assert_eq!(workspace.page_edit_running, Some(target));
    assert!(
        workspace.begin_page_edit_turn().is_none(),
        "a second send is a whole-work turn again"
    );

    workspace.resume_generating(7);
    assert!(workspace.mark_done(7));
    assert!(
        workspace.page_edit_running.is_none(),
        "the settled run releases its binding"
    );
}

#[test]
fn stop_and_failure_release_the_running_binding_too() {
    for stop in [true, false] {
        let mut workspace = WorkspaceState::default();
        workspace.open_for_reading(HomeFamily::AppUi, 5);
        workspace.stage_page_edit("s1", 0);
        workspace.begin_page_edit_turn();
        workspace.resume_generating(9);
        if stop {
            assert!(workspace.mark_stopped(9));
        } else {
            assert!(workspace.mark_failed(9));
        }
        assert!(workspace.page_edit_running.is_none());
    }
}

#[test]
fn continue_chat_drops_a_staged_binding() {
    let mut workspace = WorkspaceState::default();
    workspace.stage_page_edit("s1", 0);
    workspace.clear_staged_page_edit();
    assert!(workspace.begin_page_edit_turn().is_none());
}

#[test]
fn opening_for_reading_is_a_finished_work_without_a_run() {
    let mut workspace = WorkspaceState::default();
    workspace.stage_page_edit("stale", 4);
    workspace.open_for_reading(HomeFamily::Web, 42);
    assert!(workspace.active && workspace.visible);
    assert_eq!(workspace.phase, WorkspacePhase::Done);
    assert_eq!(workspace.view, WorkspaceView::LongPage);
    assert_eq!(workspace.run_epoch, 0);
    assert!(workspace.page_edit.is_none());
    assert_eq!(workspace.shown_at_ms, 42);
}

#[test]
fn a_document_swap_forgets_every_page_binding() {
    let mut workspace = WorkspaceState::default();
    workspace.open_for_reading(HomeFamily::AppUi, 1);
    workspace.stage_page_edit("a", 0);
    workspace.page_edit_running = Some(PageEditTarget {
        board_id: "b".into(),
        index: 1,
    });
    workspace.reset_for_new_document();
    assert!(workspace.page_edit.is_none());
    assert!(workspace.page_edit_running.is_none());
}

#[test]
fn reading_family_follows_the_board_shapes() {
    assert_eq!(infer_reading_family(&[]), HomeFamily::AppUi);
    assert_eq!(
        infer_reading_family(&[(375.0, 812.0), (375.0, 812.0)]),
        HomeFamily::AppUi
    );
    assert_eq!(
        infer_reading_family(&[(1920.0, 1080.0), (1920.0, 1080.0)]),
        HomeFamily::Presentation
    );
    assert_eq!(infer_reading_family(&[(1440.0, 4200.0)]), HomeFamily::Web);
    // Any wide landscape board reads page by page, like a slide.
    assert_eq!(
        infer_reading_family(&[(1440.0, 1100.0)]),
        HomeFamily::Presentation
    );
}
