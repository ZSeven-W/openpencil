//! APP MODE (routed multi-screen) + multi-page projection preview
//! tests — active-page projection on `enter`, screen projection into
//! APP MODE, the per-frame `reconcile` screen-switch, rejected-nav
//! warnings, and the workbench-mode centering guard.
//!
//! Split out of `preview/tests.rs` to keep both test files under the
//! repo's 800-line-per-file cap. The single-root workbench interaction
//! tests stay in `tests.rs`; the multi-root/multi-page projection
//! family lives here alongside APP MODE (which generalizes it).

#![cfg(test)]

use super::PreviewSession;
use jian_core::action::services::Router;
use jian_core::widget_state::WidgetState;

/// The default (empty) active-theme map — resolution falls back to the
/// document's default theme (first value per axis).
fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

/// A two-page document, each page carrying its own switch — drives the
/// active-page projection (jian's loader otherwise always uses page 0).
fn two_page_switch_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "pages": [
            { "id": "p0", "name": "P0", "children": [
                { "type": "frame", "id": "screen0", "width": 200, "height": 200, "children": [
                    { "type": "switch", "id": "sw0", "x": 20, "y": 20, "width": 44, "height": 24 }
                ] }
            ] },
            { "id": "p1", "name": "P1", "children": [
                { "type": "frame", "id": "screen1", "width": 200, "height": 200, "children": [
                    { "type": "switch", "id": "sw1", "x": 20, "y": 20, "width": 44, "height": 24 }
                ] }
            ] }
        ]
    }"##;
    let loaded = jian_ops_schema::load_str(src).expect("parse two-page doc");
    loaded.value
}

#[test]
fn preview_uses_active_page_for_runtime() {
    // BLOCK fix: entering preview on page 1 must build the runtime from
    // PAGE 1's roots — jian's loader defaults to page 0, so without the
    // active-page projection the page-1 widget would be absent from the
    // runtime (no hit-test, no state) while the scene painted page 1.
    let doc = two_page_switch_doc();
    let mut session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 1)
        .expect("enter preview on page 1");

    let rect = session.node_rect("sw1");
    assert!(
        rect.is_some(),
        "page-1 widget must be laid out (runtime built from the active page)"
    );
    assert!(
        session.node_rect("sw0").is_none(),
        "page-0 widget must be absent when previewing page 1"
    );

    let (x, y, w, h) = rect.unwrap();
    assert!(
        session.dispatch_tap(x + w / 2.0, y + h / 2.0),
        "tap should hit the page-1 switch"
    );
    match session.runtime().widget_states.get("sw1") {
        Some(WidgetState::Toggle { on }) => assert!(*on, "page-1 switch should toggle on"),
        other => panic!("expected Toggle state for sw1, got {other:?}"),
    }
}

/// A marked multi-screen document in EDITOR shape: one canvas page
/// carrying two top-level frames, each with a `screen` marker and their
/// authored canvas `x`/`y` (as the designer laid them out side by side).
/// `project_screens` runs INSIDE `enter`, so this fixture is exactly
/// what the editor would save — projection has not happened yet.
const TWO_SCREEN_DOC_JSON: &str = r##"{
    "version": "1.1",
    "formatVersion": "1.1",
    "id": "x",
    "app": { "name": "x", "version": "1", "id": "x" },
    "pages": [
        { "id": "canvas", "name": "Canvas", "children": [
            { "type": "frame", "id": "home", "screen": "/",
              "x": 0, "y": 0, "width": 200, "height": 200,
              "children": [
                  { "type": "switch", "id": "sw-home", "x": 20, "y": 20, "width": 44, "height": 24 },
                  { "type": "frame", "id": "go", "x": 20, "y": 60, "width": 120, "height": 40,
                    "semantics": { "role": "button" },
                    "events": { "onTap": [ { "push": "\"/detail\"" } ] } }
              ] },
            { "type": "frame", "id": "detail", "screen": "/detail",
              "x": 500, "y": 0, "width": 200, "height": 200,
              "children": [
                  { "type": "switch", "id": "sw-detail", "x": 20, "y": 20, "width": 44, "height": 24 }
              ] }
        ] }
    ]
}"##;

/// One page with two plain (unmarked) top-level frames — the classic
/// workbench shape. No `screen` marker anywhere, so `project_screens`
/// must return `None` and `enter` must keep today's active-page
/// workbench behavior (both frames mount as roots).
const UNMARKED_TWO_FRAME_PAGE_JSON: &str = r##"{
    "version": "1.1",
    "formatVersion": "1.1",
    "id": "x",
    "app": { "name": "x", "version": "1", "id": "x" },
    "pages": [
        { "id": "p0", "name": "P0", "children": [
            { "type": "frame", "id": "a", "width": 100, "height": 100 },
            { "type": "frame", "id": "b", "x": 200, "width": 100, "height": 100 }
        ] }
    ]
}"##;

#[test]
fn marked_doc_enters_app_mode_mounting_entry_screen() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(TWO_SCREEN_DOC_JSON).unwrap();
    let session = PreviewSession::enter(&doc, (1200.0, 800.0), &Default::default(), 0).unwrap();
    assert!(session.is_app_mode());
    // Entry screen only: one mounted root.
    assert_eq!(session.root_frames_len_for_test(), 1);
}

#[test]
fn unmarked_doc_keeps_workbench_mode() {
    let doc: jian_ops_schema::PenDocument =
        serde_json::from_str(UNMARKED_TWO_FRAME_PAGE_JSON).unwrap();
    let session = PreviewSession::enter(&doc, (1200.0, 800.0), &Default::default(), 0).unwrap();
    assert!(!session.is_app_mode());
    // Both top-level frames of the active page mount side by side.
    assert_eq!(session.root_frames_len_for_test(), 2);
}

#[test]
fn unmarked_doc_reports_no_screen_rect_so_centering_is_skipped() {
    // Regression guard: `current_screen_scene_rect` is gated on APP
    // MODE, so a classic workbench-mode session returns `None` even
    // though `root_frames` is populated. Both host call sites
    // (`enter_preview` centering + `reconcile` re-centering) key off
    // this `Some`, so a `None` here means entering Preview on an
    // ordinary unmarked document NEVER recenters the viewport —
    // preserving the "no behavior change for unmarked docs" invariant.
    let doc: jian_ops_schema::PenDocument =
        serde_json::from_str(UNMARKED_TWO_FRAME_PAGE_JSON).unwrap();
    let session = PreviewSession::enter(&doc, (1200.0, 800.0), &Default::default(), 0).unwrap();
    assert!(!session.is_app_mode());
    assert!(
        session.root_frames_len_for_test() > 0,
        "workbench roots are still mounted",
    );
    assert!(
        session.current_screen_scene_rect().is_none(),
        "workbench mode must report no screen rect so no centering fires",
    );
}

/// Locate the "go" nav button's painted centre (SCENE space). `home`
/// (its root) is authored at `(0, 0)` in `TWO_SCREEN_DOC_JSON`, so scene
/// space == the runtime's root-relative space here — mirrors the
/// `node_rect` lookup pattern in `overlay_reflects_widget_toggle_on_tap`.
fn go_button_center_for_test(session: &PreviewSession) -> (f32, f32) {
    let (x, y, w, h) = session.node_rect("go").expect("go button laid out");
    (x + w / 2.0, y + h / 2.0)
}

#[test]
fn tap_push_switches_screen_via_reconcile() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(TWO_SCREEN_DOC_JSON).unwrap();
    let mut session = PreviewSession::enter(&doc, (1200.0, 800.0), &Default::default(), 0).unwrap();
    // Tap the nav button center (scene coords; screens sit at origin in
    // app mode).
    let (bx, by) = go_button_center_for_test(&session);
    session.dispatch_tap(bx, by);
    assert!(session.reconcile(), "push must reconcile into a switch");
    assert!(session.is_app_mode());
    // The mounted screen is now /detail: entry-screen button gone.
    assert_eq!(session.current_path_for_test(), "/detail");
}

#[test]
fn unknown_push_appends_warning_and_stays() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(TWO_SCREEN_DOC_JSON).unwrap();
    let mut session = PreviewSession::enter(&doc, (1200.0, 800.0), &Default::default(), 0).unwrap();
    session.router_for_test().push("/missing");
    let before = session.warnings().len();
    assert!(session.reconcile(), "rejection appends a warning");
    assert_eq!(session.current_path_for_test(), "/");
    assert!(session.warnings().len() > before);
}
