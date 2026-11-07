use super::*;
use crate::input_event::{PreviewInput, PreviewInputEnvelope};
use crate::test_measure;

/// Panel-authored documents carry no `app.capabilities` block — the
/// editor's own Preview must still deliver their effects.
fn doc() -> jian_ops_schema::PenDocument {
    jian_ops_schema::load_str(
        r##"{
        "version": "1.0",
        "children": [
            { "type": "frame", "id": "btn", "x": 0, "y": 0, "width": 200, "height": 200,
              "events": { "onTap": [
                  { "toast": "\"Saved!\"" },
                  { "copy": "\"code-123\"" },
                  { "open_url": { "url": "\"https://openpencil.dev\"" } },
                  { "alert": { "title": "\"Hi\"", "message": "\"There\"" } }
              ] } }
        ]
    }"##,
    )
    .expect("parse")
    .value
}

fn tap(session: &mut PreviewSession) {
    for (phase, t) in [
        (jian_core::gesture::pointer::PointerPhase::Down, 0),
        (jian_core::gesture::pointer::PointerPhase::Up, 10),
    ] {
        let mut event = jian_core::gesture::PointerEvent::simple_at(
            1,
            phase,
            jian_core::geometry::point(50.0, 50.0),
            t,
        );
        event.kind = jian_core::gesture::pointer::PointerKind::Touch;
        let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(event)));
    }
}

#[test]
fn editor_preview_delivers_toast_copy_link_and_alert() {
    let mut session = PreviewSession::enter_with_capabilities(
        &doc(),
        (800.0, 600.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
        EDITOR_PREVIEW_CAPABILITIES,
        0,
    )
    .expect("enter");
    tap(&mut session);
    let mut ui = EditorUiState::default();
    let requests = session.deliver_editor_effects(&mut ui, 100);
    assert_eq!(
        requests,
        vec![
            PreviewPlatformRequest::CopyText("code-123".into()),
            PreviewPlatformRequest::OpenUrl("https://openpencil.dev".into()),
        ],
        "warnings: {:?}",
        session.warnings()
    );
    let toast = ui
        .visible_toast(100)
        .expect("alert shows as the latest toast");
    assert_eq!(toast.i18n_key, PREVIEW_TOAST_KEY);
    assert_eq!(toast.arg_pairs(), vec![("message", "Hi — There")]);
    assert!(session.deliver_editor_effects(&mut ui, 200).is_empty());
}
