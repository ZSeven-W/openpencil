//! Expanded generation output must use Jian functions only in known runtime fields.

use crate::script_gen::{parse_script, ScriptGenError};
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use serde_json::{json, Value};

const RETAINED: &str = include_str!("test_fixtures/generated-cart-bindings-1018.js");

fn node_script(node: Value) -> String {
    format!("I(null, {node});")
}

fn binding_script(property: &str, source: &str) -> String {
    let mut node = json!({"type":"text", "name":"Bound quantity", "content":"1"});
    node["bindings"] = json!({(property): source});
    node_script(node)
}

fn event_script(hook: &str, actions: Value) -> String {
    let mut node = json!({"type":"frame", "name":"Quantity control"});
    node["events"] = json!({(hook): actions});
    node_script(node)
}

fn assert_unknown(source: &str, name: &str) -> String {
    let error =
        parse_script(source).expect_err("unknown Jian call must reject the generated forest");
    assert!(
        matches!(&error, ScriptGenError::RuntimeExpression(_)),
        "{error}"
    );
    let message = error.to_string();
    assert!(
        message.contains("generated-expression-unknown-function"),
        "{message}"
    );
    assert!(message.contains(name), "{message}");
    message
}

fn find_named<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a PenNode> {
    nodes.iter().find_map(|node| {
        if node.base().name.as_deref() == Some(name) {
            Some(node)
        } else {
            find_named(node.children().map(Vec::as_slice).unwrap_or(&[]), name)
        }
    })
}

#[test]
fn retained_cart_binding_rejects_javascript_string_after_loop_expansion() {
    assert_eq!(RETAINED.matches("content:String(it.qty)").count(), 1);
    assert_eq!(RETAINED.matches("bindings:{content:\"String(").count(), 1);
    let message = assert_unknown(RETAINED, "String");
    assert!(message.contains("to_str"), "{message}");
    assert!(message.contains("qty_product-1"), "{message}");
}

#[test]
fn correcting_only_runtime_conversion_preserves_javascript_and_both_cart_rows() {
    let corrected = RETAINED.replace("bindings:{content:\"String(", "bindings:{content:\"to_str(");
    assert!(corrected.contains("content:String(it.qty)"));
    let (nodes, state) = parse_script(&corrected).expect("corrected Jian bindings are valid");
    assert_eq!(nodes.len(), 1);
    assert!(
        state.is_empty(),
        "missing initial state is outside this gate's scope"
    );
    for product in ["product-1", "product-2"] {
        let quantity = find_named(&nodes, &format!("qty-value-{product}")).unwrap();
        let value = serde_json::to_value(quantity).unwrap();
        assert_eq!(
            value["content"], "1",
            "generation-time String remains JavaScript"
        );
        assert_eq!(
            value["bindings"]["content"],
            format!("to_str($app.qty_{product})")
        );
        assert!(find_named(&nodes, &format!("qty-minus-{product}")).is_some());
        assert!(find_named(&nodes, &format!("qty-plus-{product}")).is_some());
    }
}

#[test]
fn hyphenated_keys_and_spaced_subtraction_keep_jian_semantics() {
    use jian_core::expression::{bytecode::OpCode, Expression};
    use jian_core::state::{Scope, Segment, StatePath};

    let expression = Expression::compile("to_str($app.qty_product-1)").unwrap();
    assert_eq!(expression.chunk.scope_paths, ["$app.qty_product-1"]);
    assert!(!expression.chunk.ops.contains(&OpCode::Sub));
    let subtract = Expression::compile("to_str($app.qty_product - 1)").unwrap();
    assert_eq!(subtract.chunk.scope_paths, ["$app.qty_product"]);
    assert!(subtract.chunk.ops.contains(&OpCode::Sub));
    for path in ["$app.qty_product-1", "$app['qty_product-1']"] {
        let parsed = StatePath::parse(path).unwrap();
        assert_eq!(parsed.scope, Scope::App);
        assert_eq!(parsed.segments, [Segment::Key("qty_product-1".into())]);
    }
    for source in [
        "to_str($app.qty_product-1)",
        "to_str($app['qty_product-1'])",
    ] {
        parse_script(&binding_script("content", source)).expect(source);
    }
}

#[test]
fn registered_builtins_nested_calls_and_dynamic_reads_are_accepted() {
    for source in [
        "to_str(max(1, $app.quantity))",
        "to_num('2') + abs(-1)",
        "upper(trim(' cart '))",
        "len($app.items)",
        "round(12.345, 2)",
        "to_bool($state.enabled)",
        "typeof($app.quantity)",
        "is_null($app.missing)",
        "to_str($app[$item.key])",
        "'String($app.quantity)'",
    ] {
        parse_script(&binding_script("content", source)).expect(source);
    }
}

#[test]
fn unknown_call_is_rejected_in_supported_binding_targets_including_two_way_value() {
    for property in ["content", "text", "opacity", "fill[0].color", "bind:value"] {
        assert_unknown(&binding_script(property, "String($app.quantity)"), "String");
    }
}

#[test]
fn unknown_call_in_an_unselected_ternary_branch_is_still_rejected() {
    assert_unknown(
        &binding_script("content", "false ? String($app.quantity) : to_str(1)"),
        "String",
    );
}

#[test]
fn default_registry_validation_is_not_a_string_constructor_blacklist() {
    for (source, name) in [
        ("Number('2')", "Number"),
        ("parseInt('2')", "parseInt"),
        ("not_registered(1)", "not_registered"),
    ] {
        assert_unknown(&binding_script("content", source), name);
    }
}

#[test]
fn standard_set_forms_validate_the_actual_right_hand_expression() {
    for action in [
        json!({"set":{"$app.quantity":"String(2)"}}),
        json!({"set":{"target":"$app.quantity", "value":"String(2)"}}),
    ] {
        assert_unknown(&event_script("onTap", json!([action])), "String");
    }
    for action in [
        json!({"set":{"$app.qty_product-1":"max(1, $app.qty_product-1 - 1)"}}),
        json!({"set":{"target":"$app['qty_product-1']", "value":"to_num('2')"}}),
    ] {
        parse_script(&event_script("onTap", json!([action]))).unwrap();
    }
}

#[test]
fn known_event_hooks_validate_set_expressions() {
    for hook in [
        "onTap",
        "onChange",
        "onSubmit",
        "onRawPointer",
        "onReachEnd",
    ] {
        assert_unknown(
            &event_script(hook, json!([{"set":{"$app.quantity":"String(2)"}}])),
            "String",
        );
    }
}

#[test]
fn if_conditions_then_and_else_branches_validate_runtime_expressions() {
    for body in [
        json!({"expr":"String(1)", "then":[]}),
        json!({"expr":"true", "then":[{"set":{"$app.quantity":"String(2)"}}]}),
        json!({"expr":"true", "then":[], "else":[{"set":{"target":"$app.quantity", "value":"String(2)"}}]}),
        json!({"expr":"false", "then":[{"if":{"expr":"true", "then":[], "else":[{"set":{"$app.quantity":"String(2)"}}]}}]}),
    ] {
        assert_unknown(&event_script("onTap", json!([{"if":body}])), "String");
    }
    let valid = json!([{"if":{
        "expr":"to_bool($app.enabled)",
        "then":[{"set":{"$app.quantity":"max(1, $app.quantity - 1)"}}],
        "else":[{"set":{"target":"$app.quantity", "value":"to_num('2')"}}]
    }}]);
    parse_script(&event_script("onTap", valid)).unwrap();
}

#[test]
fn generation_javascript_and_arbitrary_text_are_not_jian_expressions() {
    let source = r#"
        const example = "String($app.quantity)";
        const quantity = String(2);
        const sec = I(null, {type:"frame", name:example});
        I(sec, {type:"text", name:"Example", content:example});
        I(sec, {type:"text", name:"Quantity", content:quantity});
    "#;
    let (nodes, _) = parse_script(source).unwrap();
    let quantity = serde_json::to_value(find_named(&nodes, "Quantity").unwrap()).unwrap();
    assert_eq!(quantity["content"], "2");
}

#[test]
fn unknown_binding_targets_and_future_event_hooks_are_preserved_without_scanning() {
    parse_script(&binding_script("futureProperty", "String($app.quantity)")).unwrap();
    parse_script(&binding_script(
        "bind:futureProperty",
        "String($app.quantity)",
    ))
    .unwrap();
    let (nodes, _) = parse_script(&event_script(
        "onFutureGesture",
        json!([{"set":{"$app.quantity":"String(2)"}}]),
    ))
    .unwrap();
    let value = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(
        value["events"]["onFutureGesture"][0]["set"]["$app.quantity"],
        "String(2)"
    );
}

#[test]
fn custom_call_actions_and_unknown_action_fields_are_not_builtin_calls() {
    let actions = json!([
        {"call":{"module":"cart", "function":"String", "args":["'String($app.quantity)'"]}},
        {"futureAction":{"expr":"String(2)", "set":{"$app.quantity":"String(2)"}}},
        {"set":{"target":"$app.quantity", "value":"to_num('2')", "note":"String(2)"}},
        {"if":{"expr":"true", "then":[], "futureExpr":"String(2)"}}
    ]);
    parse_script(&event_script("onTap", actions)).unwrap();
}

#[test]
fn unparseable_actions_are_deferred_to_existing_runtime_validation() {
    for action in [
        json!({"set":{"target":"$app.quantity", "value":2, "note":"String(2)"}}),
        json!({"set":{"$app.quantity":"String(2)", "$app.other":2}}),
        json!({"if":{"expr":"String(1)"}}),
        json!({"set":{"$app.quantity":"String(2)"}, "futureAction":{}}),
    ] {
        parse_script(&event_script("onTap", json!([action]))).unwrap();
    }
}

#[test]
fn compiler_failures_are_outside_the_unknown_builtin_gate() {
    for source in ["String(", "String(1) +", "Math.max(1, 2)"] {
        parse_script(&binding_script("content", source)).expect(source);
    }
    let action = json!([{"set":{"$app.quantity":"String(1) +"}}]);
    parse_script(&event_script("onTap", action)).unwrap();
}

#[test]
fn feedback_bounds_and_escapes_generated_unicode_names_and_long_sources() {
    let source = format!("String(\n\t{}1)", " ".repeat(12_000));
    let name = format!("购物车\n\t{}", "数量".repeat(3_000));
    let script = node_script(json!({
        "type":"text", "name":name, "content":"1", "bindings":{"content":source}
    }));
    let message = assert_unknown(&script, "String");
    assert!(
        message.len() < 4_096,
        "feedback must fit within the retry prompt"
    );
    assert!(!message.chars().any(char::is_control), "{message:?}");
    assert!(
        message.contains("\\n\\t"),
        "control characters must be JSON escaped"
    );
    assert!(message.contains("to_str"));
}
