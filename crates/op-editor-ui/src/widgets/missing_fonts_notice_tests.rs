use super::*;
use crate::widgets::missing_fonts_flow;
use op_editor_core::{EditorCommand, Locale};

fn missing_work() -> EditorState {
    let doc = serde_json::from_value(serde_json::json!({"version":"1.0.0","children":[
        {"type":"text","id":"title","content":"咖啡首页","fontFamily":"QA Missing Face"}
    ]}))
    .unwrap();
    let mut state = EditorState::from_document(doc);
    state.editor_ui.workspace.visible = true;
    state.editor_ui.sidebar_open = false;
    state.editor_ui.system_fonts_loaded = true;
    state.editor_ui.system_font_families = std::sync::Arc::new(vec!["Inter".into()]);
    state.editor_ui.locale = Locale::ZhCn;
    missing_fonts_flow::detect_for_document(&mut state);
    state
}
fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn normal_load_preserves_work_and_only_explicit_manage_opens_the_modal() {
    let mut state = missing_work();
    let document = state.doc.clone();
    assert!(!state.editor_ui.missing_fonts_modal_open);
    let rect = MissingFontsNotice::for_editor(&state)
        .unwrap()
        .rect(390.0, 844.0)
        .unwrap();
    assert!(!press(&mut state, Point2D::new(8.0, 720.0), 390.0, 844.0));
    assert_eq!(state.doc, document);
    assert!(press(
        &mut state,
        center(MissingFontsNotice::manage_rect(rect)),
        390.0,
        844.0
    ));
    assert!(state.editor_ui.missing_fonts_modal_open);
    assert!(crate::widgets::MissingFontsPanel::for_editor(&state).is_some());
    assert!(MissingFontsNotice::for_editor(&state).is_none());
    assert_eq!(state.doc, document);
}

#[test]
fn dismissal_survives_same_family_refresh_but_new_missing_family_is_announced() {
    let mut state = missing_work();
    let rect = MissingFontsNotice::for_editor(&state)
        .unwrap()
        .rect(390.0, 844.0)
        .unwrap();
    assert!(press(
        &mut state,
        center(MissingFontsNotice::dismiss_rect(rect)),
        390.0,
        844.0
    ));
    missing_fonts_flow::detect_for_document(&mut state);
    assert!(MissingFontsNotice::for_editor(&state).is_none());
    state.apply(EditorCommand::ReplaceFontFamily {
        from: "QA Missing Face".into(),
        to: "Another QA Missing Face".into(),
    });
    missing_fonts_flow::refresh_prompt(&mut state);
    assert!(MissingFontsNotice::for_editor(&state).is_some());
    state.editor_ui.imported_font_families =
        std::sync::Arc::new(vec!["Another QA Missing Face".into()]);
    missing_fonts_flow::refresh_prompt(&mut state);
    assert!(state.editor_ui.missing_fonts_prompt.is_none());
    assert!(MissingFontsNotice::for_editor(&state).is_none());
}

#[test]
fn deferred_web_scan_uses_the_current_mode_instead_of_blocking_a_new_reader() {
    let mut state = missing_work();
    state.editor_ui.workspace.visible = false;
    state.editor_ui.home.visible = false;
    state.editor_ui.system_fonts_loaded = false;
    missing_fonts_flow::arm_pending_detection(&mut state, true);
    assert!(!missing_fonts_flow::complete_pending_detection(&mut state));
    state.editor_ui.workspace.visible = true;
    state.editor_ui.system_fonts_loaded = true;
    assert!(missing_fonts_flow::complete_pending_detection(&mut state));
    assert!(!state.editor_ui.missing_fonts_modal_open);
    assert!(MissingFontsNotice::for_editor(&state).is_some());
    state.editor_ui.workspace.visible = false;
    missing_fonts_flow::detect_for_document(&mut state);
    assert!(
        state.editor_ui.missing_fonts_modal_open,
        "professional loads retain detailed diagnostics"
    );
}

#[test]
fn controls_stay_inside_the_notice_across_phone_tablet_and_desktop() {
    for (width, height) in [
        (320.0, 640.0),
        (390.0, 844.0),
        (768.0, 1024.0),
        (1440.0, 900.0),
    ] {
        let mut state = missing_work();
        state.editor_ui.touch = width <= 1024.0;
        state.editor_ui.size_class = op_editor_core::size_class::size_class(width, height);
        state.editor_ui.workspace.sync_drawer_mode(width);
        let notice = MissingFontsNotice::for_editor(&state).unwrap();
        let rect = notice.rect(width, height).unwrap();
        assert!(rect.origin.x >= 0.0 && rect.origin.x + rect.size.x <= width);
        for control in [
            MissingFontsNotice::manage_rect(rect),
            MissingFontsNotice::dismiss_rect(rect),
        ] {
            assert!(control.size.y >= 44.0);
            assert!(control.origin.x >= rect.origin.x);
            assert!(control.origin.x + control.size.x <= rect.origin.x + rect.size.x);
        }
    }
}

#[test]
fn settings_and_home_do_not_show_an_artwork_notice() {
    let mut state = missing_work();
    state.editor_ui.agent_settings_open = true;
    assert!(MissingFontsNotice::for_editor(&state).is_none());
    state.editor_ui.agent_settings_open = false;
    state.editor_ui.workspace.visible = false;
    state.editor_ui.home.visible = true;
    missing_fonts_flow::detect_for_document(&mut state);
    assert!(!state.editor_ui.missing_fonts_modal_open);
    assert!(MissingFontsNotice::for_editor(&state).is_none());
}

#[test]
fn localized_text_and_controls_do_not_overlap_on_a_small_phone() {
    use crate::widgets::test_family_gap_backend::FamilyGapBackend;
    for locale in op_i18n::Locale::ALL {
        let mut state = missing_work();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = op_editor_core::size_class::size_class(320.0, 640.0);
        state.editor_ui.locale = locale;
        let notice = MissingFontsNotice::for_editor(&state).unwrap();
        let rect = notice.rect(320.0, 640.0).unwrap();
        let manage = MissingFontsNotice::manage_rect(rect);
        let mut backend = FamilyGapBackend::weighted();
        notice.paint(
            &mut PaintCx {
                backend: &mut backend,
            },
            rect,
        );
        assert_eq!(backend.runs.len(), 2);
        assert!(backend.runs[0].right_edge() <= manage.origin.x - 7.9);
        assert!(backend.runs[1].right_edge() <= manage.origin.x + manage.size.x - 7.9);
        assert!(backend.runs[1].origin.x >= manage.origin.x + 7.9);
    }
}

#[test]
fn transient_toast_is_placed_below_the_font_notice() {
    use crate::widgets::test_family_gap_backend::FamilyGapBackend;
    let mut state = missing_work();
    state.editor_ui.show_toast(
        "missingFonts.noneMissing",
        vec![],
        op_editor_core::editor_toast::EditorToastLevel::Info,
        0,
    );
    let notice = MissingFontsNotice::for_editor(&state)
        .unwrap()
        .rect(1440.0, 900.0)
        .unwrap();
    let mut backend = FamilyGapBackend::weighted();
    let (_, toast) = crate::widgets::editor_toast_flow::toast_rect(
        &mut PaintCx {
            backend: &mut backend,
        },
        &state,
        1440.0,
        900.0,
        0,
    )
    .unwrap();
    assert!(toast.origin.y > notice.origin.y + notice.size.y);
}
