use super::op_editor_back;
use crate::desc::{Callbacks, CreateOptions};
use crate::lifecycle::{OpEngine, Session};
use crate::OpStatus;
use op_editor_core::agent_settings::SettingsFocus;
use op_editor_core::size_class::MobileSheetKind;
use op_editor_core::{EntrySurface, NodeId};

fn phone_engine() -> OpEngine {
    OpEngine::new(
        Session::new(CreateOptions {
            document: r#"{"version":"1.0.0","children":[{"type":"frame","id":"board","width":390,"height":600,"children":[]}]}"#.into(),
            width: 390.0,
            height: 844.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("phone editor"),
    )
}

fn back(engine: &mut OpEngine) -> bool {
    let mut consumed = false;
    assert_eq!(
        unsafe { op_editor_back(engine, &mut consumed) },
        OpStatus::Ok
    );
    consumed
}

#[test]
fn system_back_at_home_root_is_not_consumed() {
    let mut engine = phone_engine();
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    host.editor_state_mut().editor_ui.home.visible = true;
    assert!(!back(&mut engine));
    assert!(!back(&mut engine));
}

#[test]
fn system_back_blurs_home_composer_without_losing_the_draft() {
    let mut engine = phone_engine();
    let ui = &mut engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state_mut()
        .editor_ui;
    ui.home.visible = true;
    ui.home.composer_focused = true;
    ui.home.set_draft("为社区做一张活动海报");

    assert!(back(&mut engine));
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    assert!(!host.text_input_focus_active());
    assert_eq!(
        host.editor_state().editor_ui.home.input.text(),
        "为社区做一张活动海报"
    );
    assert!(!back(&mut engine));
}

#[test]
fn system_back_closes_settings_focus_then_modal_then_returns_to_works() {
    let mut engine = phone_engine();
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    host.open_current_work_in_reader(390.0, 844.0);
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.agent_settings_open = true;
    ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    ui.settings_input.set_text("4321");

    assert!(back(&mut engine));
    let ui = &engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .editor_ui;
    assert!(ui.agent_settings_open);
    assert!(ui.agent_settings.focus.is_none());
    assert!(!ui.home.visible);

    assert!(back(&mut engine));
    let ui = &engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .editor_ui;
    assert!(!ui.agent_settings_open);
    assert!(!ui.home.visible);

    assert!(back(&mut engine));
    let state = engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state();
    assert!(state.editor_ui.home.visible);
    assert!(state.editor_ui.home.works_open);
    assert!(state.editor_ui.workspace.active);
    assert_eq!(state.editor_ui.entry_surface, EntrySurface::Home);
    assert_eq!(state.active_children().len(), 1);
    assert!(!back(&mut engine));
}

#[test]
fn system_back_closes_mobile_sheet_before_canvas_selection() {
    for kind in [
        MobileSheetKind::More,
        MobileSheetKind::Layers,
        MobileSheetKind::Properties,
        MobileSheetKind::Ai,
    ] {
        let mut engine = phone_engine();
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        let state = host.editor_state_mut();
        state.editor_ui.home.visible = false;
        state.editor_ui.entry_surface = EntrySurface::Canvas;
        state.editor_ui.mobile_sheet = Some(kind);
        state.chat.focused = false;
        state.set_single_selection(NodeId::new("board"));

        assert!(back(&mut engine));
        let state = engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state();
        assert!(state.editor_ui.mobile_sheet.is_none());
        assert!(!state.selection.is_empty());
        assert!(!state.chat.focused);
        if kind == MobileSheetKind::Ai {
            assert!(state.chat.collapsed);
        }
    }
}

#[test]
fn system_back_blurs_chat_before_closing_its_sheet() {
    let mut engine = phone_engine();
    let state = engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state_mut();
    state.editor_ui.home.visible = false;
    state.editor_ui.mobile_sheet = Some(MobileSheetKind::Ai);
    state.chat.focused = true;
    state.chat.input.set_text("把标题放大一点");

    assert!(back(&mut engine));
    let state = engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state();
    assert!(!state.chat.focused);
    assert_eq!(state.editor_ui.mobile_sheet, Some(MobileSheetKind::Ai));
    assert_eq!(state.chat.input.text(), "把标题放大一点");
    assert!(back(&mut engine));
    assert!(engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .editor_ui
        .mobile_sheet
        .is_none());
}

#[test]
fn system_back_rejects_null_output_without_dismissing_settings() {
    let mut engine = phone_engine();
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state_mut()
        .editor_ui
        .agent_settings_open = true;
    assert_eq!(
        unsafe { op_editor_back(&mut engine, std::ptr::null_mut()) },
        OpStatus::InvalidArg
    );
    assert!(
        engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui
            .agent_settings_open
    );
}

#[test]
fn system_back_c_and_android_bridge_preserve_the_consumed_contract() {
    let _: unsafe extern "C" fn(*mut OpEngine, *mut bool) -> OpStatus = op_editor_back;
    assert!(include_str!("../include/op_engine.h")
        .contains("OpStatus op_editor_back(OpEngine *engine, bool *consumed);"));
    assert!(include_str!(
        "../../../packaging/android/app/src/main/kotlin/tech/zseven/openpencil/OpNative.kt"
    )
    .contains("external fun nativeEditorBack(engine: Long): Boolean"));
    let activity = include_str!(
        "../../../packaging/android/app/src/main/kotlin/tech/zseven/openpencil/MainActivity.kt"
    );
    let editor_back = activity
        .find("if (surfaceView.handleSystemBack()) return")
        .unwrap();
    let login_back = activity.find("loginBackCallback = object").unwrap();
    assert!(
        editor_back < login_back,
        "native account and login callbacks must be registered last"
    );
    assert!(activity[editor_back..login_back].contains("onBackPressedDispatcher.onBackPressed()"));
}
