//! Tablet reader geometry: every tablet size and orientation keeps the
//! 44 pt floor, the bands tile the screen without overlap, the strip
//! windows onto the current board, the chat rect sits where each form
//! promises, and hit-testing answers the tablet targets.

use super::super::{ReaderForm, WorksReader};
use super::*;
use crate::Point2D;
use op_editor_core::size_class::{size_class, MobileSheetKind};
use op_editor_core::{EditorState, HomeFamily, ReaderHit, TaskDraft, WorkspacePhase};

/// iPad Pro 12.9" and 11", iPad Air, Android tablets — both ways.
const TABLETS: [(f32, f32); 10] = [
    (1024.0, 1366.0),
    (1366.0, 1024.0),
    (834.0, 1194.0),
    (1194.0, 834.0),
    (820.0, 1180.0),
    (1180.0, 820.0),
    (800.0, 1280.0),
    (1280.0, 800.0),
    (768.0, 1024.0),
    (1024.0, 768.0),
];

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn bottom(rect: Rect) -> f32 {
    rect.origin.y + rect.size.y
}

fn right(rect: Rect) -> f32 {
    rect.origin.x + rect.size.x
}

/// A touch tablet (`w × h` decides the size class, as the shell's
/// `recompute_responsive_layout` does) reading `boards` 1920×1080 slides.
fn tablet(w: f32, h: f32, boards: usize, phase: WorkspacePhase) -> EditorState {
    let children: Vec<String> = (0..boards)
        .map(|i| {
            format!(
                r#"{{ "type": "frame", "id": "b{i}", "name": "Slide {n}", "x": {x}, "y": 0,
                     "width": 1920, "height": 1080, "children": [] }}"#,
                n = i + 1,
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
    state.editor_ui.size_class = size_class(w, h);
    state.editor_ui.open_workspace_for_generation(
        HomeFamily::Presentation,
        "A product intro deck",
        TaskDraft::default(),
        0,
        1,
        None,
    );
    state.editor_ui.workspace.phase = phase;
    state
}

#[test]
fn every_tablet_size_takes_its_orientation_form() {
    for (w, h) in TABLETS {
        let state = tablet(w, h, 3, WorkspacePhase::Done);
        let reader = WorksReader::for_editor(&state).expect("reader on a tablet");
        let expected = if w > h {
            ReaderForm::TabletLandscape
        } else {
            ReaderForm::TabletPortrait
        };
        assert_eq!(reader.form(w, h), expected, "{w}×{h}");
        assert_eq!(reader.layout(w, h).form, expected, "{w}×{h}");
    }
}

#[test]
fn every_tablet_target_keeps_the_touch_floor() {
    for (w, h) in TABLETS {
        let state = tablet(w, h, 8, WorkspacePhase::Failed);
        let layout = WorksReader::for_editor(&state).unwrap().layout(w, h);
        let mut targets = vec![
            layout.back,
            layout.mode_normal,
            layout.mode_professional,
            layout.continue_chat,
            layout.edit_page,
            layout.status_action.expect("retry offered"),
            layout.prev.expect("strip arrows"),
            layout.next.expect("strip arrows"),
        ];
        assert!(!layout.thumbs.is_empty(), "{w}×{h}: tiles on show");
        targets.extend(layout.thumbs.iter().map(|(_, rect)| *rect));
        for rect in targets {
            assert!(
                rect.size.x >= 44.0 && rect.size.y >= 44.0,
                "{w}×{h}: {rect:?} is under 44 pt"
            );
            assert!(
                rect.origin.x >= 0.0 && right(rect) <= w && bottom(rect) <= h,
                "{w}×{h}: {rect:?} leaves the screen"
            );
        }
    }
}

#[test]
fn landscape_keeps_viewer_strip_and_side_panel_apart() {
    for (w, h) in TABLETS.into_iter().filter(|(w, h)| w > h) {
        let state = tablet(w, h, 5, WorkspacePhase::Done);
        let layout = WorksReader::for_editor(&state).unwrap().layout(w, h);
        let panel = layout.side_panel.expect("landscape side panel");
        let strip = layout.strip.expect("a deck is paged");
        assert_eq!(layout.stage.origin.y, READER_TABLET_HEADER_H);
        assert!(right(layout.stage) <= panel.origin.x + 0.5, "{w}×{h}");
        assert!(right(strip) <= panel.origin.x + 0.5, "{w}×{h}");
        assert!(bottom(layout.stage) <= strip.origin.y + 0.5, "{w}×{h}");
        assert_eq!(bottom(strip), h);
        // Status, page, reply and both actions stack inside the panel.
        for rect in [layout.status, layout.continue_chat, layout.edit_page] {
            assert!(
                rect.origin.x >= panel.origin.x && right(rect) <= right(panel),
                "{w}×{h}: {rect:?} outside the panel"
            );
        }
        assert!(bottom(layout.continue_chat) <= layout.edit_page.origin.y);
        let info = layout.page_info.expect("page line");
        assert!(bottom(layout.status) <= info.origin.y);
        if let Some(reply) = layout.reply {
            assert!(bottom(reply) <= layout.continue_chat.origin.y);
        }
        assert!(layout.pager.is_none() && layout.bottom_bar == Rect::ZERO);
    }
}

#[test]
fn portrait_stacks_stage_strip_status_and_bar() {
    for (w, h) in TABLETS.into_iter().filter(|(w, h)| w < h) {
        let state = tablet(w, h, 5, WorkspacePhase::Done);
        let layout = WorksReader::for_editor(&state).unwrap().layout(w, h);
        let strip = layout.strip.expect("a deck is paged");
        assert!(layout.side_panel.is_none());
        assert!(bottom(layout.stage) <= strip.origin.y + 0.5);
        assert!(bottom(strip) <= layout.status.origin.y + 0.5);
        assert!(bottom(layout.status) <= layout.bottom_bar.origin.y + 0.5);
        assert_eq!(bottom(layout.bottom_bar), h);
        // The buttons sit in a centred column, never stretched edge to
        // edge on a 1024 pt wide screen.
        let column = right(layout.edit_page) - layout.continue_chat.origin.x;
        assert!(column <= 680.5, "{w}×{h}: column {column}");
        let left = layout.continue_chat.origin.x;
        assert!((left - (w - right(layout.edit_page))).abs() < 1.0);
        assert!(layout.continue_chat.size.x < layout.edit_page.size.x);
    }
}

#[test]
fn the_strip_windows_onto_the_current_board() {
    let (w, h) = (800.0, 1280.0);
    let mut state = tablet(w, h, 20, WorkspacePhase::Done);
    for current in [0usize, 9, 19] {
        state.editor_ui.workspace.selected = current;
        let layout = WorksReader::for_editor(&state).unwrap().layout(w, h);
        let shown: Vec<usize> = layout.thumbs.iter().map(|(index, _)| *index).collect();
        assert!(shown.len() < 20, "twenty 16:9 tiles cannot fit 800 pt");
        assert!(shown.contains(&current), "{current} on show: {shown:?}");
        assert!(shown.windows(2).all(|pair| pair[1] == pair[0] + 1));
        let strip = layout.strip.unwrap();
        let prev = layout.prev.unwrap();
        let next = layout.next.unwrap();
        for (_, tile) in &layout.thumbs {
            assert!(tile.origin.x >= right(prev) && right(*tile) <= next.origin.x);
            assert!(tile.origin.y >= strip.origin.y && bottom(*tile) <= bottom(strip));
        }
    }
}

#[test]
fn tablet_hits_answer_tiles_arrows_and_actions() {
    let (w, h) = (1366.0, 1024.0);
    let mut state = tablet(w, h, 4, WorkspacePhase::Done);
    state.editor_ui.workspace.selected = 1;
    let reader = WorksReader::for_editor(&state).unwrap();
    let layout = reader.layout(w, h);
    let hit = |rect: Rect| reader.hit_test_layout(&layout, center(rect));
    for (index, tile) in &layout.thumbs {
        assert_eq!(hit(*tile), Some(ReaderHit::Page(*index)));
    }
    assert_eq!(hit(layout.prev.unwrap()), Some(ReaderHit::Prev));
    assert_eq!(hit(layout.next.unwrap()), Some(ReaderHit::Next));
    assert_eq!(hit(layout.edit_page), Some(ReaderHit::EditPage));
    assert_eq!(hit(layout.continue_chat), Some(ReaderHit::ContinueChat));
    assert_eq!(hit(layout.stage), Some(ReaderHit::Stage));
    assert_eq!(
        hit(layout.mode_professional),
        Some(ReaderHit::ModeProfessional)
    );
    // The panel's padding and the page line are dead chrome, never stage.
    let info = layout.page_info.unwrap();
    assert_eq!(hit(info), None);
}

#[test]
fn a_live_run_disables_edit_and_offers_stop_in_the_panel() {
    let (w, h) = (1280.0, 800.0);
    let state = tablet(w, h, 2, WorkspacePhase::Generating);
    let reader = WorksReader::for_editor(&state).unwrap();
    let layout = reader.layout(w, h);
    let stop = layout.status_action.expect("stop offered");
    assert!(stop.origin.x >= layout.side_panel.unwrap().origin.x);
    assert_eq!(
        reader.hit_test_layout(&layout, center(stop)),
        Some(ReaderHit::Stop)
    );
    assert_eq!(
        reader.hit_test_layout(&layout, center(layout.edit_page)),
        None
    );
}

#[test]
fn the_chat_opens_into_the_panel_or_a_sheet_over_a_shrunk_stage() {
    // Landscape: the side panel, from under the header to the keyboard.
    let (w, h) = (1366.0, 1024.0);
    let mut state = tablet(w, h, 3, WorkspacePhase::Done);
    let panel = WorksReader::for_editor(&state)
        .unwrap()
        .layout(w, h)
        .side_panel
        .unwrap();
    let chat = tablet_chat_rect(ReaderForm::TabletLandscape, w, h, h);
    assert_eq!(chat, panel);
    let raised = tablet_chat_rect(ReaderForm::TabletLandscape, w, h, h - 400.0);
    assert_eq!(bottom(raised), h - 400.0);
    state.editor_ui.mobile_sheet = Some(MobileSheetKind::Ai);
    let open = WorksReader::for_editor(&state).unwrap().layout(w, h);
    assert!(open.strip.is_some(), "the landscape viewer keeps paging");

    // Portrait: a floating sheet under a stage that shrank above it; the
    // strip / status / bar under the sheet neither paint nor hit.
    let (w, h) = (1024.0, 1366.0);
    let mut state = tablet(w, h, 3, WorkspacePhase::Done);
    let closed = WorksReader::for_editor(&state).unwrap().layout(w, h);
    state.editor_ui.mobile_sheet = Some(MobileSheetKind::Ai);
    let reader = WorksReader::for_editor(&state).unwrap();
    let open = reader.layout(w, h);
    let sheet = tablet_chat_rect(ReaderForm::TabletPortrait, w, h, h);
    assert!(open.stage.size.y < closed.stage.size.y);
    assert!(bottom(open.stage) <= sheet.origin.y);
    assert!(sheet.size.x >= 44.0 && sheet.size.y >= 280.0);
    assert!(open.strip.is_none() && open.thumbs.is_empty());
    assert_eq!(open.edit_page, Rect::ZERO);
    assert_eq!(
        reader.hit_test_layout(&open, Point2D::new(w / 2.0, h - 60.0)),
        None
    );
    // A raised keyboard shortens the sheet down to its floor, then lifts
    // it over the stage rather than under the keyboard.
    let lifted = tablet_chat_rect(ReaderForm::TabletPortrait, w, h, h - 700.0);
    assert!(bottom(lifted) <= h - 700.0);
    assert!(lifted.size.y >= 280.0 - 0.5);
    assert!(lifted.origin.y >= READER_TABLET_HEADER_H);
}

#[test]
fn tile_width_follows_the_board_aspect() {
    assert!(thumb_w_for(16.0 / 9.0) > thumb_w_for(1.0));
    assert!(
        thumb_w_for(375.0 / 812.0) >= 44.0,
        "phone screens keep the floor"
    );
    assert!(thumb_w_for(10.0) <= 160.0);
    assert_eq!(thumb_w_for(f32::NAN), thumb_w_for(16.0 / 9.0));
    let tile = Rect::xywh(0.0, 0.0, 120.0, 84.0);
    assert_eq!(thumb_plate(tile).size.y, 84.0 - THUMB_LABEL_H);
}
