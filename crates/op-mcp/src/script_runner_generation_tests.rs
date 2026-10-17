//! Fresh-generation parent contracts and ordinary MCP compatibility.

use crate::script_runner::{
    run_generation_script_to_program, run_modification_script_to_program, run_script_to_program,
};
use crate::{McpTool, ToolOutcome};
use op_editor_core::{EditorState, PenNodeExt};
use std::collections::BTreeMap;

#[test]
fn generation_rejects_unresolved_names_ids_and_root_aliases() {
    for parent in ["结算按钮", "authored-button", "root", "null", "b99"] {
        let source = format!(
            r#"const sec=I(null,{{type:"frame"}});
I(sec,{{id:"authored-button",type:"frame",name:"结算按钮"}});
I({parent:?},{{type:"text",content:"结算 (2)"}});"#
        );
        let error = run_generation_script_to_program(&source).unwrap_err();
        assert!(error.is_invalid_insertion_parent(), "{error}");
        assert!(error.to_string().contains(parent), "{error}");
        assert!(error
            .to_string()
            .contains("Save the value returned by I() or K()"));
    }
}

#[test]
fn generation_parent_errors_cannot_be_hidden_by_syntax_recovery() {
    let prefix = r#"const sec=I(null,{type:"frame"});
I(sec,{type:"frame",name:"结算按钮"});
I("结算按钮",{type:"text",content:"结算 (2)"})"#;
    for source in [
        prefix.to_owned(),
        format!("{prefix}; I(sec, {{type:"),
        prefix.replace("type:\"frame\"", "type:\"frame\",align.items:'center'"),
        prefix.replace("结算 (2)", "结算\n(2)"),
        prefix.replace("content:\"结算 (2)\"})", "content:\"结算 (2)\")"),
    ] {
        let error = run_generation_script_to_program(&source).unwrap_err();
        assert!(
            error.to_string().contains("OP_SCRIPT_PARENT_UNRESOLVED"),
            "{error}"
        );
    }
}

#[test]
fn generation_accepts_returned_i_and_k_handles_and_explicit_null_state_roots() {
    let program = run_generation_script_to_program(
        r#"I(null,{type:"frame",state:{quantity:{type:"int",default:2}}});
const sec=I(null,{type:"frame",name:"Section"});
const button=I(sec,{type:"frame",name:"结算按钮"});
I(button,{type:"text",content:"结算 (2)"});
const kit=K("shadcn/btn-primary",sec,{});
I(kit,{type:"text",content:"Continue"});"#,
    )
    .unwrap();
    let lines: Vec<_> = program.lines().collect();
    assert!(lines[0].starts_with("b0=I(null, "), "{program}");
    assert!(lines[1].starts_with("b1=I(null, "), "{program}");
    assert!(lines[3].starts_with("b3=I(b2, "), "{program}");
    assert!(
        lines[4].starts_with("b4=K(\"shadcn/btn-primary\", b1, "),
        "{program}"
    );
    assert!(lines[5].starts_with("b5=I(b4, "), "{program}");
}

#[test]
fn generation_preserves_real_kit_parent_aliases_including_inline_descendants() {
    let source = r#"const sec=I(null,{id:"authored-section",type:"frame",name:"Section",children:[
  {id:"authored-slot",type:"frame",name:"Slot",children:[]}
]});
const button=K("shadcn/btn-primary","authored-section");
K("shadcn/btn-primary","authored-slot");
I(button,{type:"text",name:"Extra label",content:"Continue"});"#;
    // Both recorders must reach the same actual executor hierarchy. K's
    // authored-id support predates generation validation and is intentional.
    for run in [run_script_to_program, run_generation_script_to_program] {
        let program = run(source).unwrap();
        let mut state = EditorState::new();
        let (envelope, command) =
            crate::batch_program_test_support::call_operations(&state, &program);
        assert!(envelope.get("errors").is_none(), "{envelope}");
        assert!(state.apply(command.expect("valid kit program")));
        assert_eq!(state.active_children().len(), 1);
        let section = &state.active_children()[0];
        assert_eq!(section.base().name.as_deref(), Some("Section"));
        let children = section.children().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].base().name.as_deref(), Some("Slot"));
        let nested = children[0].children().unwrap();
        assert_eq!(nested.len(), 1);
        assert_eq!(nested[0].base().name.as_deref(), Some("Primary Button"));
        let button = &children[1];
        assert_eq!(button.base().name.as_deref(), Some("Primary Button"));
        assert!(button
            .children()
            .unwrap()
            .iter()
            .any(|child| child.base().name.as_deref() == Some("Extra label")));
    }
}

#[test]
fn generation_kit_aliases_exclude_names_unknown_ids_and_template_source_ids() {
    for parent in [
        "Section",
        "missing-id",
        "__op_tmp_hidden",
        "shadcn-btn-primary",
        "shadcn-btn-primary-label",
        "authored-kit",
    ] {
        let source = format!(
            r#"const sec=I(null,{{id:"authored-section",type:"frame",name:"Section"}});
I(sec,{{id:"__op_tmp_hidden",type:"frame"}});
K("shadcn/btn-primary",sec,{{id:"authored-kit"}});
K("shadcn/btn-primary",{parent:?});"#
        );
        for suffix in ["", " I(sec,{type:"] {
            let error = run_generation_script_to_program(&format!("{source}{suffix}")).unwrap_err();
            assert!(error.is_invalid_insertion_parent(), "{error}");
            assert!(error.to_string().contains(parent), "{error}");
        }
        // Template/source ids and K overrides never enter ProgramCtx.alias;
        // confirm these were not valid parents in the existing executor.
        let old_program = run_script_to_program(&source).unwrap();
        let (envelope, command) =
            crate::batch_program_test_support::call_operations(&EditorState::new(), &old_program);
        assert!(envelope.get("errors").is_some(), "{parent}: {envelope}");
        assert!(
            command.is_none(),
            "{parent}: invalid parent must reject the transaction"
        );
    }
}

#[test]
fn ordinary_and_modification_recorders_preserve_external_parent_ids() {
    for run in [run_script_to_program, run_modification_script_to_program] {
        let program = run(r#"I("existing",{type:"text",content:"Child"});"#).unwrap();
        assert!(program.starts_with("b0=I(existing, "), "{program}");
    }
}

#[test]
fn ordinary_mcp_script_inserts_under_an_existing_document_parent() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(crate::test_fixtures::frame(
            "existing",
            "Existing",
            0.0,
            0.0,
            320.0,
            200.0,
            vec![],
        ));
    let tool = crate::batch_design_snapshot(&state);
    let args = BTreeMap::from([(
        "script".to_owned(),
        r#"I("existing",{type:"text",name:"External child",content:"Child"});"#.to_owned(),
    )]);
    let ToolOutcome::OkJsonWithCommand(json, command) = tool.call(&args) else {
        panic!("ordinary MCP must accept a real external parent");
    };
    assert!(!json.contains("does not exist"), "{json}");
    assert!(state.apply(command));
    assert_eq!(state.active_children().len(), 1);
    let children = state.active_children()[0].children().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].base().name.as_deref(), Some("External child"));
}
