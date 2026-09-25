//! FFI-level tests for the tablet works reader: a session created at a
//! tablet size takes touch chrome and the shell-derived size class, a
//! finished run lands in the tablet reader (never the professional canvas
//! with its rails), the reader answers taps through the full
//! `op_editor_press_at` chain, and a rotation swaps its form.

use crate::desc::{Callbacks, CreateOptions};
use crate::lifecycle::{OpEngine, Session};
use crate::{op_editor_press_at, op_editor_release_at, OpStatus};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{HomeFamily, TaskDraft, WorkspacePhase};
use op_editor_ui::widgets::works_reader::ReaderForm;
use op_editor_ui::widgets::{ReaderLayout, WorksReader};
use op_editor_ui::Rect;

const IPAD_LANDSCAPE: (f32, f32) = (1366.0, 1024.0);
const IPAD_PORTRAIT: (f32, f32) = (1024.0, 1366.0);

fn deck_document(count: usize) -> String {
    let children: Vec<String> = (0..count)
        .map(|i| {
            format!(
                r#"{{ "type": "frame", "id": "b{i}", "name": "Slide {n}", "x": {x}, "y": 0,
                     "width": 1920, "height": 1080, "children": [] }}"#,
                n = i + 1,
                x = i * 2100
            )
        })
        .collect();
    format!(
        r#"{{ "version": "1.0.0", "children": [{}] }}"#,
        children.join(",")
    )
}

/// A cold editor session at a tablet size: the lifecycle alone decides
/// touch chrome and the size class, exactly as on device.
fn tablet_engine((w, h): (f32, f32)) -> OpEngine {
    OpEngine::new(
        Session::new(CreateOptions {
            document: deck_document(3),
            width: w,
            height: h,
            dpr: 2.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session"),
    )
}

/// Home's send opened the workspace and the run finished with boards:
/// the same edges the mobile chat pump drives (`stamp_run_epoch` →
/// `settle_finished_run`).
fn finish_a_run(engine: &mut OpEngine) {
    let session = engine.session_mut_for_test();
    let viewport = session.editor_viewport();
    let host = session.editor_mut().expect("editor host");
    host.editor_state_mut()
        .editor_ui
        .open_workspace_for_generation(
            HomeFamily::Presentation,
            "A product intro deck",
            TaskDraft::default(),
            7,
            1_000,
            None,
        );
    assert_eq!(
        host.editor_state().editor_ui.workspace.phase,
        WorkspacePhase::Generating
    );
    assert!(crate::editor_chat_workspace::settle_finished_run(
        host, viewport
    ));
}

fn reader_layout(engine: &mut OpEngine) -> ReaderLayout {
    let session = engine.session_mut_for_test();
    let (w, h) = session.editor_viewport();
    let host = session.editor().expect("editor host");
    WorksReader::for_editor(host.editor_state())
        .expect("reader up")
        .layout(w, h)
}

fn tap(engine: &mut OpEngine, rect: Rect, at_ms: u64) {
    let (x, y) = (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    );
    let pointer = engine as *mut OpEngine;
    assert_eq!(
        unsafe { op_editor_press_at(pointer, x, y, at_ms) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release_at(pointer, x, y, at_ms + 40) },
        OpStatus::Ok
    );
}

#[test]
fn a_tablet_session_takes_touch_chrome_and_its_size_class() {
    for ((w, h), class) in [
        (IPAD_LANDSCAPE, EditorSizeClass::Expanded),
        (IPAD_PORTRAIT, EditorSizeClass::Expanded),
        ((800.0, 1280.0), EditorSizeClass::Medium),
    ] {
        let mut engine = tablet_engine((w, h));
        let host = engine.session_mut_for_test().editor().expect("editor");
        let ui = &host.editor_state().editor_ui;
        assert!(ui.touch_chrome(), "{w}×{h}");
        assert_eq!(ui.size_class, class, "{w}×{h}");
        assert!(!ui.compact_layout(), "{w}×{h}: not the phone");
    }
}

#[test]
fn a_tablet_engine_shows_the_reader_after_a_run() {
    let mut engine = tablet_engine(IPAD_LANDSCAPE);
    finish_a_run(&mut engine);
    {
        let session = engine.session_mut_for_test();
        let (w, h) = session.editor_viewport();
        let host = session.editor().expect("editor host");
        assert_eq!(
            host.editor_state().editor_ui.workspace.phase,
            WorkspacePhase::Done
        );
        assert!(host.works_reader_visible(), "the run reads in the reader");
        assert!(!host.workspace_visible(), "never the desktop workspace");
        assert!(
            !host.editor_state().editor_ui.expanded_touch_layout(),
            "no editing rails beside the reader"
        );
        let layout = WorksReader::for_editor(host.editor_state())
            .unwrap()
            .layout(w, h);
        assert_eq!(layout.form, ReaderForm::TabletLandscape);
        assert!(layout.side_panel.is_some());
        let stage = layout.stage;
        assert_eq!(
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(host.editor_state(), w, h),
            (stage.origin.x, stage.origin.y, stage.size.x, stage.size.y),
            "the canvas paints into the reader's stage"
        );
    }

    // The strip pages through the full FFI press chain.
    let (_, second) = reader_layout(&mut engine).thumbs[1];
    tap(&mut engine, second, 2_000);
    let host = engine.session_mut_for_test().editor().unwrap();
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 1);

    // 专业 in the reader header is not swallowed by the hidden app bar's
    // collaboration undo / redo seam that sits in the same top strip.
    let professional = reader_layout(&mut engine).mode_professional;
    tap(&mut engine, professional, 3_000);
    let host = engine.session_mut_for_test().editor().unwrap();
    assert!(!host.works_reader_visible(), "专业 opens the canvas");
    assert!(host.editor_state().editor_ui.expanded_touch_layout());
}

#[test]
fn rotating_a_tablet_swaps_the_reader_form() {
    let mut engine = tablet_engine(IPAD_LANDSCAPE);
    finish_a_run(&mut engine);
    assert_eq!(reader_layout(&mut engine).form, ReaderForm::TabletLandscape);
    let session = engine.session_mut_for_test();
    session
        .resize(IPAD_PORTRAIT.0, IPAD_PORTRAIT.1, 2.0)
        .expect("rotate");
    assert_eq!(
        session
            .editor()
            .unwrap()
            .editor_state()
            .editor_ui
            .size_class,
        EditorSizeClass::Expanded,
        "a 12.9-inch iPad stays Expanded: only the orientation decides"
    );
    let layout = reader_layout(&mut engine);
    assert_eq!(layout.form, ReaderForm::TabletPortrait);
    assert!(layout.side_panel.is_none() && layout.bottom_bar.size.y > 0.0);
}
