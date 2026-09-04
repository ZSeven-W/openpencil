//! `/api/collab/*` route coverage, driven through the same
//! `handle_web_canvas_request` entry point the connection loop uses.

use op_editor_core::collab_wire::{CollabStateWire, COLLAB_WIRE_VERSION};
use op_editor_core::{
    AuthenticatedCollabSession, CollabConnectionPhase, CollabUiAction, CollabUiRole, EditorState,
};

use super::super::{handle_web_canvas_request, WebCanvasState, WebReply};

fn daemon() -> WebCanvasState {
    WebCanvasState::new(EditorState::starter(), 0)
}

fn call(method: &str, path: &str, body: &str, state: &mut WebCanvasState) -> WebReply {
    handle_web_canvas_request(method, path, body, state)
}

fn json(reply: &WebReply) -> serde_json::Value {
    serde_json::from_str(&reply.body).expect("route bodies are JSON")
}

#[test]
fn state_answers_a_versioned_projection_even_with_no_session() {
    let mut state = daemon();
    let reply = call("GET", op_editor_core::collab_routes::STATE, "", &mut state);

    assert_eq!(reply.status, "200 OK");
    let wire: CollabStateWire = serde_json::from_str(&reply.body).expect("decodes as the DTO");
    assert_eq!(wire.wire_version, COLLAB_WIRE_VERSION);
    assert_eq!(wire.collab_seq, 0);
    assert!(wire.session.is_none());
    // Availability is whatever the runtime last published; the point is that
    // the field is always present so a client can render a reason.
    assert!(json(&reply).get("availability").is_some());
}

#[test]
fn state_reports_the_two_sequence_numbers_separately() {
    let mut state = daemon();
    state.collab.bump_seq();
    state.collab.bump_seq();

    let wire: CollabStateWire = serde_json::from_str(
        &call("GET", op_editor_core::collab_routes::STATE, "", &mut state).body,
    )
    .expect("decodes");
    assert_eq!(wire.collab_seq, 2);
    assert_eq!(wire.document_revision, state.editor.document_revision());
}

#[test]
fn a_posted_action_lands_in_the_pending_slot_and_bumps_the_projection() {
    let mut state = daemon();
    let reply = call(
        "POST",
        op_editor_core::collab_routes::ACTION,
        r#"{"type":"start"}"#,
        &mut state,
    );

    assert_eq!(reply.status, "202 Accepted");
    assert_eq!(json(&reply)["collabSeq"], 1);
    assert_eq!(
        state.editor.editor_ui.collab.pending_action,
        Some(CollabUiAction::Start)
    );
}

#[test]
fn a_second_action_conflicts_instead_of_overwriting_the_first() {
    let mut state = daemon();
    call(
        "POST",
        op_editor_core::collab_routes::ACTION,
        r#"{"type":"start"}"#,
        &mut state,
    );
    let reply = call(
        "POST",
        op_editor_core::collab_routes::ACTION,
        r#"{"type":"leave"}"#,
        &mut state,
    );

    assert_eq!(reply.status, "409 Conflict");
    assert_eq!(json(&reply)["error"], "collab-busy");
    assert_eq!(
        state.editor.editor_ui.collab.pending_action,
        Some(CollabUiAction::Start),
        "the queued action must survive — dropping it would lose what the user asked for"
    );
}

#[test]
fn a_malformed_action_is_refused_before_it_reaches_the_runtime() {
    let mut state = daemon();
    for (body, code) in [
        (r#"{"type":"selfDestruct"}"#, "malformed-action"),
        (r#"not json"#, "malformed-action"),
        (
            r#"{"type":"rejectAdmission","requestKey":"has space"}"#,
            "invalid-request-key",
        ),
        (
            r#"{"type":"joinAddress","endpoint":"  "}"#,
            "invalid-address",
        ),
    ] {
        let reply = call(
            "POST",
            op_editor_core::collab_routes::ACTION,
            body,
            &mut state,
        );
        assert_eq!(reply.status, "400 Bad Request", "{body}");
        assert_eq!(json(&reply)["error"], code, "{body}");
        assert!(state.editor.editor_ui.collab.pending_action.is_none());
    }
}

#[test]
fn lan_actions_are_accepted_by_a_local_daemon() {
    // Desktop parity: the operator is the only client of a local or managed
    // daemon, so LAN discovery and direct joins stay available. The public
    // deployment is where these get refused.
    for body in [
        r#"{"type":"startLan"}"#,
        r#"{"type":"beginDiscovery"}"#,
        r#"{"type":"joinAddress","endpoint":"192.168.1.10:43120"}"#,
    ] {
        let mut state = daemon();
        let reply = call(
            "POST",
            op_editor_core::collab_routes::ACTION,
            body,
            &mut state,
        );
        assert_eq!(reply.status, "202 Accepted", "{body}");
    }
}

#[test]
fn presence_is_stored_without_bumping_the_projection_sequence() {
    let mut state = daemon();
    let reply = call(
        "POST",
        op_editor_core::collab_routes::PRESENCE,
        r#"{"cursor":{"x":12.5,"y":-3.0}}"#,
        &mut state,
    );

    assert_eq!(reply.status, "202 Accepted");
    assert_eq!(state.collab.presence_override(), Some((12.5, -3.0)));
    assert_eq!(
        state.collab.seq(),
        0,
        "the local cursor going out is not incoming state; bumping here would \
         make every mouse move wake every poller"
    );
}

#[test]
fn presence_accepts_a_cleared_cursor() {
    let mut state = daemon();
    call(
        "POST",
        op_editor_core::collab_routes::PRESENCE,
        r#"{"cursor":{"x":1.0,"y":2.0}}"#,
        &mut state,
    );
    call(
        "POST",
        op_editor_core::collab_routes::PRESENCE,
        r#"{"cursor":null}"#,
        &mut state,
    );
    assert_eq!(state.collab.presence_override(), None);
}

#[test]
fn oversized_bodies_are_refused_before_parsing() {
    let mut state = daemon();
    let huge = format!(
        r#"{{"type":"joinAddress","endpoint":"{}"}}"#,
        "a".repeat(9000)
    );
    for path in [
        op_editor_core::collab_routes::ACTION,
        op_editor_core::collab_routes::PRESENCE,
    ] {
        let reply = call("POST", path, &huge, &mut state);
        assert_eq!(reply.status, "413 Payload Too Large", "{path}");
    }
}

#[test]
fn the_version_probe_carries_the_projection_sequence_too() {
    let mut state = daemon();
    state.collab.bump_seq();
    let reply = call("GET", "/api/mcp/version", "", &mut state);

    let body = json(&reply);
    // Additive: the existing 400 ms poll keeps reading `version` and now also
    // notices collaboration changes without a second request.
    assert_eq!(body["version"], 0);
    assert_eq!(body["collabSeq"], 1);
}

#[test]
fn a_session_projects_through_the_state_route() {
    let mut state = daemon();
    state.editor.editor_ui.collab.set_authenticated_session(
        CollabConnectionPhase::Active,
        AuthenticatedCollabSession {
            session_name: "studio".into(),
            role: CollabUiRole::Owner,
            share_endpoint: None,
        },
        Vec::new(),
    );

    let wire: CollabStateWire = serde_json::from_str(
        &call("GET", op_editor_core::collab_routes::STATE, "", &mut state).body,
    )
    .expect("decodes");
    let session = wire.session.expect("session projected");
    assert_eq!(session.session_name, "studio");
}

#[test]
fn document_push_is_refused_with_a_code_while_a_session_is_read_only_for_us() {
    let mut state = daemon();
    state.editor.editor_ui.collab.set_authenticated_session(
        CollabConnectionPhase::Active,
        AuthenticatedCollabSession {
            session_name: "studio".into(),
            role: CollabUiRole::Viewer,
            share_endpoint: None,
        },
        Vec::new(),
    );
    let before = state.editor.doc.clone();

    let reply = call(
        "POST",
        "/api/mcp/document",
        r#"{"document":{"version":"1.0","children":[]}}"#,
        &mut state,
    );

    assert_eq!(reply.status, "409 Conflict");
    assert_eq!(json(&reply)["error"], "collab-readonly");
    assert_eq!(state.editor.doc, before, "a refused push writes nothing");
}

#[test]
fn sync_reset_and_open_recent_are_refused_while_a_session_is_active() {
    for (path, body) in [
        ("/api/mcp/sync-reset", ""),
        ("/api/file/open-recent", r#"{"path":"/tmp/x.op"}"#),
    ] {
        let mut state = daemon();
        state.editor.editor_ui.collab.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "studio".into(),
                role: CollabUiRole::Owner,
                share_endpoint: None,
            },
            Vec::new(),
        );

        let reply = call("POST", path, body, &mut state);
        assert_eq!(reply.status, "409 Conflict", "{path}");
        assert_eq!(json(&reply)["error"], "collab-active", "{path}");
    }
}

#[test]
fn sync_reset_still_works_with_no_session() {
    let mut state = daemon();
    let reply = call("POST", "/api/mcp/sync-reset", "", &mut state);
    assert_eq!(reply.status, "200 OK", "{}", reply.body);
    assert_eq!(json(&reply)["ok"], true);
}

#[test]
fn the_collab_routes_are_gated_as_sensitive_browser_posts() {
    for path in [
        op_editor_core::collab_routes::ACTION,
        op_editor_core::collab_routes::PRESENCE,
    ] {
        let request = crate::mcp_serve::HttpRequest {
            method: "POST".into(),
            path: path.into(),
            body: String::new(),
            content_type: Some("application/json".into()),
            origin: None,
            host: None,
            token: None,
            authorization: None,
            cookie: None,
            query: None,
        };
        assert!(
            super::super::is_sensitive_browser_post(&request),
            "{path} must sit behind the same-origin gate"
        );
    }
}

#[test]
fn managed_collaboration_routes_use_the_origin_boundary_not_a_token() {
    let allow = vec!["http://127.0.0.1:3100".to_string()];
    assert!(super::super::managed_request_origin_allowed(
        &allow, None, None
    ));
    assert!(super::super::managed_request_origin_allowed(
        &allow,
        Some("http://127.0.0.1:3100"),
        Some("127.0.0.1:3100")
    ));
    assert!(!super::super::managed_request_origin_allowed(
        &allow,
        Some("https://evil.example"),
        Some("127.0.0.1:3100")
    ));
}
