//! Geometry and hit-test tests for the phone works reader. Paint and
//! hit-test share `reader_layout`, so these pin the one rect set both
//! read.

use super::*;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{HomeFamily, TaskDraft};

const W: f32 = 390.0;
const H: f32 = 844.0;

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// A compact touch editor whose active page holds `boards` 1920×1080
/// frames, with the workspace opened for `family`.
fn reading(family: HomeFamily, boards: usize, phase: WorkspacePhase) -> EditorState {
    let children: Vec<String> = (0..boards)
        .map(|i| {
            format!(
                r#"{{ "type": "frame", "id": "b{i}", "x": {x}, "y": 0, "width": 1920,
                     "height": 1080, "children": [] }}"#,
                x = i * 2000
            )
        })
        .collect();
    let source = format!(
        r#"{{ "version": "1.0.0", "children": [{}] }}"#,
        children.join(",")
    );
    let document = jian_ops_schema::load_str(&source).expect("fixture").value;
    let mut state = EditorState::from_document(document);
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.open_workspace_for_generation(
        family,
        "为 OpenPencil 做一份 5 页产品介绍",
        TaskDraft::default(),
        0,
        1,
        None,
    );
    state.editor_ui.workspace.phase = phase;
    state
}

#[test]
fn every_reader_target_keeps_the_touch_floor() {
    let state = reading(HomeFamily::Presentation, 3, WorkspacePhase::Failed);
    let reader = WorksReader::for_editor(&state).expect("reader visible");
    let layout = reader.layout(W, H);
    let mut targets = vec![
        layout.back,
        layout.mode_normal,
        layout.mode_professional,
        layout.continue_chat,
        layout.edit_page,
        layout.status_action.expect("retry offered"),
    ];
    targets.extend(layout.prev);
    targets.extend(layout.next);
    for rect in targets {
        assert!(
            rect.size.x >= 44.0 && rect.size.y >= 44.0,
            "{rect:?} is under 44 pt"
        );
    }
}

#[test]
fn the_bands_stack_without_overlap_down_the_screen() {
    let state = reading(HomeFamily::Presentation, 3, WorkspacePhase::Done);
    let reader = WorksReader::for_editor(&state).unwrap();
    let layout = reader.layout(W, H);
    let pager = layout.pager.expect("a deck is paged");
    let bottom = |r: Rect| r.origin.y + r.size.y;
    assert_eq!(bottom(layout.header), layout.stage.origin.y);
    assert_eq!(bottom(layout.stage), pager.origin.y);
    assert_eq!(bottom(pager), layout.status.origin.y);
    assert_eq!(bottom(layout.status), layout.bottom_bar.origin.y);
    assert_eq!(bottom(layout.bottom_bar), H);
    assert!(layout.stage.size.y > 400.0, "the work gets the screen");
}

#[test]
fn a_long_page_gives_the_pager_row_to_the_stage() {
    let web = reading(HomeFamily::Web, 1, WorkspacePhase::Done);
    let deck = reading(HomeFamily::Presentation, 1, WorkspacePhase::Done);
    let web_layout = WorksReader::for_editor(&web).unwrap().layout(W, H);
    let deck_layout = WorksReader::for_editor(&deck).unwrap().layout(W, H);
    assert!(web_layout.pager.is_none());
    assert_eq!(
        web_layout.stage.size.y,
        deck_layout.stage.size.y + READER_PAGER_H
    );
}

#[test]
fn canvas_region_is_the_stage_while_the_reader_is_up() {
    use crate::widgets::host_canvas_geometry::{canvas_origin, canvas_region};
    let state = reading(HomeFamily::Presentation, 2, WorkspacePhase::Done);
    let stage = WorksReader::for_editor(&state).unwrap().layout(W, H).stage;
    assert_eq!(
        canvas_region(&state, W, H),
        (stage.origin.x, stage.origin.y, stage.size.x, stage.size.y)
    );
    assert_eq!(canvas_origin(&state), (stage.origin.x, stage.origin.y));
}

#[test]
fn hit_test_answers_every_painted_target() {
    let mut state = reading(HomeFamily::Presentation, 3, WorkspacePhase::Done);
    state.editor_ui.workspace.selected = 1;
    let reader = WorksReader::for_editor(&state).unwrap();
    let layout = reader.layout(W, H);
    let hit = |rect: Rect| reader.hit_test_layout(&layout, center(rect));
    assert_eq!(hit(layout.back), Some(ReaderHit::Back));
    assert_eq!(hit(layout.mode_normal), Some(ReaderHit::ModeNormal));
    assert_eq!(
        hit(layout.mode_professional),
        Some(ReaderHit::ModeProfessional)
    );
    assert_eq!(hit(layout.prev.unwrap()), Some(ReaderHit::Prev));
    assert_eq!(hit(layout.next.unwrap()), Some(ReaderHit::Next));
    assert_eq!(hit(layout.continue_chat), Some(ReaderHit::ContinueChat));
    assert_eq!(hit(layout.edit_page), Some(ReaderHit::EditPage));
    assert_eq!(hit(layout.stage), Some(ReaderHit::Stage));
    assert!(layout.status_action.is_none(), "a finished run offers none");
}

#[test]
fn pager_ends_and_a_live_run_disable_their_targets() {
    let state = reading(HomeFamily::Presentation, 2, WorkspacePhase::Generating);
    let reader = WorksReader::for_editor(&state).unwrap();
    let layout = reader.layout(W, H);
    let hit = |rect: Rect| reader.hit_test_layout(&layout, center(rect));
    assert_eq!(hit(layout.prev.unwrap()), None, "already on page 1");
    assert_eq!(hit(layout.next.unwrap()), Some(ReaderHit::Next));
    assert_eq!(
        hit(layout.edit_page),
        None,
        "改这一页 waits for the run to settle"
    );
    assert_eq!(
        hit(layout.status_action.expect("stop offered")),
        Some(ReaderHit::Stop)
    );
}

#[test]
fn stopped_and_failed_runs_offer_retry() {
    for phase in [WorkspacePhase::Stopped, WorkspacePhase::Failed] {
        let state = reading(HomeFamily::AppUi, 1, phase);
        let reader = WorksReader::for_editor(&state).unwrap();
        let layout = reader.layout(W, H);
        assert_eq!(
            reader.hit_test_layout(&layout, center(layout.status_action.unwrap())),
            Some(ReaderHit::Retry)
        );
    }
}

#[test]
fn a_briefless_reading_offers_no_retry_it_could_not_honour() {
    let mut state = reading(HomeFamily::AppUi, 1, WorkspacePhase::Failed);
    state.editor_ui.workspace.brief.clear();
    let reader = WorksReader::for_editor(&state).unwrap();
    assert_eq!(reader.status_action(), None);
    assert!(reader.layout(W, H).status_action.is_none());
}

#[test]
fn the_reader_is_touch_only_and_takes_the_tablet_forms() {
    let mut state = reading(HomeFamily::AppUi, 1, WorkspacePhase::Done);
    state.editor_ui.size_class = EditorSizeClass::Medium;
    let reader = WorksReader::for_editor(&state).expect("tablets read the work too");
    assert_eq!(reader.form(820.0, 1180.0), ReaderForm::TabletPortrait);
    assert_eq!(reader.form(1180.0, 820.0), ReaderForm::TabletLandscape);
    state.editor_ui.touch = false;
    assert!(WorksReader::for_editor(&state).is_none());
}

#[test]
fn the_reader_canvas_hides_selection_chrome_but_keeps_the_selection() {
    let mut state = reading(HomeFamily::Presentation, 2, WorkspacePhase::Done);
    state.set_single_selection(op_editor_core::NodeId::new("b1"));
    let scene = crate::layout_scene::LayoutScene::default();
    let canvas = crate::widgets::CanvasViewport::from_editor(&state, &scene);
    assert!(canvas.selected_set.is_empty());
    assert_eq!(state.selection.anchor.as_str(), "b1", "selection kept");
    state.editor_ui.workspace.visible = false;
    let canvas = crate::widgets::CanvasViewport::from_editor(&state, &scene);
    assert_eq!(canvas.selected_set, vec!["b1".to_string()]);
}
