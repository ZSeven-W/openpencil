//! P0 regression: the compact phone Home top bar's 专业 segment must
//! survive the full FFI press chain.
//!
//! The mobile shells deliver every tap through `op_editor_press_at`, whose
//! ladder consults off-surface seams BEFORE the widget host sees the point.
//! With the Home takeover up, the canvas mobile app bar is not painted, yet
//! its geometry was still hit-tested here — so a tap on 专业 landed on the
//! app bar's ghost undo/redo buttons (same top strip, x ∈ [264, 352] at a
//! 402 pt viewport) and was consumed as collaboration history, while the
//! gear (x ∈ [352, 392], the overflow slot the seam does not claim) fell
//! through and worked. Both sides of that asymmetry are pinned end to end.

use crate::desc::{Callbacks, CreateOptions};
use crate::lifecycle::{OpEngine, Session};
use crate::{op_editor_press_at, OpStatus};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::EntrySurface;
use op_editor_ui::widgets::HomeSurface;
use op_editor_ui::Rect;

const PHONE: (f32, f32) = (402.0, 874.0);
const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

/// Cold compact-phone session with the Home takeover already up — the
/// state the device report starts from.
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

fn segment_center(
    engine: &mut OpEngine,
    segment: impl FnOnce(&op_editor_ui::widgets::HomeLayout) -> Rect,
) -> (f32, f32) {
    let session = engine.session_mut_for_test();
    let (w, h) = session.editor_viewport();
    let host = session.editor().expect("editor host");
    let home = HomeSurface::for_editor_at(host.editor_state(), session.now_ms).expect("home up");
    let rect = segment(&home.layout(w, h));
    (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn ffi_press_on_professional_enters_the_canvas() {
    let mut engine = compact_home_engine();
    let pointer = &mut engine as *mut OpEngine;
    let (x, y) = segment_center(&mut engine, |layout| layout.professional);

    assert_eq!(
        unsafe { op_editor_press_at(pointer, x, y, 1_000) },
        OpStatus::Ok
    );

    let ui = &engine
        .session_mut_for_test()
        .editor()
        .expect("editor host")
        .editor_state()
        .editor_ui;
    assert_eq!(ui.entry_surface, EntrySurface::Canvas);
    assert!(!ui.home.visible, "专业 must tear down the Home takeover");
}

#[test]
fn ffi_press_on_the_top_bar_gear_still_opens_settings() {
    let mut engine = compact_home_engine();
    let pointer = &mut engine as *mut OpEngine;
    let (x, y) = segment_center(&mut engine, |layout| layout.settings);

    assert_eq!(
        unsafe { op_editor_press_at(pointer, x, y, 1_000) },
        OpStatus::Ok
    );

    let ui = &engine
        .session_mut_for_test()
        .editor()
        .expect("editor host")
        .editor_state()
        .editor_ui;
    assert!(ui.agent_settings_open);
    assert!(ui.home.visible, "the gear keeps Home up");
}
