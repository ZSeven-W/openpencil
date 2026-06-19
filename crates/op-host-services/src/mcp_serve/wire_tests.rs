//! Wire-protocol handshake/identity unit tests (ping / sanitize_token /
//! shutdown / classify_stateless / sniff / initialize). Split out of
//! `tests.rs` to keep each test file under the 800-line cap.

#![cfg(test)]

use super::*;

#[test]
fn ping_response_carries_openpencil_identity_marker() {
    // The `op` CLI keys discovery on this marker; if it ever changes,
    // CLI `mcp_ping` must change in lockstep. No token ⇒ no `mode`/`token`.
    let resp = ping_response("7", None);
    assert!(resp.contains(r#""server":"openpencil-mcp""#), "{resp}");
    assert!(resp.contains(r#""id":7"#), "{resp}");
    assert!(!resp.contains("mode"), "{resp}");
}

#[test]
fn ping_response_embeds_headless_token_when_present() {
    let resp = ping_response("5", Some("abc-123"));
    assert!(resp.contains(r#""mode":"headless""#), "{resp}");
    assert!(resp.contains(r#""token":"abc-123""#), "{resp}");
    assert!(resp.contains(r#""server":"openpencil-mcp""#), "{resp}");
}

#[test]
fn sanitize_token_rejects_unsafe_or_empty() {
    // Safe tokens (the CLI emits pid-nanos hex) pass through.
    assert_eq!(sanitize_token("abc-123".into()).as_deref(), Some("abc-123"));
    assert_eq!(sanitize_token("a_b-9F".into()).as_deref(), Some("a_b-9F"));
    // A token that would break the JSON ping reply is rejected (no injection).
    assert_eq!(sanitize_token("bad\"token".into()), None);
    assert_eq!(sanitize_token("".into()), None);
}

#[test]
fn shutdown_request_id_requires_matching_nonempty_token() {
    let line =
        r#"{"jsonrpc":"2.0","id":9,"method":"openpencil/shutdown","params":{"token":"tok-1"}}"#;
    assert_eq!(shutdown_request_id(line, "tok-1").as_deref(), Some("9"));
    assert_eq!(shutdown_request_id(line, "tok-2"), None); // wrong token
    assert_eq!(shutdown_request_id(line, ""), None); // empty never authenticates
                                                     // A non-shutdown method is never treated as a shutdown.
    assert_eq!(
        shutdown_request_id(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#, "tok-1"),
        None
    );
    // Missing params.token → no shutdown.
    assert_eq!(
        shutdown_request_id(
            r#"{"jsonrpc":"2.0","id":1,"method":"openpencil/shutdown","params":{}}"#,
            "tok-1"
        ),
        None
    );
}

#[test]
fn classify_stateless_routes_handshake_methods() {
    assert!(matches!(
        classify_stateless(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#),
        Stateless::Ping(_)
    ));
    assert!(matches!(
        classify_stateless(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#),
        Stateless::Respond(_)
    ));
    assert!(matches!(
        classify_stateless(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
        Stateless::Swallow
    ));
    // tools/call needs document state — must NOT be short-circuited.
    assert!(matches!(
        classify_stateless(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{}}"#),
        Stateless::NeedsState
    ));
}

#[test]
fn sniff_method_walks_top_level() {
    assert_eq!(
        sniff_method(r#"{"id":1,"method":"initialize","params":{}}"#),
        Some("initialize".into())
    );
    assert_eq!(
        sniff_method(r#"{"id":1,"method":"tools/call","params":{"name":"x"}}"#),
        Some("tools/call".into())
    );
    // Nested `method` keys must not shadow the real one.
    assert_eq!(
        sniff_method(r#"{"id":1,"method":"tools/list","params":{"method":"fake"}}"#),
        Some("tools/list".into())
    );
    assert_eq!(sniff_method("not even json"), None);
}

#[test]
fn sniff_id_raw_preserves_type() {
    assert_eq!(sniff_id_raw(r#"{"id":42,"method":"x"}"#), Some("42".into()));
    assert_eq!(
        sniff_id_raw(r#"{"id":"abc","method":"x"}"#),
        Some(r#""abc""#.into())
    );
}

#[test]
fn initialize_response_includes_protocol_and_capabilities() {
    let r = initialize_response("7");
    assert!(r.contains(r#""id":7"#));
    assert!(r.contains(r#""protocolVersion":"2024-11-05""#));
    assert!(r.contains(r#""tools""#));
    assert!(r.contains(r#""serverInfo""#));
    // Cross-stack parity with TS pen-mcp: serverInfo.name must be the shared
    // `openpencil-mcp` identity, and serverInfo.version must track the crate
    // version (TS sources its package.json) — guards against version drift.
    assert!(r.contains(r#""name":"openpencil-mcp""#));
    assert!(r.contains(&format!(r#""version":"{}""#, env!("CARGO_PKG_VERSION"))));
}
