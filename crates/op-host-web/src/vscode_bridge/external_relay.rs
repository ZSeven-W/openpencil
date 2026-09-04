//! Authorization boundary for external navigation from the nested editor.
//!
//! The verification URI is untrusted until the current bridge token proves
//! this page is attached to a `managed` daemon. Keep it in the async callback,
//! then re-check the page-lifetime Init lock before sending it to the exact
//! extension origin.

use std::rc::Rc;

use js_sys::Object;
use wasm_bindgen::JsValue;

use super::BRIDGE_ORIGIN;

#[derive(Clone, Debug, PartialEq, Eq)]
struct RelayState {
    origin: String,
    token: String,
}

/// Pure capture rule: a relay exists only after Init locked a non-empty origin
/// and installed a non-empty token, and only while the page has a real parent.
fn relay_state(
    origin: Option<&str>,
    token: Option<&str>,
    has_distinct_parent: bool,
) -> Option<RelayState> {
    if !has_distinct_parent {
        return None;
    }
    let origin = origin.filter(|value| !value.is_empty())?;
    let token = token.filter(|value| !value.is_empty())?;
    Some(RelayState {
        origin: origin.to_string(),
        token: token.to_string(),
    })
}

fn relay_state_matches(
    expected: &RelayState,
    origin: Option<&str>,
    token: Option<&str>,
    has_distinct_parent: bool,
) -> bool {
    relay_state(origin, token, has_distinct_parent).as_ref() == Some(expected)
}

/// A network answer authorizes navigation only when it is an HTTP success and
/// parsed JSON names the managed daemon mode exactly. Error text, substring
/// matches, local/online servers, and older bodies are all denied.
fn managed_probe_allows_relay(http_status: u16, body: &str) -> bool {
    if http_status != 200 {
        return false;
    }
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    parsed.get("serveMode").and_then(serde_json::Value::as_str) == Some("managed")
}

/// Return the page and its distinct parent without accessing cross-origin
/// properties. WindowProxy identity is safe to compare across origins.
fn distinct_parent() -> Option<(web_sys::Window, web_sys::Window)> {
    let window = web_sys::window()?;
    let parent = window.parent().ok().flatten()?;
    if Object::is(window.self_().as_ref(), parent.as_ref()) {
        return None;
    }
    Some((window, parent))
}

fn current_state(has_distinct_parent: bool) -> Option<RelayState> {
    let origin = BRIDGE_ORIGIN.with(|slot| slot.borrow().clone());
    let token = crate::live_sync::bridge_token();
    relay_state(origin.as_deref(), token.as_deref(), has_distinct_parent)
}

fn current_state_matches(expected: &RelayState, has_distinct_parent: bool) -> bool {
    let origin = BRIDGE_ORIGIN.with(|slot| slot.borrow().clone());
    let token = crate::live_sync::bridge_token();
    relay_state_matches(
        expected,
        origin.as_deref(),
        token.as_deref(),
        has_distinct_parent,
    )
}

/// Start the proof request. The URL is moved into the callback and is never
/// posted before the response passes all checks.
pub(super) fn request(url: &str) {
    let Some((_window, _parent)) = distinct_parent() else {
        return;
    };
    let Some(expected) = current_state(true) else {
        return;
    };

    let verification_url = url.to_string();
    let probe_url = crate::daemon_base::daemon_url("/api/mcp/server");
    let _ = crate::live_sync::get_with_status(
        &probe_url,
        Rc::new(move |http_status, body| {
            if !managed_probe_allows_relay(http_status, &body) {
                return;
            }

            // Re-read every authority input after the await. A new Init, token
            // rotation, navigation out of the frame, or parent replacement
            // invalidates this particular URL.
            let Some((_window, parent)) = distinct_parent() else {
                return;
            };
            if !current_state_matches(&expected, true) {
                return;
            }

            let message = serde_json::json!({
                "type": "op-shell/open-external",
                "url": verification_url,
            })
            .to_string();
            let _ = parent.post_message(&JsValue::from_str(&message), &expected.origin);
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_state_requires_locked_init_and_real_parent() {
        assert_eq!(relay_state(None, Some("token"), true), None);
        assert_eq!(relay_state(Some("https://host"), None, true), None);
        assert_eq!(
            relay_state(Some("https://host"), Some("token"), false),
            None
        );
        assert_eq!(relay_state(Some(""), Some("token"), true), None);
        assert_eq!(relay_state(Some("https://host"), Some(""), true), None);
        assert_eq!(
            relay_state(Some("https://host"), Some("token"), true),
            Some(RelayState {
                origin: "https://host".into(),
                token: "token".into(),
            })
        );
    }

    #[test]
    fn only_a_successful_managed_probe_authorizes_the_relay() {
        assert!(!managed_probe_allows_relay(
            401,
            r#"{"serveMode":"managed"}"#
        ));
        assert!(!managed_probe_allows_relay(200, r#"{"serveMode":"local"}"#));
        assert!(!managed_probe_allows_relay(
            200,
            r#"{"serveMode":"online"}"#
        ));
        assert!(!managed_probe_allows_relay(200, "serveMode=managed"));
        assert!(managed_probe_allows_relay(
            200,
            r#"{"running":true,"serveMode":"managed"}"#
        ));
    }

    #[test]
    fn post_await_recheck_rejects_origin_token_or_parent_changes() {
        let expected = relay_state(Some("https://host"), Some("token"), true).unwrap();
        assert!(relay_state_matches(
            &expected,
            Some("https://host"),
            Some("token"),
            true
        ));
        assert!(!relay_state_matches(
            &expected,
            Some("https://replacement"),
            Some("token"),
            true
        ));
        assert!(!relay_state_matches(
            &expected,
            Some("https://host"),
            Some("rotated"),
            true
        ));
        assert!(!relay_state_matches(
            &expected,
            Some("https://host"),
            Some("token"),
            false
        ));
    }
}
