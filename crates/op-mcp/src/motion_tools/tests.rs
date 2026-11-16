use super::*;
use crate::{parse_tool_call, ToolRegistry, ToolResponse};

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.1","children":[{"id":"board","type":"frame","children":[
          {"id":"first","type":"rectangle","width":30,"height":20},
          {"id":"second","type":"rectangle","width":30,"height":20}]}]}"#,
    )
    .unwrap()
    .value;
    EditorState::from_document(doc)
}

fn invoke(state: &EditorState, tool: MotionTool, args: Value) -> ToolResponse {
    let line = json!({"jsonrpc":"2.0","id":7,"method":"tools/call",
        "params":{"name":tool.name(),"arguments":args}
    })
    .to_string();
    let mut registry = ToolRegistry::default();
    registry.register(Box::new(tool.snapshot(state)));
    registry.dispatch(parse_tool_call(&line).expect("documented structured arguments parse"))
}

fn apply(state: &mut EditorState, tool: MotionTool, args: Value) {
    let outcome = invoke(state, tool, args);
    let ToolResponse::Ok {
        command: Some(command),
        ..
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert!(state.apply(command));
}

#[test]
fn staggered_presets_keep_order_and_expose_absolute_times() {
    let mut state = state();
    apply(
        &mut state,
        MotionTool::ApplyPreset,
        json!({
            "nodeIds":["second","first"],"preset":"fadeIn","clipId":"reveal","stagger":75
        }),
    );
    let ToolResponse::Ok {
        json: Some(data), ..
    } = invoke(
        &state,
        MotionTool::Get,
        json!({"nodeId":"board","deep":true}),
    )
    else {
        panic!("missing motion")
    };
    let data: Value = serde_json::from_str(&data).unwrap();
    assert_eq!(data["nodes"][1]["nodeId"], "first");
    assert_eq!(
        data["nodes"][1]["animations"][0]["keyframeTimesMs"],
        json!([75.0, 375.0])
    );
    assert_eq!(data["nodes"][2]["animations"][0]["startMs"], 0.0);
}

#[test]
fn invalid_animation_and_invalid_preset_group_emit_no_command() {
    let state = state();
    for (tool, args) in [
        (
            MotionTool::SetAnimation,
            json!({"nodeId":"first","animation":{
                "trigger":"manual","keyframes":[{"offset":0,"opacity":0},{"offset":1,"opacity":1}]
            }}),
        ),
        (
            MotionTool::ApplyPreset,
            json!({"nodeIds":["first","missing"],"preset":"fadeIn"}),
        ),
    ] {
        assert!(matches!(
            invoke(&state, tool, args),
            ToolResponse::Err {
                code: ToolErrorCode::InvalidArgument,
                ..
            }
        ));
    }
    assert!(state.active_children()[0].children().unwrap()[0]
        .base()
        .animations
        .is_none());
}

#[test]
fn edit_remove_transition_clear_and_preference_use_editor_commands() {
    let mut state = state();
    let animation = json!({"trigger":"mount","durationMs":200,"keyframes":[{"offset":0,"opacity":0},{"offset":1,"opacity":1}]});
    apply(
        &mut state,
        MotionTool::SetAnimation,
        json!({"nodeId":"first","animation":animation}),
    );
    apply(
        &mut state,
        MotionTool::SetAnimation,
        json!({"nodeId":"first","index":0,"animation":{
            "trigger":"mount","durationMs":300,"keyframes":[{"offset":0,"opacity":0},{"offset":1,"opacity":1}]
        }}),
    );
    apply(
        &mut state,
        MotionTool::RemoveAnimation,
        json!({"nodeId":"first","index":0}),
    );
    apply(
        &mut state,
        MotionTool::SetTransition,
        json!({"nodeId":"first","transition":{"durationMs":200}}),
    );
    apply(
        &mut state,
        MotionTool::SetTransition,
        json!({"nodeId":"first","transition":null}),
    );
    apply(
        &mut state,
        MotionTool::SetPreference,
        json!({"value":"reduced"}),
    );
    assert_eq!(state.doc.motion, Some(MotionPreference::Reduced));
    assert!(matches!(
        invoke(
            &state,
            MotionTool::SetPreference,
            json!({"value":"reduced"})
        ),
        ToolResponse::Ok { command: None, .. }
    ));
}
