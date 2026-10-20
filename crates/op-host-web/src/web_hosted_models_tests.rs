use super::*;

fn route_json(protocol: &str) -> serde_json::Value {
    serde_json::json!({
        "route_id": "session-route-1",
        "model_id": "hosted-model",
        "display_name": "Hosted Model",
        "protocol": protocol,
        "capabilities": {
            "image_input": true, "tools": true,
            "structured_output": false, "reasoning": false
        }
    })
}

fn response_json(route: serde_json::Value) -> String {
    serde_json::json!({"status":"ready","revision":1,"routes":[route]}).to_string()
}

#[test]
fn protocols_retain_their_exact_identity() {
    for (wire, protocol) in [
        ("openai-compatible", HostedProtocol::OpenAiCompatible),
        ("anthropic-messages", HostedProtocol::AnthropicMessages),
        ("openai-responses", HostedProtocol::OpenAiResponses),
    ] {
        let parsed = parse_catalog(&response_json(route_json(wire))).expect("valid catalog");
        assert!(!parsed.loading);
        assert_eq!(parsed.routes[0].protocol(), protocol);
        assert!(parsed.routes[0].capabilities().tools);
    }
    assert!(parse_catalog(&response_json(route_json("openai"))).is_err());
}

#[test]
fn rejects_duplicate_unknown_secret_and_oversized_fields() {
    for body in [
        r#"{"status":"ready","status":"signed_out","revision":1,"routes":[]}"#,
        r#"{"status":"ready","revision":1,"routes":[],"bearer":"secret"}"#,
        r#"{"status":"ready","revision":0,"routes":[]}"#,
        r#"{"status":"unexpected","revision":1,"routes":[]}"#,
    ] {
        assert!(parse_catalog(body).is_err());
    }
    for field in ["gateway_base_url", "credential", "bearer"] {
        let mut route = route_json("openai-compatible");
        route[field] = serde_json::json!("secret");
        assert!(parse_catalog(&response_json(route)).is_err());
    }
    let duplicate = response_json(route_json("openai-compatible"))
        .replace("\"route_id\":", "\"route_id\":\"old\",\"route_id\":");
    assert!(parse_catalog(&duplicate).is_err());
    let oversized = " ".repeat(MAX_CATALOG_BYTES + 1);
    assert!(matches!(
        parse_catalog(&oversized),
        Err(CatalogError::TooLarge)
    ));
    let mut route = route_json("openai-compatible");
    route["display_name"] = serde_json::json!("x".repeat(129));
    assert!(parse_catalog(&response_json(route)).is_err());
}

#[test]
fn rejects_duplicate_routes_excess_routes_and_nonready_route_projection() {
    let route = route_json("openai-compatible");
    for routes in [
        vec![route.clone(); 2],
        vec![route.clone(); MAX_HOSTED_MODEL_ROUTES + 1],
    ] {
        let body = serde_json::json!({"status":"ready","revision":1,"routes":routes});
        assert!(parse_catalog(&body.to_string()).is_err());
    }
    for status in ["loading", "signed_out", "unavailable", "failed"] {
        let with_route = serde_json::json!({"status":status,"revision":1,"routes":[route.clone()]});
        assert!(parse_catalog(&with_route.to_string()).is_err());
        let empty = serde_json::json!({"status":status,"revision":1,"routes":[]});
        let catalog = parse_catalog(&empty.to_string()).expect("valid empty state");
        assert_eq!(catalog.loading, status == "loading");
    }
}

#[test]
fn catalog_failure_clears_all_tabs_and_cannot_install_while_signed_out() {
    let mut state = EditorState::new();
    let body = response_json(route_json("openai-compatible"));
    apply_catalog(&mut state, parse_catalog(&body));
    assert!(state.editor_ui.hosted_model_routes().is_empty());
    state.editor_ui.account = op_editor_core::AccountState::signed_in_profile("test".into(), None);
    apply_catalog(&mut state, parse_catalog(&body));
    assert_eq!(state.editor_ui.hosted_model_routes().len(), 1);
    state.chat.new_tab();
    apply_catalog(&mut state, Err(CatalogError::Invalid));
    assert!(state.editor_ui.hosted_model_routes().is_empty());
    assert!(state
        .chat
        .tabs()
        .iter()
        .all(|tab| tab.available_models.iter().all(|entry| !entry.is_hosted())));
}

#[test]
fn account_switch_logout_and_superseding_refresh_reject_late_catalogs() {
    crate::identity_epoch::reset_for_test();
    crate::identity_epoch::observe_subject(Some("account-a"));
    reset();
    let generation = GENERATION.with(Cell::get);
    let epoch = crate::identity_epoch::epoch();
    assert!(current_request(generation, epoch));
    reset();
    assert!(!current_request(generation, epoch));
    let next = GENERATION.with(Cell::get);
    crate::identity_epoch::observe_subject(Some("account-b"));
    assert!(!current_request(next, epoch));
    let epoch = crate::identity_epoch::epoch();
    assert!(current_request(next, epoch));
    crate::identity_epoch::observe_subject(None);
    assert!(!current_request(next, epoch));
}
