//! Hosted routing never converts runtime model metadata into CLI authority.

use super::providers::provider_for_hosted_selection_with;
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
use op_editor_core::{
    AccountState, EditorState, HostedModelCapabilities, HostedModelRoute, HostedProtocol,
    ModelEntry,
};
use std::cell::Cell;

struct RuntimeProvider;

impl ChatProvider for RuntimeProvider {
    fn provider_label(&self) -> &str {
        "Hosted fixture"
    }

    fn send(&self, _: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(std::iter::empty())
    }
}

fn route(model_id: &str) -> HostedModelRoute {
    HostedModelRoute::new(
        "hosted-route",
        model_id,
        "Hosted model",
        HostedProtocol::OpenAiCompatible,
        HostedModelCapabilities::default(),
    )
    .expect("route fixture")
}

fn signed_in_state() -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.account =
        AccountState::signed_in_profile("Test user".into(), Some("test-user".into()));
    state
}

#[test]
fn authenticated_catalog_routes_only_the_opaque_route_id() {
    let mut state = signed_in_state();
    let current = route("gateway-alias");
    state
        .replace_hosted_model_catalog(vec![current.clone()])
        .unwrap();
    let entry = ModelEntry::hosted(current);
    let resolved = provider_for_hosted_selection_with(&state, &entry, |route_id| {
        assert_eq!(route_id, "hosted-route");
        Some(Box::new(RuntimeProvider))
    });
    assert_eq!(resolved.unwrap().provider_label(), "Hosted fixture");
    assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
}

#[test]
fn signed_out_projection_cannot_start_a_runtime_request() {
    let state = EditorState::new();
    let called = Cell::new(false);
    let result =
        provider_for_hosted_selection_with(&state, &ModelEntry::hosted(route("one")), |_| {
            called.set(true);
            Some(Box::new(RuntimeProvider))
        });
    assert!(result.is_none());
    assert!(!called.get());
}

#[test]
fn removed_or_replaced_catalog_rows_are_rejected_before_runtime_dispatch() {
    let mut state = signed_in_state();
    let stale = ModelEntry::hosted(route("old-model"));
    for replacement in [vec![], vec![route("new-model")]] {
        state.replace_hosted_model_catalog(replacement).unwrap();
        let called = Cell::new(false);
        let result = provider_for_hosted_selection_with(&state, &stale, |_| {
            called.set(true);
            Some(Box::new(RuntimeProvider))
        });
        assert!(result.is_none());
        assert!(!called.get());
    }
}

#[test]
fn runtime_rejection_never_substitutes_a_local_provider() {
    let mut state = signed_in_state();
    let current = route("gateway-alias");
    state
        .replace_hosted_model_catalog(vec![current.clone()])
        .unwrap();
    assert!(
        provider_for_hosted_selection_with(&state, &ModelEntry::hosted(current), |_| None)
            .is_none()
    );
}

#[test]
fn hosted_protocol_hints_never_become_cli_model_flags() {
    let mut host = op_host_native::WidgetHostNative::new();
    *host.editor_state_mut() = signed_in_state();
    host.editor_state_mut()
        .replace_hosted_model_catalog(vec![route("model-alias")])
        .unwrap();
    host.editor_state_mut().chat.selected_model = 0;
    host.editor_state_mut().editor_ui.chat_selected_agent = 1;
    assert!(super::providers::selected_cli_model_id(&host).is_none());
    assert!(super::providers::builtin_provider_with_tools(&host).is_none());
}
