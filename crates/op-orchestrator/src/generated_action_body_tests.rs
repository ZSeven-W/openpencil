//! Retained restaurant events exercise the real registry, without execution.

use crate::script_gen::{parse_script, ScriptGenError};
use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

const RETAINED: &str = include_str!("test_fixtures/generated-restaurant-add-events-1018.json");

fn retained_buttons() -> Vec<Value> {
    let fixture: Value = serde_json::from_str(RETAINED).unwrap();
    assert_eq!(
        fixture["source_sha256"],
        "a51f0d9e49dc018eeab66207e7e7d052e4cb2113db31d62ab897b20a635da7d6"
    );
    fixture["buttons"].as_array().unwrap().clone()
}

fn event_script(actions: Value) -> String {
    format!(
        "I(null, {});",
        json!({"type":"frame", "name":"Retained action control", "events":{"onTap":actions}})
    )
}

fn assert_action_body(script: &str) -> String {
    let error = parse_script(script).expect_err("unparseable action chain must reject");
    // Reuse the existing self-check retry class; no new retry rung or API.
    assert!(
        matches!(&error, ScriptGenError::RuntimeExpression(_)),
        "{error}"
    );
    let message = error.to_string();
    assert!(message.starts_with("generated-action-body:"), "{message}");
    message
}

#[test]
fn all_six_original_add_buttons_reject_null_target_without_mutating_nodes() {
    let buttons = retained_buttons();
    assert_eq!(buttons.len(), 6);
    for (original, expected_id) in buttons
        .iter()
        .zip(["n222", "n233", "n245", "n256", "n267", "n278"])
    {
        assert_eq!(original["id"], expected_id);
        assert!(original["events"]["onTap"][0]["animate"]["target"].is_null());
        assert_eq!(original["events"]["onTap"][1]["toast"], "'已加入购物车'");
        let node: PenNode = serde_json::from_value(original.clone()).unwrap();
        let before = serde_json::to_value(&node).unwrap();
        let error = super::validate(std::slice::from_ref(&node)).unwrap_err();
        let message = error.to_string();
        assert!(message.starts_with("generated-action-body:"), "{message}");
        assert!(message.contains("events.onTap[0]"), "{message}");
        assert!(message.contains("target"), "{message}");
        assert!(message.contains("must be a string"), "{message}");
        assert_eq!(serde_json::to_value(&node).unwrap(), before);
        // Also cover the public generation boundary after JS expansion.
        assert_action_body(&event_script(original["events"]["onTap"].clone()));
    }
}

#[test]
fn correcting_only_each_target_keeps_all_retained_actions_and_visual_fields() {
    for mut corrected in retained_buttons() {
        let original = corrected.clone();
        corrected["events"]["onTap"][0]["animate"]["target"] = original["id"].clone();
        let node: PenNode = serde_json::from_value(corrected.clone()).unwrap();
        super::validate(&[node]).unwrap();
        let actions = corrected["events"]["onTap"].clone();
        let (nodes, state) = parse_script(&event_script(actions.clone())).unwrap();
        assert!(
            state.is_empty(),
            "the parser must not synthesize cart state"
        );
        assert_eq!(
            serde_json::to_value(&nodes[0]).unwrap()["events"]["onTap"],
            actions
        );
        corrected["events"]["onTap"][0]["animate"]["target"] = Value::Null;
        assert_eq!(corrected, original, "target is the only test-variant edit");
    }
}

#[test]
fn retained_bad_chain_is_rejected_inside_eager_factory_branches() {
    let bad = retained_buttons()[0]["events"]["onTap"].clone();
    for action in [
        json!({"if":{"expr":"false", "then":bad}}),
        json!({"if":{"expr":"true", "then":[], "else":bad}}),
        json!({"for_each":{"in":"[]", "as":"dish", "do":bad}}),
        json!({"parallel":[bad, []]}),
        json!({"race":[bad, []]}),
        json!({"fetch":{"url":"'/api/cart'", "on_error":bad}}),
        json!({"confirm":{"title":"'Cart'", "message":"'Ready?'", "on_confirm":bad}}),
        json!({"confirm":{"title":"'Cart'", "message":"'Ready?'", "on_cancel":bad}}),
    ] {
        let message = assert_action_body(&event_script(json!([action])));
        assert!(message.contains("target"), "{message}");
        assert!(message.contains("must be a string"), "{message}");
    }
}

#[test]
fn known_hooks_reject_unknown_actions_and_custom_shape_errors() {
    for action in [
        json!({"futureAction":{"opaque":"String("}}),
        json!({"pop":null, "animate":{"target":null}}),
        json!({"set":{"not-a-state-path":"1"}}),
    ] {
        assert_action_body(&event_script(json!([action])));
    }
}

#[test]
fn valid_registered_actions_and_ignored_fields_follow_the_runtime_parser() {
    parse_script(&event_script(json!([
        {"push":"'/screen-1'"}, {"replace":"'/'"}, {"pop":null},
        {"set":{"target":"$app.quantity", "value":"2", "opaque":"String("}},
        {"call":{"module":"cart", "function":"String", "args":["'String('"]}},
        {"toast":"'已加入购物车'"}
    ])))
    .unwrap();
}

#[test]
fn future_hook_payload_and_lazy_websocket_callback_remain_opaque() {
    let bad = retained_buttons()[0]["events"]["onTap"].clone();
    let source = format!(
        "I(null, {});",
        json!({
            "type":"frame", "events":{"onFutureGesture":bad}
        })
    );
    let (nodes, _) = parse_script(&source).unwrap();
    assert_eq!(
        serde_json::to_value(&nodes[0]).unwrap()["events"]["onFutureGesture"],
        bad
    );
    // This is a documented registry limit, not a promise of callback validity:
    // ws_connect stores raw on_message JSON and parses it only on delivery.
    let lazy = json!([{"ws_connect":{
        "id":"cart", "url":"'wss://example.invalid/cart'", "on_message":bad
    }}]);
    let (nodes, _) = parse_script(&event_script(lazy.clone())).unwrap();
    assert_eq!(
        serde_json::to_value(&nodes[0]).unwrap()["events"]["onTap"],
        lazy
    );
}

#[test]
fn temporarily_disabled_known_hook_still_requires_parseable_authored_body() {
    let source = format!(
        "I(null, {});",
        json!({
            "type":"frame", "gestures":{"disabledEvents":["onTap"]},
            "events":{"onTap":retained_buttons()[0]["events"]["onTap"]}
        })
    );
    assert_action_body(&source);
}

#[test]
fn action_feedback_bounds_and_escapes_untrusted_names_keys_and_bodies() {
    let source = format!(
        "I(null, {});",
        json!({
            "type":"frame", "name":"\"\n\t菜单".repeat(3000),
            "events":{"onTap":[{("\"\n\t未知".repeat(3000)):{"body":"\"\n\t内容".repeat(3000)}}]}
        })
    );
    let message = assert_action_body(&source);
    assert!(message.len() < 4096, "{} bytes", message.len());
    assert!(!message.chars().any(char::is_control), "{message:?}");
    assert!(message.contains("\\n\\t"), "{message}");
    assert!(message.contains("preserve its content, layout, state and interactions"));
    assert!(!message.contains("quotes inside the expression string"));
}
