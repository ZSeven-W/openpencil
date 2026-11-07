use super::*;
use serde_json::json;

fn handlers(v: Value) -> EventHandlers {
    serde_json::from_value(v).unwrap()
}

#[test]
fn summarize_orders_known_triggers_then_unknown() {
    let events = handlers(json!({
        "onHoverEnter": [{ "show": "tip" }],
        "onTap": [{ "push": "\"/home\"" }, { "pop": null }],
        "onPanStart": [{ "set": { "$app.x": "1" } }],
    }));
    let views = summarize_events(Some(&events));
    assert_eq!(views.len(), 3);
    assert_eq!(views[0].trigger, Some(InteractionTrigger::Tap));
    assert_eq!(
        views[0].actions[0].kind,
        Some(InteractionActionKind::Navigate)
    );
    assert_eq!(views[0].actions[0].param, "/home");
    assert_eq!(views[0].actions[1].kind, Some(InteractionActionKind::Back));
    assert_eq!(views[1].trigger, Some(InteractionTrigger::HoverEnter));
    assert_eq!(views[1].actions[0].param, "tip");
    assert_eq!(views[2].trigger, None);
    assert_eq!(views[2].raw_key, "onPanStart");
    assert_eq!(views[2].actions[0].kind, None);
    assert_eq!(views[2].actions[0].raw_key, "set");
}

#[test]
fn params_round_trip_through_build_action() {
    use InteractionActionKind as K;
    let cases = [
        (K::Navigate, "/profile"),
        (K::Show, "panel-1"),
        (K::Hide, "panel-1"),
        (K::ToggleVisibility, "menu"),
        (K::ScrollTo, "footer"),
        (K::ToggleState, "open"),
        (K::Toast, "Saved \"draft\""),
        (K::OpenUrl, "https://openpencil.dev"),
        (K::Copy, "code-123"),
        (K::Delay, "450"),
    ];
    for (kind, param) in cases {
        let action = build_action(kind, param, None);
        let view = action_view(&action);
        assert_eq!(view.kind, Some(kind), "{action}");
        assert_eq!(view.param, param, "{action}");
    }
}

#[test]
fn built_actions_match_runtime_body_shapes() {
    use InteractionActionKind as K;
    assert_eq!(
        build_action(K::Navigate, "/a", None),
        json!({"push": "\"/a\""})
    );
    assert_eq!(
        build_action(K::Navigate, "/a", Some("replace")),
        json!({"replace": "\"/a\""})
    );
    assert_eq!(build_action(K::Back, "", None), json!({"pop": null}));
    assert_eq!(
        build_action(K::ToggleState, "is open!", None),
        json!({"toggle": "$app.isopen"})
    );
    assert_eq!(
        build_action(K::Delay, "abc", None),
        json!({"delay": {"ms": 300}})
    );
    assert_eq!(
        build_action(K::OpenUrl, "https://x.dev", None),
        json!({"open_url": {"url": "\"https://x.dev\""}})
    );
}

#[test]
fn unquote_accepts_single_and_double_quoted_literals() {
    assert_eq!(unquote_literal("'hi'"), "hi");
    assert_eq!(unquote_literal("\"hi\""), "hi");
    assert_eq!(unquote_literal("$app.name"), "$app.name");
}

#[test]
fn state_keys_are_sanitized() {
    assert_eq!(sanitize_state_key("$state.menu_open"), "menu_open");
    assert_eq!(sanitize_state_key("1x"), "_1x");
    assert_eq!(sanitize_state_key("--"), "");
}
