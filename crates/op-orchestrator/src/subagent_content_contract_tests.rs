use super::*;

#[test]
fn actual_empty_navigation_buttons_are_rejected_before_insert() {
    let script = r##"const sec=I(null,{type:"frame",name:"Header",layout:"horizontal",width:375});
I(sec,{type:"frame",name:"back-button",role:"icon-button",width:40,height:40,fill:[{type:"solid",color:"#fff"}]});
I(sec,{type:"text",content:"Delivery",fontSize:20});
I(sec,{type:"frame",name:"support-button",role:"icon-button",width:40,height:40});"##;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(script.into())]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    let error = outcome.error.unwrap();
    assert!(
        error.contains("back-button") && error.contains("support-button"),
        "{error}"
    );
    assert!(crate::retry::is_self_check_rejection(&error));
    assert_eq!(outcome.node_count, 0);
    assert!(!sink
        .applied
        .iter()
        .any(|c| matches!(c, EditorCommand::InsertSubtree { .. })));
}

#[test]
fn orphan_icon_is_adopted_before_empty_button_gate() {
    let script = r#"const sec=I(null,{type:"frame",name:"Header",layout:"horizontal",width:375});
I(sec,{type:"frame",name:"back-button",role:"icon-button",width:40,height:40,layout:"horizontal"});
I(sec,{type:"icon_font",name:"back-icon",iconFontFamily:"lucide",iconFontName:"arrow-left",width:20,height:20});
I(sec,{type:"text",content:"Delivery",fontSize:20});"#;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(script.into())]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
        panic!()
    };
    let value = serde_json::to_value(nodes).unwrap();
    assert_eq!(
        value[0]["children"][0]["children"][0]["iconFontName"],
        "arrow-left"
    );
}

#[test]
fn empty_control_gate_preserves_custom_paint_and_hidden_controls() {
    for extras in [
        r#""visible":false"#,
        r#""opacity":0"#,
        r#""fill":[{"type":"image","url":"asset:back"}]"#,
        r#""children":[{"type":"path","id":"glyph","d":"M0 0 L10 10"}]"#,
        r#""children":[{"type":"text","id":"label","content":"Back"}]"#,
    ] {
        let node: PenNode = serde_json::from_str(&format!(
            r#"{{"type":"frame","id":"button","role":"icon-button",{extras}}}"#
        ))
        .unwrap();
        assert!(empty_controls::rejection(&[node]).is_none(), "{extras}");
    }
    let node: PenNode = serde_json::from_value(serde_json::json!({
        "type":"frame","id":"button","role":"icon-button","children":[
            {"type":"icon_font","id":"hidden","visible":false,"iconFontName":"arrow-left"}
        ]
    }))
    .unwrap();
    assert!(empty_controls::rejection(&[node]).is_some());
}

fn spacer_plan() -> OrchestratorPlan {
    let mut task = subtask();
    task.id = "spacer".into();
    task.label = "底部安全区留白".into();
    task.elements = Some("纯留白安全区，无任何按钮或 Tab 栏".into());
    task.region.height = 28.0;
    let mut plan = plan();
    plan.subtasks = vec![subtask(), task];
    plan
}

#[test]
fn blank_safe_area_inserts_exact_spacing_without_model_or_retry() {
    let plan = spacer_plan();
    let llm = ScriptedLlm::new(vec![]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &plan.subtasks[1],
        &plan,
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    assert_eq!(outcome.node_count, 1);
    assert!(llm.user_prompts().is_empty());
    let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
        panic!()
    };
    assert_eq!(serde_json::to_value(nodes).unwrap()[0]["height"], 28.0);
    let abort = AbortFlag::new();
    abort.set();
    let count = sink.applied.len();
    let outcome = block_on(run_subtask(
        &plan.subtasks[1],
        &plan,
        &req(),
        &llm,
        &mut sink,
        &abort,
        false,
        false,
    ));
    assert_eq!(outcome.error.as_deref(), Some("aborted"));
    assert_eq!(count, sink.applied.len());
}

#[test]
fn spacers_do_not_swallow_real_footer_content_or_empty_whole_pages() {
    let plan = spacer_plan();
    for text in [
        "安全区，包含首页横条",
        "纯留白安全区，确认按钮",
        "footer copyright",
        "",
    ] {
        let mut task = plan.subtasks[1].clone();
        task.elements = Some(text.into());
        assert!(spacing::node(&task, &plan).is_none());
    }
    let mut task = plan.subtasks[1].clone();
    task.covers = Some(vec!["联系骑手".into()]);
    assert!(spacing::node(&task, &plan).is_none());
    let mut only = plan.clone();
    only.subtasks.remove(0);
    assert!(spacing::node(&only.subtasks[0], &only).is_none());
}
