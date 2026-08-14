use super::Session;
use crate::desc::{Callbacks, CreateOptions};
use crate::viewport::OpInsets;
use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
use op_editor_core::{FontPickerPurpose, PropertyFocus};

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

fn editor_session() -> Session {
    editor_session_at(834.0, 1_112.0)
}

fn editor_session_at(width: f32, height: f32) -> Session {
    Session::new(CreateOptions {
        document: SAMPLE_DOC.to_owned(),
        width,
        height,
        dpr: 1.0,
        callbacks: Callbacks::default(),
        asset_base: None,
        editor_mode: true,
    })
    .expect("editor session")
}

#[test]
fn responsive_resize_preserves_visible_property_and_releases_hidden_owners() {
    let mut property = editor_session();
    {
        let state = property.editor_mut().unwrap().editor_state_mut();
        assert_eq!(state.editor_ui.size_class, EditorSizeClass::Medium);
        state.editor_ui.mobile_sheet = Some(MobileSheetKind::Properties);
        state.ui.property_focus = Some(PropertyFocus::PositionX);
        state.ui.property_input.set_text("123");
        state.editor_ui.font_picker.open = true;
        state.editor_ui.font_picker_purpose = Some(FontPickerPurpose::PropertyText);
        state.editor_ui.image_panel.search_open = true;
    }
    property.resize(1_194.0, 834.0, 1.0).unwrap();
    let host = property.editor().unwrap();
    assert_eq!(
        host.editor_state().editor_ui.size_class,
        EditorSizeClass::Expanded
    );
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
    assert_eq!(
        host.editor_state().ui.property_focus,
        Some(PropertyFocus::PositionX)
    );
    assert!(host.editor_state().editor_ui.font_picker.open);
    assert!(host.editor_state().editor_ui.image_panel.search_open);
    assert!(host.text_input_focus_active());

    let mut ai = editor_session();
    {
        let state = ai.editor_mut().unwrap().editor_state_mut();
        state.editor_ui.mobile_sheet = Some(MobileSheetKind::Ai);
        state.chat.collapsed = false;
        state.chat.focus_input_at_end(0);
    }
    ai.resize(1_194.0, 834.0, 1.0).unwrap();
    let host = ai.editor().unwrap();
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
    assert!(!host.editor_state().chat.focused);
    assert!(host.editor_state().chat.collapsed);
    assert!(!host.text_input_focus_active());

    let mut covered_property = editor_session();
    covered_property.resize(1_194.0, 834.0, 1.0).unwrap();
    {
        let state = covered_property.editor_mut().unwrap().editor_state_mut();
        state.editor_ui.mobile_sheet = Some(MobileSheetKind::More);
        state.ui.property_focus = Some(PropertyFocus::SizeW);
        state.ui.property_input.set_text("456");
    }
    covered_property.resize(834.0, 1_112.0, 1.0).unwrap();
    let host = covered_property.editor().unwrap();
    assert_eq!(
        host.editor_state().editor_ui.size_class,
        EditorSizeClass::Medium
    );
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
    assert!(host.editor_state().ui.property_focus.is_none());
    assert!(!host.text_input_focus_active());
}

#[test]
fn atomic_resize_avoids_transient_size_class_surface_reset() {
    let old_insets = OpInsets {
        top: 0.0,
        right: 150.0,
        bottom: 0.0,
        left: 150.0,
    };
    let new_insets = OpInsets {
        top: 0.0,
        right: 20.0,
        bottom: 0.0,
        left: 20.0,
    };

    // Both stable tuples are Medium, while the new size paired with the old
    // insets is Compact. A split update observes that transient class and
    // irreversibly closes the open surface.
    let mut split = editor_session_at(1_000.0, 800.0);
    split
        .resize_with_safe_area(1_000.0, 800.0, 1.0, old_insets)
        .unwrap();
    {
        let state = split.editor_mut().unwrap().editor_state_mut();
        state.editor_ui.mobile_sheet = Some(MobileSheetKind::Properties);
        state.ui.property_focus = Some(PropertyFocus::SizeW);
        state.ui.property_input.set_text("512");
    }
    split.resize(800.0, 800.0, 1.0).unwrap();
    assert_eq!(
        split.editor().unwrap().editor_state().editor_ui.size_class,
        EditorSizeClass::Compact
    );
    assert_eq!(
        split
            .editor()
            .unwrap()
            .editor_state()
            .editor_ui
            .mobile_sheet,
        None
    );
    assert_eq!(
        split.editor().unwrap().editor_state().ui.property_focus,
        None
    );

    let mut atomic = editor_session_at(1_000.0, 800.0);
    atomic
        .resize_with_safe_area(1_000.0, 800.0, 1.0, old_insets)
        .unwrap();
    {
        let state = atomic.editor_mut().unwrap().editor_state_mut();
        state.editor_ui.mobile_sheet = Some(MobileSheetKind::Properties);
        state.ui.property_focus = Some(PropertyFocus::SizeW);
        state.ui.property_input.set_text("512");
    }
    atomic
        .resize_with_safe_area(800.0, 800.0, 1.0, new_insets)
        .unwrap();
    let ui = &atomic.editor().unwrap().editor_state().editor_ui;
    assert_eq!(ui.size_class, EditorSizeClass::Medium);
    assert_eq!(ui.mobile_sheet, Some(MobileSheetKind::Properties));
    assert_eq!(
        atomic.editor().unwrap().editor_state().ui.property_focus,
        Some(PropertyFocus::SizeW)
    );
}

#[test]
fn atomic_resize_rejects_invalid_tuple_without_partial_mutation() {
    let mut session = editor_session();
    let original = (session.logical, session.dpr, session.insets);

    assert!(session
        .resize_with_safe_area(
            600.0,
            700.0,
            2.0,
            OpInsets {
                top: 24.0,
                right: -1.0,
                bottom: 20.0,
                left: 0.0,
            },
        )
        .is_err());

    assert_eq!((session.logical, session.dpr, session.insets), original);
}
