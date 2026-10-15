use super::*;
use crate::{
    test_support::{ScriptResponse, ScriptedLlm, VecDocSink},
    types::AbortFlag,
};
use op_editor_core::EditorCommand;
use serde_json::json;

fn request() -> DesignRequest {
    DesignRequest {
        prompt: "手机骑行导航，全屏地图，图层和定位按钮、结束导航按钮可操作。".into(),
        ..Default::default()
    }
}

fn plan() -> crate::plan::OrchestratorPlan {
    let mut plan = crate::plan::parse_plan(&json!({
        "rootFrame":{"id":"page","name":"Navigation","width":375,"height":812},
        "subtasks":[{"id":"map","label":"全屏地图","elements":"ARCHETYPE: route-map — full screen","region":{"width":375,"height":734}}]
    }).to_string()).unwrap();
    crate::plan_normalize::normalize(&mut plan, &request());
    assert!(crate::map_viewport::owns(&plan.subtasks[0]));
    plan
}

fn script(action: &str, binding: &str, state: bool) -> String {
    format!(
        r#"const root=I(null,{{type:"frame",name:"Map viewport",width:375,height:734,layout:"none",state:{state}}});
for (const [index,name] of ["layer-button","locate-button","end-navigation-button"].entries()) {{
const btn=I(root,{{type:"frame",name,width:44,height:44,x:315,y:180+index*52,events:{{onTap:{action}}}}});
I(btn,{{type:"text",content:name,bindings:{{content:{binding}}}}});
}}"#,
        state = if state {
            r#"{active:{type:"bool",default:false}}"#
        } else {
            "{}"
        }
    )
}

fn check(script: &str) -> Option<String> {
    let (nodes, state) = crate::script_gen::parse_script(script).unwrap();
    rejection(&request(), &plan().subtasks[0], &nodes, &state)
}

#[test]
fn real_no_event_response_is_rejected_before_insertion() {
    let plan = plan();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        include_str!("test_fixtures/navigation-dock-generated.js").into(),
    )]);
    let mut sink = VecDocSink::new();
    let outcome = futures::executor::block_on(crate::subagent::run_subtask(
        &plan.subtasks[0],
        &plan,
        &request(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    let reason = outcome.error.unwrap();
    assert!(crate::retry::is_self_check_rejection(&reason));
    for name in ["图层/layers", "定位/locate", "结束导航/end navigation"] {
        assert!(reason.contains(name), "{reason}");
    }
    assert!(!sink
        .applied
        .iter()
        .any(|c| matches!(c, EditorCommand::InsertSubtree { .. })));
}

#[test]
fn nonempty_events_are_insufficient_without_declared_observable_state() {
    for (action, binding, declared) in [
        ("[]", r#""$app.active ? 'On' : 'Off'""#, true),
        (
            r#"[{toast:"已切换"}]"#,
            r#""$app.active ? 'On' : 'Off'""#,
            true,
        ),
        (r#"[{toggle:"$app.active"}]"#, r#""'Off'""#, true),
        (
            r#"[{set:{"$app.active":"true","$app.other":false}}]"#,
            r#""$app.active ? 'On' : 'Off'""#,
            true,
        ),
        (
            r#"[{set:{"$app.active":true}}]"#,
            r#""$app.active ? 'On' : 'Off'""#,
            true,
        ),
        (
            r#"[{set:{"$app.active":""}}]"#,
            r#""$app.active ? 'On' : 'Off'""#,
            true,
        ),
        (
            r#"[{toggle:"$app.active"}]"#,
            r#""$app.activeExtra ? 'On' : 'Off'""#,
            true,
        ),
        (
            r#"[{toggle:"$app.active"}]"#,
            r#""$app.active ? 'On' : 'Off'""#,
            false,
        ),
    ] {
        assert!(
            check(&script(action, binding, declared)).is_some(),
            "{action} {binding}"
        );
    }
}

#[test]
fn declared_state_driven_controls_and_conditional_actions_pass_structural_gate() {
    for action in [
        r#"[{toggle:"$app.active"}]"#,
        r#"[{set:{"$app.active":"!$app.active"}}]"#,
        r#"[{if:{expr:"$app.active",then:[{set:{"$app.active":"false"}}],else:[{toggle:"$app.active"}]}}]"#,
    ] {
        assert_eq!(
            check(&script(action, r#""$app.active ? 'On' : 'Off'""#, true)),
            None
        );
    }
}

#[test]
fn malformed_feedback_cannot_disable_an_otherwise_valid_state_action() {
    let binding = r#""$app.active ? 'On' : 'Off'""#;
    let invalid =
        r#"[{set:{"$app.active":"true"}},{toast:"已定位到演示位置（本地示意，非实时GPS）"}]"#;
    let reason = check(&script(invalid, binding, true)).unwrap();
    assert!(reason.contains("invalid action chain"), "{reason}");
    let valid =
        r#"[{set:{"$app.active":"true"}},{toast:"'已定位到演示位置（本地示意，非实时GPS）'"}]"#;
    assert_eq!(check(&script(valid, binding, true)), None);
}

#[test]
fn static_designs_and_unrelated_sections_do_not_require_actions() {
    let (nodes, state) =
        crate::script_gen::parse_script(include_str!("test_fixtures/navigation-dock-generated.js"))
            .unwrap();
    let mut req = request();
    for brief in [
        "全屏地图导航",
        "图层和定位按钮不可操作",
        "图层按钮仅视觉，无需交互",
        "static only, not interactive layers",
    ] {
        req.prompt = brief.into();
        assert!(rejection(&req, &plan().subtasks[0], &nodes, &state).is_none());
    }
    let mut task = plan().subtasks[0].clone();
    task.elements = None;
    assert!(rejection(&request(), &task, &nodes, &state).is_none());
}

#[test]
fn narrow_contract_and_examples_survive_minimal_retries() {
    let plan = plan();
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        let (call, _) = crate::prompt::build_subagent_prompt(
            &plan.subtasks[0],
            &plan,
            &request(),
            AbortFlag::new(),
            reduced,
            minimal,
            &op_editor_core::ComponentLibrary::default(),
        );
        assert!(call.user_prompt.contains("OPERABLE MAP CONTROLS"));
        assert!(call.user_prompt.contains("toast alone"));
        assert!(call.user_prompt.contains("local prototype"));
    }
}
