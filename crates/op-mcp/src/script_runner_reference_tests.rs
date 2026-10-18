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
