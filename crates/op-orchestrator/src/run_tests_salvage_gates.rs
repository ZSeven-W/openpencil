//! The final recovery attempt must keep the same delivery gates as attempts 1–3.

use super::*;

fn salvage_with(
    request: DesignRequest,
    elements: &str,
    script: &str,
) -> (RunSummary, Vec<Progress>, usize) {
    let mut plan: serde_json::Value = serde_json::from_str(MOBILE_PLAN_JSON).unwrap();
    plan["subtasks"][0]["elements"] = elements.into();
    let empty = || {
        ScriptResponse::Fail(crate::types::LlmError {
            message: "empty content from provider".into(),
            aborted: false,
        })
    };
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(plan.to_string()),
        empty(),
        empty(),
        empty(),
        ScriptResponse::Text(script.into()),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let summary = futures::executor::block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut |event| events.push(event),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("incomplete recovery remains reviewable");
    assert!(summary.total_nodes > 0);
    (summary, events, llm.user_prompts().len())
}

#[test]
fn salvage_cannot_mark_missing_named_options_done() {
    let (summary, events, calls) = salvage_with(
        req(),
        "3 个规格胶囊（月白/燕麦/苔绿）",
        r#"I(null,{type:'frame',width:390,height:300,children:[
          {type:'frame',children:[{type:'text',content:'月白'}]},
          {type:'frame',children:[{type:'text',content:'燕麦'}]}
        ]});"#,
    );
    assert_eq!(calls, 5, "planning plus the same four bounded attempts");
    assert!(summary.incomplete_subtask_failure);
    assert!(summary.subtasks[0].node_count > 0);
    assert!(summary.subtasks[0]
        .error
        .as_deref()
        .unwrap()
        .contains("苔绿"));
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::SubtaskIncomplete {
            expected: 3,
            delivered: 2,
            ..
        }
    )));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskDone { .. })));
}

#[test]
fn salvage_cannot_mark_wrong_language_done() {
    let mut request = req();
    request.prompt = "设计一个短视频手机首页，显示关注、推荐、直播、发现、消息和个人中心".into();
    let (summary, events, calls) = salvage_with(
        request,
        "视频首页导航和动态内容",
        r#"I(null,{type:'frame',width:390,height:300,children:[
          {type:'text',content:'Follow'}, {type:'text',content:'For You'},
          {type:'text',content:'Live'}, {type:'text',content:'HOME'},
          {type:'text',content:'DISCOVER'}, {type:'text',content:'INBOX'},
          {type:'text',content:'PROFILE'}
        ]});"#,
    );
    assert_eq!(calls, 5);
    assert!(summary.subtasks[0].node_count > 0);
    assert!(crate::output_language::is_language_mismatch_outcome(
        &summary.subtasks[0]
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::SubtaskLanguageMismatch {
            checked: 7,
            mismatched: 7,
            ..
        }
    )));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskDone { .. })));
}
