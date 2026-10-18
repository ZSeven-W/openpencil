//! Retained cart controls isolate action-schema validation from business semantics.

use crate::script_gen::{parse_script, ScriptGenError};
use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

const RETAINED: &str = include_str!("test_fixtures/generated-cart-stepper-events-1018.json");

fn retained() -> Value {
    let fixture: Value = serde_json::from_str(RETAINED).unwrap();
    assert_eq!(
        fixture["source_sha256"],
        "c5c248e25dd1c8fdd11efa60a6f4dac42c83985524a6b8f84ae35f22fcfadeb1"
    );
    assert!(fixture["source_root_state"].get("cartQtyA").is_none());
    assert!(fixture["source_root_state"].get("cartQtyB").is_none());
    fixture
}

fn corrected_target_only(mut node: Value) -> Value {
    let pairs = node["events"]["onTap"][0]["set"].as_object_mut().unwrap();
    assert_eq!(pairs.len(), 1);
    let target = pairs.keys().next().unwrap().clone();
    let value = pairs.remove(&target).unwrap();
    pairs.insert(format!("$app.{target}"), value);
    node
}

#[test]
fn all_four_retained_cart_steppers_reject_bare_targets_without_mutation() {
    let fixture = retained();
    let controls = fixture["controls"].as_array().unwrap();
    assert_eq!(controls.len(), 4);
    for (original, expected_id) in controls.iter().zip(["n340", "n343", "n354", "n357"]) {
        assert_eq!(original["id"], expected_id);
        let node: PenNode = serde_json::from_value(original.clone()).unwrap();
        let before = serde_json::to_value(&node).unwrap();
        let message = super::validate(std::slice::from_ref(&node))
            .unwrap_err()
            .to_string();
        assert!(message.starts_with("generated-action-body:"), "{message}");
        assert!(message.contains("events.onTap[0]"), "{message}");
        assert!(message.contains("cartQty"), "{message}");
        assert!(message.contains("path does not start"), "{message}");
        assert_eq!(serde_json::to_value(&node).unwrap(), before);
        let script = format!("I(null, {original});");
        let error = parse_script(&script).expect_err("public generation gate must reject");
        assert!(
            matches!(&error, ScriptGenError::RuntimeExpression(_)),
            "{error}"
        );
        assert!(error.to_string().starts_with("generated-action-body:"));
    }
}

#[test]
fn correcting_targets_only_does_not_invent_missing_state_or_fix_no_op_minus() {
    let fixture = retained();
    for original in fixture["controls"].as_array().unwrap() {
        let corrected = corrected_target_only(original.clone());
        let node: PenNode = serde_json::from_value(corrected.clone()).unwrap();
        let before = serde_json::to_value(&node).unwrap();
        super::validate(std::slice::from_ref(&node)).unwrap();
        assert_eq!(serde_json::to_value(&node).unwrap(), before);
        let (nodes, state) = parse_script(&format!("I(null, {corrected});")).unwrap();
        assert!(
            state.is_empty(),
            "schema validation must not infer cart defaults"
        );
        assert_eq!(
            serde_json::to_value(&nodes[0]).unwrap()["events"],
            corrected["events"]
        );
        let (target, value) = corrected["events"]["onTap"][0]["set"]
            .as_object()
            .unwrap()
            .iter()
            .next()
            .unwrap();
        if corrected["name"].as_str().unwrap().contains("minus") {
            assert_eq!(
                value.as_str(),
                Some(target.as_str()),
                "original no-op remains explicit"
            );
        }
    }
    // These are valid expressions over undeclared state, a separate whole-document
    // initial-binding concern. This gate does not execute or rewrite their output.
    for display in fixture["displays"].as_array().unwrap() {
        let node: PenNode = serde_json::from_value(display.clone()).unwrap();
        let before = serde_json::to_value(&node).unwrap();
        super::validate(std::slice::from_ref(&node)).unwrap();
        assert_eq!(serde_json::to_value(&node).unwrap(), before);
    }
}

#[test]
fn structured_set_targets_use_real_scopes_and_do_not_require_app_only() {
    for target in ["cartQtyA", "cartQtyB"] {
        let source = format!(
            "I(null, {});",
            json!({
                "type":"frame", "events":{"onTap":[{"set":{"target":target,"value":"1"}}]}
            })
        );
        let error = parse_script(&source).unwrap_err();
        assert!(
            error.to_string().starts_with("generated-action-body:"),
            "{error}"
        );
    }
    for target in [
        "$app.cartQtyA",
        "$state.cartQtyA",
        "$page.cartQtyA",
        "$self.cartQtyA",
    ] {
        let actions = json!([{"set":{"target":target,"value":"1"}}]);
        let source = format!(
            "I(null, {});",
            json!({"type":"frame","events":{"onTap":actions}})
        );
        let (nodes, state) = parse_script(&source).unwrap();
        assert!(state.is_empty());
        assert_eq!(
            serde_json::to_value(&nodes[0]).unwrap()["events"]["onTap"],
            actions
        );
    }
}

#[test]
fn correcting_cart_targets_exposes_missing_initial_values_to_existing_check() {
    let fixture = retained();
    let mut children = fixture["controls"]
        .as_array()
        .unwrap()
        .iter()
        .cloned()
        .map(corrected_target_only)
        .collect::<Vec<_>>();
    children.extend(fixture["displays"].as_array().unwrap().iter().cloned());
    // Only this isolated in-memory counterpart changes targets. Keep the
    // original root state and all quantity/price bindings, including missing keys.
    let document = json!({
        "version":"1.0.0", "state":fixture["source_root_state"],
        "children":[{"type":"frame","id":"cart-section","children":children}]
    });
    let state =
        op_editor_core::EditorState::from_document(serde_json::from_value(document).unwrap());
    let before = serde_json::to_value(&state.doc).unwrap();
    let findings =
        crate::initial_binding_check::missing_initial_bindings(&state, &["cart-section".into()]);
    assert_eq!(findings.len(), 4, "{findings:?}");
    for (id, key, expected, authored) in [
        ("n342", "cartQtyA", "null", "1"),
        ("n345", "cartQtyA", "¥0", "¥89"),
        ("n356", "cartQtyB", "null", "1"),
        ("n359", "cartQtyB", "¥0", "¥89"),
    ] {
        let finding = findings
            .iter()
            .find(|finding| finding.node_id == id)
            .unwrap();
        assert_eq!(finding.state_key, key);
        assert_eq!(finding.initial_value, json!(expected));
        assert_eq!(finding.authored_value, Some(json!(authored)));
    }
    assert_eq!(serde_json::to_value(&state.doc).unwrap(), before);
}
