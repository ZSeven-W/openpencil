//! P1 regression: the compact phone Home's software keyboard follows the
//! composer through the full FFI chain.
//!
//! The iOS shell polls `op_editor_ime_focused` every frame and only calls
//! `imeFocusChanged` on a boolean flip, so the ABI bool itself must flip
//! when the composer gains focus and again when another Home target takes
//! the press — a latched `true` is exactly the "the keyboard never
//! dismisses" bug this pins.

use crate::desc::{Callbacks, CreateOptions};
use crate::lifecycle::{OpEngine, Session};
use crate::{op_editor_ime_focused, op_editor_press_at, op_editor_release_at, OpStatus};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::EntrySurface;
use op_editor_ui::widgets::HomeSurface;
use op_editor_ui::Rect;

const PHONE: (f32, f32) = (402.0, 874.0);
const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

/// Cold compact-phone session with the Home takeover already up.
fn compact_home_engine() -> OpEngine {
    let mut engine = OpEngine::new(
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: PHONE.0,
            height: PHONE.1,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session"),
    );
    let session = engine.session_mut_for_test();
    let host = session.editor_mut().expect("editor host");
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = true;
    ui.size_class = EditorSizeClass::Compact;
    ui.entry_surface = EntrySurface::Home;
    ui.home.visible = true;
    engine
}

fn target_center(
    engine: &mut OpEngine,
    target: impl FnOnce(&op_editor_ui::widgets::HomeLayout) -> Rect,
) -> (f32, f32) {
    let session = engine.session_mut_for_test();
    let (w, h) = session.editor_viewport();
    let host = session.editor().expect("editor host");
    let home = HomeSurface::for_editor_at(host.editor_state(), session.now_ms).expect("home up");
    let rect = target(&home.layout(w, h));
    (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn ime_focused(engine: &mut OpEngine) -> bool {
    let mut focused = false;
    assert_eq!(
        unsafe { op_editor_ime_focused(engine as *mut OpEngine, &mut focused) },
        OpStatus::Ok
    );
    focused
}

#[test]
fn ffi_home_ime_focus_flips_with_the_composer() {
    let mut engine = compact_home_engine();
    assert!(
        !ime_focused(&mut engine),
        "an untouched Home reports no IME focus, so no keyboard rises"
    );

    // Tap the composer's input box: the compact tap replays on release.
    let (x, y) = target_center(&mut engine, |layout| layout.input_box);
    let pointer = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { op_editor_press_at(pointer, x, y, 1_000) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release_at(pointer, x, y, 1_050) },
        OpStatus::Ok
    );
    assert!(
        ime_focused(&mut engine),
        "the composer's tap must raise the keyboard on the ABI"
    );

    // Tap a task-grid cell: the same ABI bool must flip back.
    let (x, y) = target_center(&mut engine, |layout| layout.tabs[1]);
    let pointer = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { op_editor_press_at(pointer, x, y, 1_100) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release_at(pointer, x, y, 1_150) },
        OpStatus::Ok
    );
    assert!(
        !ime_focused(&mut engine),
        "a task-grid tap must lower the keyboard on the ABI"
    );
    let host = engine.session_mut_for_test().editor().expect("editor host");
    assert!(
        host.editor_state().editor_ui.home.visible,
        "the grid tap keeps the Home takeover up"
    );
}
