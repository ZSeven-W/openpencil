//! Expanded generation output must use valid Jian only in known runtime fields.

use crate::script_gen::{parse_script, ScriptGenError};
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use serde_json::{json, Value};

const RETAINED: &str = include_str!("test_fixtures/generated-cart-bindings-1018.js");
const RETAINED_CART_SYNTAX: &str =
    include_str!("test_fixtures/generated-cart-runtime-syntax-1018.js");
const RETAINED_PRODUCT_SYNTAX: &str =
    include_str!("test_fixtures/generated-product-runtime-syntax-1018.js");

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

fn assert_action_body(source: &str) -> String {
    let error = parse_script(source).expect_err("invalid action body must reject");
    assert!(
        matches!(&error, ScriptGenError::RuntimeExpression(_)),
        "{error}"
    );
    let message = error.to_string();
    assert!(message.starts_with("generated-action-body:"), "{message}");
    message
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

fn assert_syntax(source: &str, site: &str) -> String {
    let error = parse_script(source).expect_err("invalid known Jian expression must reject");
    assert!(
        matches!(&error, ScriptGenError::RuntimeExpression(_)),
        "{error}"
    );
    let message = error.to_string();
    assert!(
        message.contains("generated-expression-syntax:"),
        "{message}"
    );
    assert!(message.contains("syntaxError"), "{message}");
    assert!(message.contains(site), "{message}");
    assert!(!message.contains("unknownFunction"), "{message}");
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
        {"set":{"target":"$app.quantity", "value":"to_num('2')", "note":"String(2)"}},
        {"if":{"expr":"true", "then":[], "futureExpr":"String(2)"}}
    ]);
    parse_script(&event_script("onTap", actions)).unwrap();
}

#[test]
fn unparseable_action_bodies_are_rejected_by_the_registry_gate() {
    for action in [
        json!({"set":{"target":"$app.quantity", "value":2, "note":"String(2)"}}),
        json!({"set":{"$app.quantity":"String(2)", "$app.other":2}}),
        json!({"if":{"expr":"String(1)"}}),
        json!({"set":{"$app.quantity":"String(2)"}, "futureAction":{}}),
    ] {
        assert_action_body(&event_script("onTap", json!([action])));
    }
}

#[test]
fn compiler_failures_in_known_sites_are_syntax_errors_not_unknown_functions() {
    for property in ["content", "text", "opacity", "fill[0].color", "bind:value"] {
        for source in [
            "String(",
            "String(1) +",
            "Math.max(1, 2)",
            "$app.price.toFixed(2)",
        ] {
            assert_syntax(
                &binding_script(property, source),
                &format!("bindings.{property}"),
            );
        }
    }
    let action = json!([{"set":{"$app.quantity":"String(1) +"}}]);
    assert_syntax(
        &event_script("onTap", action),
        "events.onTap[0].set.$app.quantity",
    );
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

#[test]
fn set_string_values_are_expressions_in_both_runtime_forms() {
    for source in ["350ml", "奶油白", "String(1) +"] {
        for action in [
            json!({"set":{"$state.capacity":source}}),
            json!({"set":{"target":"$state.capacity", "value":source, "note":"opaque"}}),
        ] {
            assert_syntax(&event_script("onTap", json!([action])), ".set.");
        }
    }
    for source in ["'350ml'", "'奶油白'", "2", "false", "'String('"] {
        parse_script(&event_script(
            "onTap",
            json!([{"set":{"$state.capacity":source}}]),
        ))
        .unwrap();
    }
}

#[test]
fn toast_validates_only_its_runtime_message_expression() {
    for source in ["已加入购物车", "'unterminated", "String(1) +"] {
        assert_syntax(
            &event_script("onTap", json!([{"toast":source}])),
            "events.onTap[0].toast",
        );
        assert_syntax(
            &event_script("onTap", json!([{"toast":{"message":source}}])),
            ".toast.message",
        );
    }
    for action in [
        json!({"toast":"'已加入购物车'"}),
        json!({"toast":{"message":"to_str($app.qty)", "duration_ms":2000, "level":"success", "note":"String("}}),
        json!({"toast":{"message":"'Ready'", "duration_ms":"String(", "level":"未解析("}}),
    ] {
        parse_script(&event_script("onTap", json!([action]))).unwrap();
    }
    for action in [
        json!({"toast":"String(2)"}),
        json!({"toast":{"message":"String(2)"}}),
    ] {
        assert_unknown(&event_script("onTap", json!([action])), "String");
    }
}

#[test]
fn factory_expression_errors_do_not_hide_nested_if_sites() {
    for body in [
        json!({"expr":"true +", "then":[]}),
        json!({"expr":"true", "then":[{"set":{"$state.capacity":"350ml"}}]}),
        json!({"expr":"true", "then":[], "else":[{"toast":"裸中文"}]}),
        json!({"expr":"false", "then":[{"if":{"expr":"true", "then":[], "else":[{"toast":{"message":"裸中文"}}]}}]}),
    ] {
        assert_syntax(
            &event_script("onTap", json!([{"if":body}])),
            "events.onTap[0].if.",
        );
    }
    let actions = json!([{"if":{
        "expr":"true", "then":[{"call":{"module":"cart", "function":"custom", "args":["'String('"]}}],
        "else":[{"toast":"'Ready'"}], "futureExpr":"String("
    }}]);
    parse_script(&event_script("onTap", actions)).unwrap();
    let future = json!([{"if":{"expr":"true", "then":[{"futureAction":{"expr":"String("}}]}}]);
    assert_action_body(&event_script("onTap", future));
}

#[test]
fn unknown_sites_and_ignored_action_fields_remain_opaque() {
    for property in ["futureProperty", "bind:futureProperty"] {
        parse_script(&binding_script(property, "String(")).unwrap();
    }
    parse_script(&event_script(
        "onFutureGesture",
        json!([{"toast":"裸中文"}]),
    ))
    .unwrap();
    let valid = json!([
        {"push":"'/商品/350ml'"},
        {"set":{"target":"$state.qty", "value":"2", "note":"String("}}
    ]);
    parse_script(&event_script("onTap", valid)).unwrap();
    // Each body is checked independently so the first bad action cannot mask
    // another formerly deferred shape. Unknown hooks remain distinct above.
    for action in [
        json!({"push":"/商品/350ml"}),
        json!({"push":{"path":"/商品/350ml", "note":"String("}}),
        json!({"futureAction":{"expr":"String(", "toast":"裸中文"}}),
        json!({"set":{"target":"$state.qty", "value":2}}),
        json!({"set":{"$state.enabled":true}}),
        json!({"if":{"expr":true, "then":[]}}),
        json!({"toast":false}),
        json!({"toast":{"message":2}}),
        json!({"set":{"$state.qty":"String("}, "futureAction":{}}),
        json!({"call":{"module":"cart", "function":"custom", "args":["String("]}}),
    ] {
        assert_action_body(&event_script("onTap", json!([action])));
    }
}

#[test]
fn outer_javascript_methods_remain_valid_and_do_not_rewrite_runtime_strings() {
    let source = r#"
        const price = (12.3).toFixed(2);
        const capacity = "350ml";
        I(null, {type:"text", name:"Price", content:price,
          bindings:{content:"to_str(round($state.price, 2))"},
          events:{onTap:[{set:{"$state.capacity":JSON.stringify(capacity)}},{toast:"'已加入购物车'"}]}});
    "#;
    let (nodes, _) = parse_script(source).unwrap();
    let value = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(value["content"], "12.30");
    assert_eq!(
        value["events"]["onTap"][0]["set"]["$state.capacity"],
        "\"350ml\""
    );
    assert_eq!(
        value["bindings"]["content"],
        "to_str(round($state.price, 2))"
    );
}

#[test]
fn retained_real_scripts_reject_each_runtime_syntax_defect_without_changing_bytes() {
    for source in [RETAINED_CART_SYNTAX, RETAINED_PRODUCT_SYNTAX] {
        assert_syntax(source, "events.onTap");
    }
    // Use the same builder and executor to inspect the original forest without
    // bypassing the gate's rejection through a rewritten fixture.
    let expand = |source| {
        let program = op_mcp::script_runner::run_generation_script_to_program(source).unwrap();
        crate::program_gen::run_program_to_forest(&program)
            .unwrap()
            .0
    };
    let cart = expand(RETAINED_CART_SYNTAX);
    let product = expand(RETAINED_PRODUCT_SYNTAX);
    for (nodes, name, site, source) in [
        (&cart, "管理按钮", "events.onTap[0].toast", "进入管理模式"),
        (&cart, "汇总行-商品合计", "bindings.content", ".toFixed(2)"),
        (&cart, "合计行", "bindings.content", ".toFixed(2)"),
        (
            &product,
            "chip-capacity-350ml",
            "events.onTap[0].set.$state.capacity",
            "350ml",
        ),
        (
            &product,
            "chip-color-奶油白",
            "events.onTap[0].set.$state.color",
            "奶油白",
        ),
    ] {
        let node = find_named(nodes, name).expect(name);
        let before = serde_json::to_value(node).unwrap();
        let error = super::validate(std::slice::from_ref(node)).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("generated-expression-syntax:"),
            "{message}"
        );
        assert!(message.contains(site), "{message}");
        assert!(message.contains(source), "{message}");
        assert_eq!(serde_json::to_value(node).unwrap(), before);
    }
    assert!(RETAINED_CART_SYNTAX.contains("p.price.toFixed(2)"));
    let price = serde_json::to_value(find_named(&cart, "价格-p1").unwrap()).unwrap();
    assert_eq!(price["content"], "¥89.00");
}

#[test]
fn syntax_feedback_is_bounded_escaped_and_keeps_fixed_repair_instruction() {
    let name = "\"\n\t商品".repeat(3_000);
    let source = format!("{}String(", " \n\t".repeat(4_000));
    let script = node_script(
        json!({"type":"text", "name":name, "content":"1", "bindings":{"content":source}}),
    );
    let message = assert_syntax(&script, "bindings.content");
    assert!(message.len() < 4_096, "{} bytes", message.len());
    assert!(!message.chars().any(char::is_control), "{message:?}");
    assert!(message.contains("\\n\\t"));
    assert!(message.contains("preserve its content, layout, state and interactions"));
    assert!(message.contains("quotes inside the expression string"));
}
