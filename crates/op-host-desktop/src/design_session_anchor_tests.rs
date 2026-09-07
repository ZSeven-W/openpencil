use op_editor_core::{ChatActivity, ChatActivityStatus, ChatMessage, Locale};
use op_orchestrator::plan::{Region, Subtask};
use op_orchestrator::{RunSummary, SubtaskOutcome};

fn failed_subtask() -> Subtask {
    Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
        bleed_hero: false,
    }
}

#[test]
fn failed_middle_subtask_records_the_last_root_of_the_prior_outcome() {
    let mut messages = vec![ChatMessage::assistant_streaming()];
    messages[0].activities.push(ChatActivity {
        id: "hero".into(),
        title: "Hero".into(),
        detail: None,
        status: ChatActivityStatus::Running,
        content_offset: None,
    });
    messages[0].design_request_json_for_retry = Some("{}".into());
    let summary = RunSummary {
        root_frame_id: "root".into(),
        subtasks: vec![
            SubtaskOutcome {
                id: "nav".into(),
                node_count: 3,
                error: None,
                inserted_root_ids: vec!["nav-first".into(), "nav-last".into()],
                headline: None,
                subtask: None,
            },
            SubtaskOutcome {
                id: "hero".into(),
                node_count: 0,
                error: Some("empty content from provider".into()),
                inserted_root_ids: Vec::new(),
                headline: None,
                subtask: Some(failed_subtask()),
            },
        ],
        total_nodes: 3,
        unfilled_screens: Vec::new(),
    };

    assert!(super::workers::finish_design_success(
        &mut messages,
        &summary,
        Locale::EnUs
    ));
    assert_eq!(messages[0].failed_subtasks.len(), 1);
    assert_eq!(
        messages[0].failed_subtasks[0]
            .insert_after_sibling_id
            .as_deref(),
        Some("nav-last")
    );
}
