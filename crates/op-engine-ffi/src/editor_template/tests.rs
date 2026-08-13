use super::*;
use crate::desc::{Callbacks, CreateOptions};
use crate::editor::op_editor_release;
use crate::lifecycle::{OpEngine, Session};
use op_editor_core::size_class::EditorSizeClass;

const SAMPLE_DOC: &str =
    include_str!("../../../op-editor-core/assets/scene_templates/daily-sign-card.op");

fn editor_engine_with(doc: &str) -> OpEngine {
    OpEngine::new(
        Session::new(CreateOptions {
            document: doc.to_owned(),
            width: 800.0,
            height: 600.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
        })
        .expect("editor session"),
    )
}

fn editor_engine() -> OpEngine {
    editor_engine_with(SAMPLE_DOC)
}

fn queue_template(engine: &mut OpEngine, id: &str) {
    engine
        .session_mut_for_test()
        .editor_mut()
        .expect("host")
        .editor_state_mut()
        .editor_ui
        .scene_template_center
        .request_open(id);
}

#[test]
fn release_drains_real_template_and_synchronizes_host_and_session() {
    let mut engine = editor_engine();
    let pointer = &mut engine as *mut OpEngine;
    let before_children = engine
        .session_mut_for_test()
        .editor()
        .unwrap()
        .editor_state()
        .active_children()
        .len();
    {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.touch = true;
        ui.size_class = EditorSizeClass::Medium;
        ui.theme_mode = op_editor_core::ThemeMode::Light;
        // A real More-menu selection closes the sheet before the template
        // center opens. Installation must preserve that live shell state.
        ui.mobile_sheet = None;
    }
    queue_template(&mut engine, "slide-deck");

    assert_eq!(
        unsafe { op_editor_release(pointer, 0.0, 0.0) },
        OpStatus::Ok
    );

    let session = engine.session_mut_for_test();
    let host = session.editor().unwrap();
    let live = host.editor_state();
    assert!(live.active_children().len() > before_children);
    assert_eq!(
        live.editor_ui.scenario,
        Some(op_editor_core::scene_template_catalog::TemplateScene::Slides),
        "an unclassified document adopts the template scenario"
    );
    assert!(live.editor_ui.touch);
    assert_eq!(live.editor_ui.size_class, EditorSizeClass::Medium);
    assert_eq!(live.editor_ui.theme_mode, op_editor_core::ThemeMode::Light);
    assert_eq!(live.editor_ui.mobile_sheet, None);
    assert!(live.is_dirty());
    assert_eq!(session.state.doc, live.doc);
    assert_eq!(session.state.editor_ui.scenario, live.editor_ui.scenario);
    assert_eq!(
        session.state.ui.active_page_index,
        live.ui.active_page_index
    );
    assert_eq!(session.state.viewport, live.viewport);
    assert_eq!(session.scene.active_page_index, live.ui.active_page_index);
}

#[test]
fn bad_template_id_reports_typed_error_without_changing_document() {
    let mut engine = editor_engine();
    let pointer = &mut engine as *mut OpEngine;
    let before = engine
        .session_mut_for_test()
        .editor()
        .unwrap()
        .editor_state()
        .doc
        .clone();
    queue_template(&mut engine, "not-a-shipped-template");

    assert_eq!(
        unsafe { op_editor_release(pointer, 0.0, 0.0) },
        OpStatus::BadDocument
    );

    let session = engine.session_mut_for_test();
    assert_eq!(session.editor().unwrap().editor_state().doc, before);
    assert_eq!(session.state.doc, before);
}

#[test]
fn untouched_starter_is_replaced_without_losing_mobile_shell_state() {
    let starter = serde_json::to_string(&op_editor_core::EditorState::starter().doc)
        .expect("serialize starter");
    let mut engine = editor_engine_with(&starter);
    let pointer = &mut engine as *mut OpEngine;
    let initial_epoch = {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        let epoch = host.document_epoch();
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.touch = true;
        ui.size_class = EditorSizeClass::Medium;
        ui.theme_mode = op_editor_core::ThemeMode::Light;
        epoch
    };
    queue_template(&mut engine, "slide-deck");

    assert_eq!(
        unsafe { op_editor_release(pointer, 0.0, 0.0) },
        OpStatus::Ok
    );

    let session = engine.session_mut_for_test();
    let host = session.editor().unwrap();
    let live = host.editor_state();
    assert!(!op_editor_core::blank_starter::active_page_is_blank_starter(live));
    assert_eq!(host.document_epoch(), initial_epoch.wrapping_add(1));
    assert!(live.editor_ui.touch);
    assert_eq!(live.editor_ui.size_class, EditorSizeClass::Medium);
    assert_eq!(live.editor_ui.theme_mode, op_editor_core::ThemeMode::Light);
    assert_eq!(
        live.editor_ui.scenario,
        Some(op_editor_core::scene_template_catalog::TemplateScene::Slides)
    );
    assert_eq!(session.state.doc, live.doc);
}
