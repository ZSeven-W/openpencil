//! Per-subtask replay regressions for the concurrent screen-group executor.

use super::*;

const TWO_GROUP_MULTI_SUBTASK_PLAN_JSON: &str = r##"{
  "sharedContext": {}, "rootFrame": { "id": "root", "name": "App", "width": 390, "height": 844,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "search-one", "label": "Search One", "screen": "Search",
      "region": { "width": 390, "height": 200 } },
    { "id": "search-two", "label": "Search Two", "screen": "Search",
      "region": { "width": 390, "height": 200 } },
    { "id": "library-one", "label": "Library One", "screen": "Library",
      "region": { "width": 390, "height": 200 } }
  ]
}"##;

#[test]
fn successful_subtask_replays_before_its_group_finishes_and_groups_can_interleave() {
    // Initial polling starts Search One and Library One. Search One finishes
    // first; Search Two then stays slow while Library lands between the two
    // Search commits. Whole-group buffering would instead produce l1,s1,s2.
    let llm = DelayedLlm::new(vec![
        (0, TWO_GROUP_MULTI_SUBTASK_PLAN_JSON.into()),
        (1, node_json("s1")),
        (4, node_json("l1")),
        (12, node_json("s2")),
    ]);
    let mut sink = VecDocSink::new();

    futures::executor::block_on(Orchestrator::new().run(
        req_with_concurrency(2),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("two-group concurrent run ok");

    assert_eq!(
        insert_marker_order(&sink.applied, &["s1", "l1", "s2"]),
        vec!["s1", "l1", "s2"],
        "Search One must commit before its slow sibling finishes, while Library may interleave"
    );
}

fn marker_insert_count(commands: &[EditorCommand], marker: &str) -> usize {
    commands
        .iter()
        .map(|command| match command {
            EditorCommand::InsertSubtree { nodes, .. } => {
                usize::from(nodes.iter().any(|node| contains_text(node, marker)))
            }
            EditorCommand::Batch { commands } => marker_insert_count(commands, marker),
            _ => 0,
        })
        .sum()
}

#[test]
fn failed_subtask_does_not_replay_or_duplicate_prior_group_buffer() {
    let blocked = || LlmError {
        message: "content blocked by policy".into(),
        aborted: false,
    };
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(TWO_GROUP_MULTI_SUBTASK_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("s1")),
        ScriptResponse::Text(node_json("l1")),
        ScriptResponse::Fail(blocked()),
        // The policy failure is non-retryable and must skip salvage.
    ]);
    let mut sink = VecDocSink::new();

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_with_concurrency(2),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("partial failure must not abort sibling groups");

    assert_eq!(marker_insert_count(&sink.applied, "s1"), 1);
    assert_eq!(marker_insert_count(&sink.applied, "l1"), 1);
    assert_eq!(marker_insert_count(&sink.applied, "s2"), 0);
    assert!(summary
        .subtasks
        .iter()
        .any(|outcome| outcome.id == "search-two" && outcome.node_count == 0));
}
