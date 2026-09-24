//! Tests for the narrow-window chat drawer.

use super::*;
use crate::{HomeFamily, TaskDraft};

fn workspace_ui() -> EditorUiState {
    let mut ui = EditorUiState::default();
    ui.open_workspace_for_generation(
        HomeFamily::AppUi,
        "brief",
        TaskDraft::default(),
        0,
        1_000,
        None,
    );
    ui
}

#[test]
fn crossing_the_breakpoint_switches_mode_and_starts_shut() {
    let mut ui = workspace_ui();
    assert!(!ui.workspace.sync_drawer_mode(1280.0), "wide stays docked");
    assert!(!ui.workspace_drawer_active());
    assert!(ui.workspace.sync_drawer_mode(820.0));
    assert!(ui.workspace_drawer_active());
    assert!(!ui.workspace.drawer_open, "the design gets the width first");
    assert!(!ui.workspace.sync_drawer_mode(700.0), "no change inside");
    assert!(ui.workspace.sync_drawer_mode(WORKSPACE_DRAWER_BREAKPOINT));
    assert!(!ui.workspace_drawer_active());
}

#[test]
fn the_drawer_owns_pinning_without_touching_the_dock_flag() {
    let mut ui = workspace_ui();
    assert!(ui.sidebar_open);
    ui.workspace.sync_drawer_mode(820.0);
    assert!(!ui.chat_pinned(), "shut drawer: nothing pinned");
    assert!(ui.workspace.set_drawer_open(true, 2_000));
    assert!(ui.chat_pinned());
    assert!(ui.sidebar_open, "the docked preference is untouched");
    ui.workspace.sync_drawer_mode(1280.0);
    assert!(ui.chat_pinned(), "back to the docked column");
}

#[test]
fn the_slide_eases_over_its_window_and_reduced_motion_settles() {
    let mut ui = workspace_ui();
    ui.workspace.sync_drawer_mode(820.0);
    ui.workspace.set_drawer_open(true, 2_000);
    assert_eq!(ui.workspace_drawer_progress(2_000), 0.0);
    let mid = ui.workspace_drawer_progress(2_000 + WORKSPACE_DRAWER_SLIDE_MS / 2);
    assert!(
        mid > 0.5 && mid < 1.0,
        "ease-out is past half at half time: {mid}"
    );
    assert_eq!(
        ui.workspace_drawer_progress(2_000 + WORKSPACE_DRAWER_SLIDE_MS),
        1.0
    );
    assert_eq!(
        ui.workspace_drawer_deadline_ms(2_100),
        Some(2_000 + WORKSPACE_DRAWER_SLIDE_MS)
    );
    ui.reduced_motion = true;
    assert_eq!(ui.workspace_drawer_progress(2_000), 1.0);
    assert_eq!(ui.workspace_drawer_deadline_ms(2_100), None);
    ui.reduced_motion = false;
    ui.workspace.set_drawer_open(false, 3_000);
    assert_eq!(ui.workspace_drawer_progress(3_000), 1.0);
    assert_eq!(
        ui.workspace_drawer_progress(3_000 + WORKSPACE_DRAWER_SLIDE_MS),
        0.0
    );
}

#[test]
fn the_drawer_always_leaves_a_strip_of_canvas() {
    let mut ui = workspace_ui();
    ui.layer_panel_width = 440.0;
    assert_eq!(ui.workspace_drawer_width(820.0), 440.0);
    assert_eq!(
        ui.workspace_drawer_width(400.0),
        400.0 - WORKSPACE_DRAWER_MIN_GUTTER
    );
}

#[test]
fn a_new_run_and_a_new_document_shut_the_drawer() {
    let mut ui = workspace_ui();
    ui.workspace.sync_drawer_mode(820.0);
    ui.workspace.set_drawer_open(true, 2_000);
    ui.open_workspace_for_generation(
        HomeFamily::AppUi,
        "again",
        TaskDraft::default(),
        0,
        5_000,
        None,
    );
    assert!(ui.workspace.drawer_mode, "the window did not change");
    assert!(!ui.workspace.drawer_open);
    ui.workspace.set_drawer_open(true, 6_000);
    ui.workspace.reset_for_new_document();
    assert!(!ui.workspace.drawer_open);
}
