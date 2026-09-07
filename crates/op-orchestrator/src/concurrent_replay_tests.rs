use super::*;
use crate::agent_identity::AgentIdentity;
use crate::plan::{Region, RootFrameSpec, Subtask};
use crate::test_support::VecDocSink;
use jian_ops_schema::state::{PrimitiveType, StateEntry, StateType};
use op_editor_core::NodeId;
use std::collections::BTreeMap;

fn subtask() -> Subtask {
    Subtask {
        id: "section".into(),
        label: "Section".into(),
        region: Region {
            width: 390.0,
            height: 200.0,
        },
        bleed_hero: false,
        id_prefix: "section".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: Some("Screen".into()),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "App".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![subtask()],
        style_guide_name: None,
    }
}

#[test]
fn rejected_atomic_batch_rolls_back_and_becomes_a_failed_outcome() {
    let mut app_state = BTreeMap::new();
    app_state.insert(
        "cart".into(),
        StateEntry {
            kind: StateType::Primitive(PrimitiveType::Int),
            default: Some(serde_json::json!(1)),
            description: None,
            persist: None,
        },
    );
    let node = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "generated",
        "name": "Generated",
        "width": 390,
        "height": 200
    }))
    .expect("valid frame");
    let commands = vec![
        EditorCommand::MergeAppState {
            plan_idx: 0,
            state: app_state,
        },
        EditorCommand::InsertSubtree {
            nodes: vec![node],
            parent_id: NodeId::new("missing-parent"),
            page_id: None,
        },
    ];
    let outcome = SubtaskOutcome {
        id: "section".into(),
        node_count: 1,
        error: None,
        inserted_root_ids: Vec::new(),
        headline: None,
        subtask: None,
    };
    let (ack_tx, ack_rx) = oneshot::channel();
    let mut sink = VecDocSink::new();
    let mut outcomes = vec![None];
    let mut progress = Vec::new();
    let plan = plan();

    apply_worker_event(
        WorkerSignal::SubtaskSettled(Box::new(SubtaskReplay {
            group_idx: 0,
            plan_idx: 0,
            outcome,
            commands: Some(commands),
            ack: ack_tx,
        })),
        &[ScreenGroup {
            screen: "Screen".into(),
            indices: vec![0],
        }],
        &[AgentIdentity {
            name: "Nova".into(),
            color: "#5B8DEF".into(),
        }],
        &plan,
        &mut sink,
        None,
        &mut outcomes,
        &mut |event| progress.push(event),
    );

    assert!(!futures::executor::block_on(ack_rx).expect("writer ack"));
    assert!(sink.state.active_children().is_empty());
    assert!(sink
        .state
        .doc
        .state
        .as_ref()
        .is_none_or(|state| !state.contains_key("cart")));
    let (outcome, is_zero) = outcomes[0].as_ref().expect("settled outcome");
    assert!(*is_zero);
    assert_eq!(outcome.node_count, 0);
    assert_eq!(outcome.error.as_deref(), Some("atomic replay rejected"));
    assert!(outcome.subtask.is_some(), "failure must remain retryable");
    assert!(matches!(
        progress.as_slice(),
        [Progress::WorkerScoped(worker)]
            if matches!(worker.event.as_ref(), Progress::SubtaskFailed { id, error }
            if id == "section" && error == "atomic replay rejected"
            )
    ));
}
