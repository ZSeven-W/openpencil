use super::*;

const EMPTY: &str = include_str!("test_fixtures/product_detail_1018_empty_capsules.js");
const PLAN: &str = include_str!("test_fixtures/product_detail_1018_plan.json");
const COMPLETE: &str = r#"
const section=I(null,{type:"frame",name:"规格选择区",width:375,layout:"vertical",height:"fit_content"});
I(section,{type:"text",content:"规格",fontSize:16});
const choices=I(section,{type:"frame",name:"规格胶囊组",width:"fill_container",height:40,layout:"horizontal",gap:8});
for (const label of ["350ml","450ml","礼盒装"]) {
  I(choices,{type:"frame",width:90,height:40,layout:"horizontal",children:[{type:"text",content:label,fontSize:14}]});
}
"#;

fn retained_task() -> Subtask {
    crate::plan::parse_plan(PLAN)
        .unwrap()
        .subtasks
        .into_iter()
        .find(|task| task.id == "spec-selector")
        .unwrap()
}

#[test]
fn retained_unnumbered_spec_list_retries_with_all_original_names() {
    // These are the unchanged real plan and empty-capsule response. A complete
    // synthetic response exercises repair; it is not a visual acceptance sample.
    let task = retained_task();
    assert!(!task.elements.as_ref().unwrap().contains("3 个"));
    let (result, events, prompts, roots) = ladder_for_task(task.clone(), &[EMPTY, EMPTY, COMPLETE]);
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(roots.len(), 1, "incomplete attempts must be rolled back");
    assert_eq!(prompts.len(), 3);
    for prompt in &prompts[1..] {
        assert!(prompt.contains("3 named specification options"));
        for label in ["350ml", "450ml", "礼盒装"] {
            assert!(prompt.contains(label), "{prompt}");
        }
    }
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event,
                Progress::SubtaskRetry {reason,..} if reason.contains("named specification options")
            ))
            .count(),
        2
    );
    let final_sink = sink(
        roots
            .iter()
            .map(|node| serde_json::to_value(node).unwrap())
            .collect(),
    );
    assert_eq!(
        named_options::inspect(&final_sink, &task, &result).unwrap(),
        3
    );
}

#[test]
fn exhausted_unnumbered_spec_list_is_reported_incomplete() {
    let (result, events, prompts, roots) = ladder_for_task(retained_task(), &[EMPTY, EMPTY, EMPTY]);
    assert_eq!(prompts.len(), 3);
    assert_eq!(roots.len(), 1);
    assert!(is_incomplete_outcome(&result));
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::SubtaskIncomplete {
            expected: 3,
            delivered: 0,
            ..
        }
    )));
}
