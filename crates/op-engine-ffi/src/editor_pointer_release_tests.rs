//! ABI regression coverage for slideshow flicks whose final travel exists
//! only in the pointer-up event.

use crate::desc::{Callbacks, CreateOptions};
use crate::lifecycle::{OpEngine, Session};
use crate::{op_editor_press, op_editor_release, op_pointer, OpStatus};
use op_editor_core::preview_slideshow::SlideshowToolbarButton;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_ui::widgets::SlideshowToolbar;
use op_editor_ui::{Point2D, Rect};

const SLIDE_DECK: &str = include_str!("../../op-editor-core/assets/scene_templates/slide-deck.op");
const START: (f32, f32) = (260.0, 220.0);

fn slideshow_engine() -> OpEngine {
    let mut engine = OpEngine::new(
        Session::new(CreateOptions {
            document: SLIDE_DECK.to_owned(),
            width: 390.0,
            height: 844.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session"),
    );
    let session = engine.session_mut_for_test();
    let viewport = session.editor_viewport();
    let host = session.editor_mut().expect("editor host");
    host.editor_state_mut().editor_ui.scenario = Some(TemplateScene::Slides);
    host.editor_state_mut().editor_ui.touch = true;
    host.editor_state_mut().editor_ui.size_class = EditorSizeClass::Compact;
    assert!(host.enter_preview(viewport));
    assert!(host.preview_slideshow_active());
    engine
}

fn slide_index(engine: &mut OpEngine) -> usize {
    engine
        .session_mut_for_test()
        .editor()
        .expect("editor host")
        .editor_state()
        .preview_slideshow()
        .expect("slideshow")
        .index()
}

fn toolbar_point(engine: &mut OpEngine, button: SlideshowToolbarButton) -> (f32, f32) {
    let session = engine.session_mut_for_test();
    let viewport = session.editor_viewport();
    let label = session
        .editor()
        .expect("editor host")
        .editor_state()
        .preview_slideshow()
        .expect("slideshow")
        .counter_label();
    // Presenting touch layouts own the complete safe-area-local viewport.
    let canvas = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(viewport.0, viewport.1),
    };
    let rect = SlideshowToolbar::button_rects(canvas, &label)
        .into_iter()
        .find(|(candidate, _)| *candidate == button)
        .expect("toolbar button")
        .1;
    (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn dedicated_editor_release_uses_up_position_without_a_move() {
    let mut engine = slideshow_engine();
    let pointer = &mut engine as *mut OpEngine;

    assert_eq!(
        unsafe { op_editor_press(pointer, START.0, START.1) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release(pointer, START.0 - 100.0, START.1 + 8.0) },
        OpStatus::Ok
    );
    assert_eq!(slide_index(&mut engine), 1, "left flick advances");

    assert_eq!(
        unsafe { op_editor_press(pointer, START.0, START.1) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release(pointer, START.0 + 100.0, START.1 - 8.0) },
        OpStatus::Ok
    );
    assert_eq!(slide_index(&mut engine), 0, "right flick goes back");

    // Down + Up at one point remains the existing tap-to-advance gesture.
    assert_eq!(
        unsafe { op_editor_press(pointer, START.0, START.1) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release(pointer, START.0, START.1) },
        OpStatus::Ok
    );
    assert_eq!(slide_index(&mut engine), 1, "a tap still advances");
}

#[test]
fn generic_pointer_up_uses_its_endpoint_without_a_move() {
    let mut engine = slideshow_engine();
    let pointer = &mut engine as *mut OpEngine;

    assert_eq!(
        unsafe { op_pointer(pointer, 7, 0, START.0, START.1, 1) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(pointer, 7, 2, START.0 - 100.0, START.1 + 8.0, 2) },
        OpStatus::Ok
    );
    assert_eq!(slide_index(&mut engine), 1, "left flick advances");

    assert_eq!(
        unsafe { op_pointer(pointer, 8, 0, START.0, START.1, 3) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_pointer(pointer, 8, 2, START.0 + 100.0, START.1 - 8.0, 4) },
        OpStatus::Ok
    );
    assert_eq!(slide_index(&mut engine), 0, "right flick goes back");
}

#[test]
fn toolbar_release_uses_up_position_to_cancel_or_activate_exit() {
    let mut engine = slideshow_engine();
    let pointer = &mut engine as *mut OpEngine;
    let exit = toolbar_point(&mut engine, SlideshowToolbarButton::Exit);

    // A no-Move release on the board must cancel the Exit pressed on Down.
    assert_eq!(
        unsafe { op_editor_press(pointer, exit.0, exit.1) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release(pointer, START.0, START.1) },
        OpStatus::Ok
    );
    assert!(
        engine
            .session_mut_for_test()
            .editor()
            .expect("editor host")
            .preview_slideshow_active(),
        "releasing away from Exit cancels it"
    );

    assert_eq!(
        unsafe { op_editor_press(pointer, exit.0, exit.1) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { op_editor_release(pointer, exit.0, exit.1) },
        OpStatus::Ok
    );
    assert!(
        !engine
            .session_mut_for_test()
            .editor()
            .expect("editor host")
            .preview_slideshow_active(),
        "releasing on Exit activates it"
    );
}
