//! Tests for the generation-workspace state: phase transitions with the
//! run-epoch fence, dock clamping, view defaults per family, and the
//! document-swap reset.

use super::*;
use crate::tool::Tool;

fn opened(family: HomeFamily, epoch: u64) -> WorkspaceState {
    let mut workspace = WorkspaceState::default();
    workspace.open_for_generation(
        family,
        "取餐预约，3 个页面",
        TaskDraft::default(),
        epoch,
        5_000,
        Some(Tool::Select),
    );
    workspace
}

#[test]
fn view_defaults_per_family() {
    assert_eq!(
        WorkspaceView::default_for(HomeFamily::AppUi),
        WorkspaceView::AllBoards
    );
    assert_eq!(
        WorkspaceView::default_for(HomeFamily::KnowledgeCards),
        WorkspaceView::AllBoards
    );
    assert_eq!(
        WorkspaceView::default_for(HomeFamily::ScreenshotTutorial),
        WorkspaceView::AllBoards
    );
    assert_eq!(
        WorkspaceView::default_for(HomeFamily::EventPoster),
        WorkspaceView::AllBoards
    );
    assert_eq!(
        WorkspaceView::default_for(HomeFamily::Presentation),
        WorkspaceView::Single { index: 0 }
    );
    assert_eq!(
        WorkspaceView::default_for(HomeFamily::Web),
        WorkspaceView::LongPage
    );
    assert_eq!(
        WorkspaceView::default_for(HomeFamily::Infographic),
        WorkspaceView::LongPage
    );
}

#[test]
fn open_takes_the_stage_with_the_run_facts() {
    let workspace = opened(HomeFamily::Presentation, 7);
    assert!(workspace.active);
    assert!(workspace.visible);
    assert_eq!(workspace.phase, WorkspacePhase::Generating);
    assert_eq!(workspace.run_epoch, 7);
    assert_eq!(workspace.brief, "取餐预约，3 个页面");
    assert_eq!(workspace.previous_tool, Some(Tool::Select));
    assert_eq!(workspace.shown_at_ms, 5_000);
}

#[test]
fn mark_done_on_the_idle_edge_with_a_matching_epoch() {
    let mut workspace = opened(HomeFamily::AppUi, 7);
    assert!(workspace.mark_done(7));
    assert_eq!(workspace.phase, WorkspacePhase::Done);
    // Terminal phases are sticky — a second edge changes nothing.
    assert!(!workspace.mark_done(7));
    assert!(!workspace.mark_failed(7));
    assert_eq!(workspace.phase, WorkspacePhase::Done);
}

#[test]
fn mark_done_with_a_stale_epoch_is_ignored() {
    let mut workspace = opened(HomeFamily::AppUi, 7);
    assert!(!workspace.mark_done(3), "a stale epoch must not finish us");
    assert_eq!(workspace.phase, WorkspacePhase::Generating);
    // ...while an unstamped workspace accepts any epoch (the launch has
    // not identified the run yet).
    let mut unstamped = opened(HomeFamily::AppUi, 0);
    assert!(unstamped.mark_done(3));
    assert_eq!(unstamped.phase, WorkspacePhase::Done);
}

#[test]
fn stop_is_sticky_against_the_late_idle_edge() {
    let mut workspace = opened(HomeFamily::AppUi, 7);
    assert!(workspace.mark_stopped(7));
    assert_eq!(workspace.phase, WorkspacePhase::Stopped);
    assert!(
        !workspace.mark_done(7),
        "the stopped turn's late finish edge must not repaint Done"
    );
    assert_eq!(workspace.phase, WorkspacePhase::Stopped);
    assert!(!workspace.mark_failed(7));
}

#[test]
fn a_stale_epoch_cannot_stop_a_newer_run_either() {
    let mut workspace = opened(HomeFamily::AppUi, 7);
    assert!(!workspace.mark_stopped(3));
    assert_eq!(workspace.phase, WorkspacePhase::Generating);
}

#[test]
fn failure_only_lands_from_generating() {
    let mut workspace = opened(HomeFamily::AppUi, 7);
    assert!(workspace.mark_failed(7));
    assert_eq!(workspace.phase, WorkspacePhase::Failed);
    assert!(!workspace.mark_done(7));
    assert!(!workspace.mark_failed(7));
}

#[test]
fn a_retry_resumes_generating_with_the_new_epoch() {
    let mut workspace = opened(HomeFamily::AppUi, 7);
    workspace.mark_failed(7);
    workspace.resume_generating(9);
    assert_eq!(workspace.phase, WorkspacePhase::Generating);
    assert_eq!(workspace.run_epoch, 9);
    // The fence now discriminates against the old run's edges.
    assert!(!workspace.mark_done(7));
    assert!(workspace.mark_done(9));
}

#[test]
fn dock_width_clamps_into_the_drag_range() {
    // The dock's width IS the left panel's now — the shared clamp lives
    // on `EditorUiState::set_layer_panel_width` and is covered by the
    // slides-panel state tests. What remains workspace-specific is that
    // opening a run never widens the panel on its own: only the Chat
    // tab's one-time bump does.
    let mut ui = crate::EditorUiState {
        layer_panel_width: 404.0,
        ..crate::EditorUiState::default()
    };
    ui.open_workspace_for_generation(
        HomeFamily::AppUi,
        "取餐预约，3 个页面",
        TaskDraft::default(),
        7,
        5_000,
        Some(Tool::Select),
    );
    assert_eq!(ui.layer_panel_width, 404.0, "an open never moves the width");
    assert!(ui.workspace.active);
    assert!(ui.sidebar_open, "the dock is the left panel, open");
    assert_eq!(ui.slides_panel.tab, crate::LeftPanelTab::Chat);
    assert!(ui.chat_pinned(), "the run's chat is pinned into the dock");
}

#[test]
fn professional_and_reenter_round_trip_keeps_the_facts() {
    let mut workspace = opened(HomeFamily::Presentation, 7);
    workspace.view = WorkspaceView::Single { index: 2 };
    workspace.phase = WorkspacePhase::Done;
    workspace.enter_professional();
    assert!(!workspace.visible);
    assert!(workspace.active, "active survives the pro-canvas visit");
    assert_eq!(workspace.view, WorkspaceView::Single { index: 2 });
    assert_eq!(workspace.phase, WorkspacePhase::Done);
    workspace.reenter(20_000);
    assert!(workspace.visible);
    assert_eq!(
        workspace.shown_at_ms, 20_000,
        "reentry restamps the entrance"
    );
    // Reentering an already-visible workspace is a no-op (no restamp).
    workspace.reenter(30_000);
    assert_eq!(workspace.shown_at_ms, 20_000);
}

/// The workspace and the professional editor share ONE left column: a
/// round trip across 专业编辑 / 回到工作区 keeps the panel open, the
/// Chat tab active and the width — nothing jumps, the modes differ
/// only in their chrome.
#[test]
fn the_workspace_round_trip_preserves_the_panel_tab_and_width() {
    let mut state = crate::EditorState::new();
    state.editor_ui.open_workspace_for_generation(
        HomeFamily::AppUi,
        "做一个咖啡点单 App",
        TaskDraft::default(),
        7,
        5_000,
        Some(Tool::Select),
    );
    // The one-time Chat-tab bump ran on open.
    assert_eq!(state.editor_ui.layer_panel_width, 320.0);
    state.editor_ui.set_layer_panel_width(372.0);

    state.editor_ui.workspace.enter_professional();
    let ui = &state.editor_ui;
    assert!(ui.sidebar_open, "专业编辑 keeps the left panel open");
    assert_eq!(ui.slides_panel.tab, crate::LeftPanelTab::Chat);
    assert_eq!(ui.layer_panel_width, 372.0);
    assert!(ui.chat_pinned(), "the rail still owns the chat's rect");

    state.editor_ui.workspace.reenter(20_000);
    let ui = &state.editor_ui;
    assert_eq!(
        (ui.slides_panel.tab, ui.layer_panel_width),
        (crate::LeftPanelTab::Chat, 372.0),
        "回到工作区 keeps the same panel and width"
    );
    assert!(ui.chat_pinned());

    // Closing the column unpins in both modes: no chat, no floating
    // controls, and the canvas reclaims the width.
    state.editor_ui.sidebar_open = false;
    assert!(!state.editor_ui.chat_pinned());
    state.editor_ui.workspace.enter_professional();
    assert!(!state.editor_ui.chat_pinned());
}

#[test]
fn selection_steps_and_clamps_within_the_deck() {
    let mut workspace = opened(HomeFamily::Presentation, 7);
    workspace.view = WorkspaceView::Single { index: 0 };
    assert!(workspace.step_selected(1, 5));
    assert_eq!(workspace.selected, 1);
    assert_eq!(workspace.view, WorkspaceView::Single { index: 1 });
    assert!(workspace.step_selected(-1, 5), "clamped at the first board");
    assert_eq!(workspace.selected, 0);
    assert!(
        !workspace.step_selected(-9, 5),
        "already at the first board"
    );
    assert!(workspace.step_selected(9, 5), "clamped at the last board");
    assert_eq!(workspace.selected, 4);
    workspace.select_board(99, 5);
    assert_eq!(workspace.selected, 4);
    assert_eq!(workspace.view, WorkspaceView::Single { index: 4 });
    // An empty deck has nothing to select.
    assert!(!workspace.step_selected(1, 0));
}

#[test]
fn a_document_swap_clears_every_transient_flag() {
    let mut workspace = opened(HomeFamily::AppUi, 7);
    workspace.mark_done(7);
    workspace.reset_for_new_document();
    assert!(!workspace.active);
    assert!(!workspace.visible);
    assert!(workspace.brief.is_empty());
    assert_eq!(workspace.run_epoch, 0);
    assert_eq!(workspace.phase, WorkspacePhase::Generating);
    assert_eq!(workspace.previous_tool, None);
    assert_eq!(workspace.fitted_board_count, 0);
}

#[test]
fn entrance_deadline_spans_the_window_and_settles() {
    let mut workspace = WorkspaceState::default();
    assert_eq!(workspace.entrance_deadline_ms(0), None);
    workspace.open_for_generation(
        HomeFamily::AppUi,
        "brief",
        TaskDraft::default(),
        0,
        1_000,
        None,
    );
    assert_eq!(
        workspace.entrance_deadline_ms(1_000),
        Some(1_000 + WORKSPACE_ENTER_MS)
    );
    assert_eq!(
        workspace.entrance_deadline_ms(1_000 + WORKSPACE_ENTER_MS),
        None
    );
}

#[test]
fn the_chat_is_composer_only_until_the_agent_tab_owns_the_rail() {
    // Desktop has exactly one home for the conversation. Anywhere else
    // the chat is a launcher docked at the canvas floor, never a second
    // place to read the transcript.
    let mut ui = crate::EditorUiState {
        sidebar_open: true,
        ..crate::EditorUiState::default()
    };
    assert!(ui.chat_composer_only(), "Layers tab: composer only");
    assert!(!ui.chat_pinned());

    ui.enter_chat_tab();
    assert!(!ui.chat_composer_only(), "Agent tab: the real panel");
    assert!(ui.chat_pinned());

    // The workspace's dock is the chat's home too, so no composer there.
    ui.slides_panel.tab = LeftPanelTab::Layers;
    ui.workspace.visible = true;
    assert!(!ui.chat_composer_only(), "the workspace dock IS the chat");

    // Touch chrome hosts a sheet, not a docked composer.
    ui.workspace.visible = false;
    ui.touch = true;
    assert!(!ui.chat_composer_only());
}
