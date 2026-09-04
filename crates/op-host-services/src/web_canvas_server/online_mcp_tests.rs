//! The online MCP capability profile, exercised end to end through the
//! accept loop.
//!
//! Split out of `online_run_loop_tests.rs` at the 800-line cap; nested under
//! it so `use super::*` still reaches the request builder, the mock stream,
//! and the tenant/verifier helpers.

use super::*;

/// `tokR` is read-only; `tokA`/`tokB` carry full authority.
fn scoped_verifier() -> StaticVerifier {
    StaticVerifier::parse("tokA=userA,tokB=userB,tokR=userR:read")
}

fn mcp_call(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

/// Drive one JSON-RPC message at `/mcp` and return the decoded body.
fn mcp(
    registry: &TenantRegistry,
    verifier: &StaticVerifier,
    token: &'static str,
    message: &str,
) -> serde_json::Value {
    let response = serve(
        registry,
        verifier,
        Request::json("POST", "/mcp", message).with_bearer(token),
    );
    assert_eq!(
        status_line(&response),
        "HTTP/1.1 200 OK",
        "a refusal is still a 200 JSON-RPC envelope: {response}"
    );
    body(&response)
}

const TOOLS_LIST: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;

/// The text block of a `tools/call` response.
fn call_text(payload: &serde_json::Value) -> String {
    payload["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn the_online_tool_catalog_omits_every_local_resource_tool() {
    let listed = mcp(&registry(), &verifier(), "tokA", TOOLS_LIST);
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("a tools array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(!names.is_empty(), "the catalog must not be empty");
    for denied in crate::mcp_serve::tool_profile::denied_tool_names() {
        assert!(
            !names.contains(&denied),
            "{denied} must not be advertised to a public client"
        );
    }
    // The in-memory catalog is still there — this is a filter, not a shutdown.
    for kept in [
        "get_node",
        "add_page",
        "insert_node",
        "batch_design",
        "list_scene_templates",
        "use_scene_template",
    ] {
        assert!(names.contains(&kept), "{kept} must still be offered");
    }
}

#[test]
fn online_tool_search_cannot_rediscover_hidden_local_tools() {
    let payload = mcp(
        &registry(),
        &verifier(),
        "tokA",
        &mcp_call(
            "ToolSearch",
            r#"{"query":"select:save_document,get_node","max_results":11}"#,
        ),
    );
    assert_ne!(payload["result"]["isError"], true, "{payload}");

    let result: serde_json::Value =
        serde_json::from_str(&call_text(&payload)).expect("ToolSearch result JSON");
    let names: Vec<&str> = result["results"]
        .as_array()
        .expect("results array")
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert_eq!(names, ["get_node"], "{result}");
}

#[test]
fn calling_a_denied_tool_by_name_is_refused_rather_than_executed() {
    let payload = mcp(
        &registry(),
        &verifier(),
        "tokA",
        &mcp_call(
            "save_document",
            r#"{"filePath":"/tmp/op-m3-should-not-exist.op"}"#,
        ),
    );
    assert_eq!(payload["result"]["isError"], true, "{payload}");
    let text = call_text(&payload);
    assert!(text.contains("tool-not-available"), "{text}");
    assert!(text.contains("save_document"), "{text}");
    // Filtering the catalog is discovery; this is the enforcement.
    assert!(
        !std::path::Path::new("/tmp/op-m3-should-not-exist.op").exists(),
        "a denied tool must never reach the code that writes the path"
    );
}

#[test]
fn a_path_traversal_argument_on_a_denied_tool_never_reaches_the_filesystem() {
    let target = std::env::temp_dir().join("op-m3-traversal-probe.op");
    let _ = std::fs::remove_file(&target);
    let payload = mcp(
        &registry(),
        &verifier(),
        "tokA",
        &mcp_call(
            "save_document",
            &serde_json::json!({
                "filePath": format!("../../../../../../{}", target.display()),
            })
            .to_string(),
        ),
    );
    assert_eq!(payload["result"]["isError"], true, "{payload}");
    assert!(
        !target.exists(),
        "the deny layer runs before the tool parses its path, so traversal is moot"
    );
}

#[test]
fn every_denied_tool_is_refused_when_called_directly() {
    let registry = registry();
    let verifier = verifier();
    for denied in crate::mcp_serve::tool_profile::denied_tool_names() {
        let payload = mcp(&registry, &verifier, "tokA", &mcp_call(denied, "{}"));
        assert_eq!(payload["result"]["isError"], true, "{denied}: {payload}");
        assert!(
            call_text(&payload).contains("tool-not-available"),
            "{denied}: {}",
            call_text(&payload)
        );
    }
}

#[test]
fn a_read_scope_token_may_read_but_not_write() {
    let registry = registry();
    let verifier = scoped_verifier();

    let read = mcp(
        &registry,
        &verifier,
        "tokR",
        &mcp_call("get_document_info", "{}"),
    );
    assert_ne!(
        read["result"]["isError"], true,
        "a read must succeed: {read}"
    );

    let write = mcp(
        &registry,
        &verifier,
        "tokR",
        &mcp_call("add_page", r#"{"name":"nope"}"#),
    );
    assert_eq!(write["result"]["isError"], true, "{write}");
    assert!(
        call_text(&write).contains("scope-insufficient"),
        "{}",
        call_text(&write)
    );
}

#[test]
fn a_full_scope_token_may_write_the_same_tool_a_read_token_cannot() {
    let registry = registry();
    let verifier = scoped_verifier();
    let write = mcp(
        &registry,
        &verifier,
        "tokA",
        &mcp_call("add_page", r#"{"name":"ok"}"#),
    );
    assert_ne!(write["result"]["isError"], true, "{write}");
}

#[test]
fn a_read_scope_token_is_refused_the_write_before_the_document_changes() {
    let registry = registry();
    let verifier = scoped_verifier();
    let before = mcp(
        &registry,
        &verifier,
        "tokR",
        &mcp_call("get_document_info", "{}"),
    );
    let _ = mcp(
        &registry,
        &verifier,
        "tokR",
        &mcp_call("add_page", r#"{"name":"nope"}"#),
    );
    let after = mcp(
        &registry,
        &verifier,
        "tokR",
        &mcp_call("get_document_info", "{}"),
    );
    assert_eq!(
        call_text(&before),
        call_text(&after),
        "a scope refusal must not have mutated anything"
    );
}

#[test]
fn the_local_daemon_still_lists_and_calls_the_whole_catalog() {
    // The other half of the contract: none of the above may leak into the
    // single-user daemon, whose operator owns the filesystem it writes.
    let profile = crate::mcp_serve::tool_profile::McpAccessProfile::UNRESTRICTED;
    for denied in crate::mcp_serve::tool_profile::denied_tool_names() {
        assert!(profile.lists(denied), "{denied} must stay listed locally");
        assert_eq!(
            profile.refuse(denied),
            None,
            "{denied} must stay callable locally"
        );
    }
}

// ---------------------------------------------------------------------------
// H7: scopes apply to REST, not just to /mcp.
// ---------------------------------------------------------------------------

#[test]
fn a_read_scope_token_cannot_replace_the_document_over_rest() {
    // The bypass: the same token is refused `add_page` on /mcp but could
    // replace the entire document here — strictly more damage.
    let response = serve(
        &registry(),
        &scoped_verifier(),
        Request::json("POST", "/api/mcp/document", SYNC_BODY).with_bearer("tokR"),
    );
    assert_eq!(
        status_line(&response),
        "HTTP/1.1 403 Forbidden",
        "{response}"
    );
    assert_eq!(body(&response)["error"], "scope-insufficient");
}

#[test]
fn a_read_scope_token_may_still_read_over_rest() {
    let response = serve(
        &registry(),
        &scoped_verifier(),
        Request::new("GET", "/api/mcp/document").with_bearer("tokR"),
    );
    assert_eq!(status_line(&response), "HTTP/1.1 200 OK", "{response}");
}

#[test]
fn a_token_with_no_scopes_is_refused_every_rest_route_but_the_probe() {
    // Fail-closed: op-hub issues no tokens yet, so an unscoped token is inert.
    let verifier = StaticVerifier::parse("tokN=userN:none");
    let registry = registry();
    for (method, path) in [("GET", "/api/mcp/document"), ("GET", "/api/mcp/version")] {
        let response = serve(
            &registry,
            &verifier,
            Request::new(method, path).with_bearer("tokN"),
        );
        assert_eq!(
            status_line(&response),
            "HTTP/1.1 403 Forbidden",
            "{method} {path}: {response}"
        );
    }
    let push = serve(
        &registry,
        &verifier,
        Request::json("POST", "/api/mcp/document", SYNC_BODY).with_bearer("tokN"),
    );
    assert_eq!(status_line(&push), "HTTP/1.1 403 Forbidden", "{push}");

    // The health probe stays reachable so a client can discover the daemon.
    let probe = serve(
        &registry,
        &verifier,
        Request::new("GET", "/api/mcp/server").with_bearer("tokN"),
    );
    assert_eq!(status_line(&probe), "HTTP/1.1 200 OK", "{probe}");
}

#[test]
fn a_full_scope_token_is_unaffected_over_rest() {
    let registry = registry();
    let response = serve(
        &registry,
        &scoped_verifier(),
        Request::json("POST", "/api/mcp/document", SYNC_BODY).with_bearer("tokA"),
    );
    assert_eq!(status_line(&response), "HTTP/1.1 200 OK", "{response}");
}

#[test]
fn a_browser_session_is_not_scope_limited_over_rest() {
    // A session IS the account; scopes narrow a token below it, not the
    // account below itself.
    let registry = registry();
    let verifier = StaticVerifier::parse("sessA=userA");
    let response = serve(
        &registry,
        &verifier,
        Request::json("POST", "/api/mcp/document", SYNC_BODY)
            .with_session("sessA")
            .with_origin(PUBLIC_ORIGIN),
    );
    assert_eq!(status_line(&response), "HTTP/1.1 200 OK", "{response}");
}

#[test]
fn a_read_scope_token_cannot_reach_the_specially_dispatched_write_routes() {
    // These are dispatched ahead of the `/api/*` branch the gate used to live
    // in, so a read-only token could drive all of them.
    let registry = registry();
    let verifier = scoped_verifier();
    for (method, path, body) in [
        (
            "POST",
            op_editor_core::share_routes::GRANT,
            r#"{"userId":"userB"}"#,
        ),
        (
            "POST",
            op_editor_core::share_routes::REVOKE,
            r#"{"userId":"userB"}"#,
        ),
        ("POST", "/api/ai/standard", "{}"),
        ("POST", "/api/ai/stream", "{}"),
        ("POST", "/api/figma/convert", "{}"),
        (
            "POST",
            op_editor_core::collab_routes::ACTION,
            r#"{"type":"openCreate"}"#,
        ),
    ] {
        let response = serve(
            &registry,
            &verifier,
            Request::json(method, path, body).with_bearer("tokR"),
        );
        assert_eq!(
            status_line(&response),
            "HTTP/1.1 403 Forbidden",
            "{method} {path}: {response}"
        );
        assert_eq!(body_of(&response)["error"], "scope-insufficient", "{path}");
    }
}

#[test]
fn a_scopeless_token_cannot_even_subscribe_to_the_event_stream() {
    // SSE is a GET, so it needs `mcp:read` — and a scopeless token has none.
    let response = serve(
        &registry(),
        &StaticVerifier::parse("tokN=userN:none"),
        Request::new("GET", "/api/mcp/events").with_bearer("tokN"),
    );
    assert_eq!(
        status_line(&response),
        "HTTP/1.1 403 Forbidden",
        "{response}"
    );
}

#[test]
fn a_read_scope_token_may_still_subscribe_to_the_event_stream() {
    // A read token reading is the whole point; only writes are refused.
    let response = serve(
        &registry(),
        &scoped_verifier(),
        Request::new("GET", op_editor_core::collab_routes::STATE).with_bearer("tokR"),
    );
    assert_eq!(status_line(&response), "HTTP/1.1 200 OK", "{response}");
}

/// The share/AI routes answer with a plain body; reuse the shared decoder.
fn body_of(response: &str) -> serde_json::Value {
    body(response)
}
