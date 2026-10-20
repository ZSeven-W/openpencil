//! Hosted-model lifecycle checks for whole-state document replacement.

use super::*;
use op_editor_core::{
    AccountState, HostedModelCapabilities, HostedModelRoute, HostedProtocol, ModelEntry,
};

fn hosted_route(route_id: &str, model_id: &str) -> HostedModelRoute {
    HostedModelRoute::new(
        route_id,
        model_id,
        model_id,
        HostedProtocol::OpenAiResponses,
        HostedModelCapabilities::default(),
    )
    .expect("valid hosted route")
}

fn signed_in_state() -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.account_ui_available = true;
    state.editor_ui.account =
        AccountState::signed_in_profile("Hosted User".to_string(), Some("hosted".to_string()));
    state
        .replace_hosted_model_catalog(vec![
            hosted_route("hosted-route-a", "hosted-model-a"),
            hosted_route("hosted-route-b", "hosted-model-b"),
        ])
        .expect("catalog");
    let route_b = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.hosted_route_id() == Some("hosted-route-b"))
        .expect("route b");
    state.select_chat_model(route_b);
    state
}

#[test]
fn app_preferences_preserve_transient_hosted_catalog_and_selection() {
    let previous = signed_in_state();
    let mut next = EditorState::starter();

    preserve_app_preferences(&previous, &mut next);

    assert!(next.editor_ui.account.is_signed_in());
    assert_eq!(next.editor_ui.hosted_model_routes().len(), 2);
    assert_eq!(
        next.chat
            .selected_model_entry()
            .and_then(ModelEntry::hosted_route_id),
        Some("hosted-route-b")
    );
}

#[test]
fn canonical_save_never_serializes_hosted_catalog_or_account_profile() {
    let state = signed_in_state();
    let path = std::env::temp_dir().join(format!(
        "openpencil-hosted-catalog-{}-{:?}.op",
        std::process::id(),
        std::thread::current().id()
    ));

    save_to_path(&state, &path).expect("save");
    let saved = std::fs::read_to_string(&path).expect("read save");
    assert!(!saved.contains("hosted-route-a"));
    assert!(!saved.contains("hosted-model-a"));
    assert!(!saved.contains("Hosted User"));

    let reopened = load_editor_state(&path, op_editor_core::Locale::EnUs).expect("reopen");
    assert!(reopened.editor_ui.hosted_model_routes().is_empty());
    assert_eq!(reopened.editor_ui.account, AccountState::Anonymous);

    let _ = std::fs::remove_file(path);
}
