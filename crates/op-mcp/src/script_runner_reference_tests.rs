//! Runtime-expression scopes and generation-time node handles have different repairs.

use super::*;

#[test]
fn unquoted_runtime_scopes_explain_expression_quotation_not_stale_handles() {
    for scope in ["$state", "$app"] {
        let source = format!(
            "const row = I(null, {{type:'frame'}});\n\
             I(row, {{type:'text', content:'A', bindings:{{content:'Selected: ' + {scope}.choice}}}});"
        );
        let error = run_generation_script_to_program(&source).unwrap_err();
        assert!(
            matches!(&error, ScriptError::RuntimeBindingReference { scope: actual } if actual == scope)
        );
        let message = error.to_string();
        assert!(message.contains(&format!("{scope} is not defined")));
        assert!(message.contains("Quote the ENTIRE runtime expression"));
        assert!(message.contains("typed state"));
        assert!(!message.contains("earlier batch"));
        assert!(!message.contains(&format!("I({scope},")));
    }
}

#[test]
fn a_missing_node_handle_still_gets_the_stale_reference_explanation() {
    let error =
        run_generation_script_to_program("I(previousRow, {type:'text',content:'A'});").unwrap_err();
    assert!(matches!(&error, ScriptError::StaleReference { name, .. } if name == "previousRow"));
    assert!(error.to_string().contains("earlier batch"));
    assert!(!error.to_string().contains("Preview runtime scope"));
}

#[test]
fn quoted_runtime_expression_remains_a_binding_in_the_recorded_program() {
    let source = r#"const row=I(null,{type:'frame',state:{choice:{type:'string',default:'A'}}});
I(row,{type:'text',content:'Selected: A',bindings:{content:"'Selected: ' + $app.choice"},
events:{onTap:[{set:{'$app.choice':"'B'"}}]}});"#;
    let program = run_generation_script_to_program(source).unwrap();
    let nodes = crate::parse_program_objects(&program);
    assert_eq!(nodes[0].1["state"]["choice"]["default"], "A");
    assert_eq!(
        nodes[1].1["bindings"]["content"],
        "'Selected: ' + $app.choice"
    );
    assert_eq!(
        nodes[1].1["events"]["onTap"][0]["set"]["$app.choice"],
        "'B'"
    );
}

#[test]
fn retained_product_spec_failures_identify_unquoted_runtime_builtins() {
    // Exact responses from two failed 1018 batches; provenance is retained
    // beside the proposal. Do not repair the original action or JS literals.
    // A thrown reference must reject the whole script, never return its prefix.
    for source in [
        include_str!("../tests/fixtures/script_runner/product-spec-unquoted-1018.js"),
        include_str!("../tests/fixtures/script_runner/product-spec-argument-quoted-1018.js"),
        include_str!("../tests/fixtures/script_runner/product-spec-actions-batch-unquoted-1018.js"),
    ] {
        let error = run_generation_script_to_program(source).unwrap_err();
        assert!(
            matches!(&error, ScriptError::RuntimeBuiltinReference { name } if name == "to_str")
        );
        let message = error.to_string();
        assert!(message.contains("quote the ENTIRE expression"));
        assert!(message.contains(r#"bindings: {content: "to_str($app.qty)"}"#));
        assert!(message.contains("quoting only the argument is insufficient"));
        assert!(!message.contains("earlier batch"));
        assert!(!message.contains("I(to_str,"));
    }
}

#[test]
fn builtin_classification_follows_the_runtime_registry_without_inventing_function_names() {
    for name in jian_core::expression::builtins::default_builtins().keys() {
        assert!(matches!(
            ScriptError::runtime_reference(name),
            Some(ScriptError::RuntimeBuiltinReference { name: actual }) if actual == *name
        ));
    }
    assert!(ScriptError::runtime_reference("previousRow").is_none());
    assert!(ScriptError::runtime_reference("to_str_not_a_builtin").is_none());
}

#[test]
fn builtin_name_collision_does_not_assert_a_stale_node_or_force_expression_quotation() {
    // `first` is a runtime builtin and a plausible author-chosen node handle.
    let error =
        run_generation_script_to_program("I(first, {type:'text',content:'A'});").unwrap_err();
    assert!(matches!(&error, ScriptError::RuntimeBuiltinReference { name } if name == "first"));
    let message = error.to_string();
    assert!(message.contains("If you intended a binding/action expression"));
    assert!(message.contains("If it is a JavaScript node handle or helper"));
    assert!(message.contains("declare it in this complete script"));
    assert!(!message.contains("earlier batch"));
}

#[test]
fn quoted_builtin_expressions_and_declared_javascript_helpers_remain_valid() {
    let source = r#"const root=I(null,{type:'frame',state:{qty:{type:'int',default:1}}});
I(root,{type:'text',content:String(1),bindings:{content:"to_str($app.qty)"},
events:{onTap:[{set:{'$app.qty':"to_num($app.qty) + 1"}}]}});
const to_str = value => String(value);
I(root,{type:'text',content:to_str(2)});"#;
    let program = run_generation_script_to_program(source).unwrap();
    let nodes = crate::parse_program_objects(&program);
    assert_eq!(nodes[1].1["content"], "1");
    assert_eq!(nodes[1].1["bindings"]["content"], "to_str($app.qty)");
    assert_eq!(
        nodes[1].1["events"]["onTap"][0]["set"]["$app.qty"],
        "to_num($app.qty) + 1"
    );
    assert_eq!(nodes[2].1["content"], "2");
}
