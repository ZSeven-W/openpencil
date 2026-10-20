//! Account-scoped model state across daemon document swaps.

use super::*;
use op_editor_core::{
    AccountState, HostedModelCapabilities, HostedModelRoute, HostedProtocol, ModelEntry,
};

#[test]
fn daemon_document_swap_preserves_hosted_catalog_and_selection() {
    let mut previous = EditorState::new();
    previous.editor_ui.account_ui_available = true;
    previous.editor_ui.account =
        AccountState::signed_in_profile("Hosted User".to_string(), Some("hosted".to_string()));
    let route = HostedModelRoute::new(
        "daemon-hosted-route",
        "hosted-model",
        "Hosted Model",
        HostedProtocol::AnthropicMessages,
        HostedModelCapabilities::default(),
    )
    .expect("valid hosted route");
    previous
        .replace_hosted_model_catalog(vec![route])
        .expect("catalog");
    let mut next = EditorState::starter();

    preserve_web_canvas_preferences(&previous, &mut next);

    assert!(next.editor_ui.account_ui_available);
    assert_eq!(next.editor_ui.account, previous.editor_ui.account);
    assert_eq!(next.editor_ui.hosted_model_routes().len(), 1);
    assert_eq!(
        next.chat
            .selected_model_entry()
            .and_then(ModelEntry::hosted_route_id),
        Some("daemon-hosted-route")
    );
}
