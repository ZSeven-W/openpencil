use super::register_new_node_reveals;
use std::collections::HashSet;

// ── F2: inserted_root_ids ──────────────────────────────────────────────────────

use super::run_subtask;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, DocSink};

fn f2_request() -> DesignRequest {
    DesignRequest {
        prompt: "a page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
    }
}

fn f2_plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn f2_subtask() -> Subtask {
    Subtask {
        id: "sec0".into(),
        label: "Hero".into(),
        region: Region {
            width: 390.0,
            height: 200.0,
        },
        id_prefix: "sec0".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    }
}

#[test]
fn reveal_schedule_streams_large_subtrees_at_a_uniform_beat() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let ids_before = HashSet::from(["root".to_string()]);
    let mut state = op_editor_core::EditorState::new();
    let children: Vec<serde_json::Value> = (0..20)
        .map(|i| {
            serde_json::json!({
                "type": "text",
                "id": format!("label-{i}"),
                "content": format!("Item {i}"),
                "fontSize": 14
            })
        })
        .collect();
    state.doc.children = vec![serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "frame",
            "id": "section",
            "name": "Section",
            "width": "fill_container",
            "height": "fit_content",
            "children": children
        }]
    }))
    .expect("fixture parses")];

    register_new_node_reveals(&ids_before, &state, Some(epoch), 1_000);

    let snapshot = op_editor_core::agent_indicators::snapshot_at(1_000);
    let section = *snapshot.reveals.get("section").expect("section reveal");
    let last_label = *snapshot.reveals.get("label-19").expect("last label reveal");
    assert!(
        last_label - section >= 800,
        "large nested subtrees should stay visibly streamed instead of arriving in one burst"
    );
    assert_eq!(
        last_label - section,
        20 * op_editor_core::agent_indicators::REVEAL_STAGGER_MS,
        "the queue keeps one uniform, readable beat per element — no tail compression"
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn reveal_schedule_keeps_nested_stream_order_across_sibling_groups() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let ids_before = HashSet::from(["root".to_string()]);
    let mut state = op_editor_core::EditorState::new();
    state.doc.children = vec![serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "frame",
            "id": "row-0",
            "name": "Row 0",
            "children": [{
                "type": "text",
                "id": "label-0",
                "content": "A"
            }]
        }, {
            "type": "frame",
            "id": "row-1",
            "name": "Row 1",
            "children": [{
                "type": "text",
                "id": "label-1",
                "content": "B"
            }]
        }]
    }))
    .expect("fixture parses")];

    register_new_node_reveals(&ids_before, &state, Some(epoch), 1_000);

    let snapshot = op_editor_core::agent_indicators::snapshot_at(1_000);
    let row_0 = *snapshot.reveals.get("row-0").expect("row 0 reveal");
    let label_0 = *snapshot.reveals.get("label-0").expect("label 0 reveal");
    let row_1 = *snapshot.reveals.get("row-1").expect("row 1 reveal");
    let label_1 = *snapshot.reveals.get("label-1").expect("label 1 reveal");

    assert!(
        row_0 < label_0 && label_0 < row_1 && row_1 < label_1,
        "nested reveals should follow visual stream order instead of sharing sibling-local slots"
    );
    assert!(
        [label_0 - row_0, row_1 - label_0, label_1 - row_1]
            .into_iter()
            .all(|gap| gap == op_editor_core::agent_indicators::REVEAL_STAGGER_MS),
        "nested stream items keep the uniform queue beat (which already covers the container runway)"
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn reveal_schedule_gives_new_container_children_an_entrance_runway() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let ids_before = HashSet::from(["root".to_string()]);
    let mut state = op_editor_core::EditorState::new();
    state.doc.children = vec![serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "frame",
            "id": "status-bar",
            "name": "Status Bar",
            "children": [{
                "type": "text",
                "id": "time",
                "content": "9:41"
            }, {
                "type": "frame",
                "id": "levels",
                "name": "Levels"
            }, {
                "type": "frame",
                "id": "battery",
                "name": "Battery"
            }]
        }]
    }))
    .expect("fixture parses")];

    register_new_node_reveals(&ids_before, &state, Some(epoch), 1_000);

    let snapshot = op_editor_core::agent_indicators::snapshot_at(1_000);
    let status = *snapshot
        .reveals
        .get("status-bar")
        .expect("status bar reveal");
    let starts = ["time", "levels", "battery"].map(|id| {
        *snapshot
            .reveals
            .get(id)
            .unwrap_or_else(|| panic!("{id} reveal"))
    });

    assert!(
        starts[0] - status >= op_editor_core::agent_indicators::REVEAL_CHILD_RUNWAY_MS,
        "children of a new container need their own runway instead of starting inside the parent's first beat"
    );
    assert!(
        starts.windows(2).all(|pair| pair[1] - pair[0] >= 40),
        "container children should continue at readable cadence after the runway"
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn reveal_schedule_does_not_spend_stream_slots_on_structure_only_containers() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let ids_before = HashSet::from(["root".to_string()]);
    let mut state = op_editor_core::EditorState::new();
    state.doc.children = vec![serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "group",
            "id": "layout-shell-0",
            "children": [{
                "type": "group",
                "id": "layout-shell-1",
                "children": [{
                    "type": "group",
                    "id": "layout-shell-2",
                    "children": [{
                        "type": "group",
                        "id": "layout-shell-3",
                        "children": [{
                            "type": "group",
                            "id": "layout-shell-4",
                            "children": [{
                                "type": "text",
                                "id": "headline",
                                "content": "Hello"
                            }]
                        }]
                    }]
                }]
            }]
        }]
    }))
    .expect("fixture parses")];

    register_new_node_reveals(&ids_before, &state, Some(epoch), 1_000);

    let snapshot = op_editor_core::agent_indicators::snapshot_at(1_000);
    let headline = *snapshot.reveals.get("headline").expect("headline reveal");
    assert!(
        headline <= 1_120,
        "structure-only wrappers should not delay the first visible generated node"
    );
    assert!(
        snapshot
            .reveals
            .keys()
            .all(|id| !id.starts_with("layout-shell")),
        "layout-only wrappers should not get their own reveal slots"
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

// ── F2 test ────────────────────────────────────────────────────────────────────

/// `run_subtask` via the immediate-apply (`VecDocSink`) path must populate
/// `SubtaskOutcome.inserted_root_ids` with the post-remap root id.
/// The id must differ from the placeholder in the LLM output ("sec") and
/// must resolve under the live tree.
#[tokio::test]
async fn run_subtask_records_inserted_root_ids() {
    // Scripted LLM returns one full-width section frame with a text child
    // (empty frames are rejected as blank containers, so we need a child).
    // Script-gen is the default protocol, so the fixture is a JS program
    // calling the bound `I(parent, obj)` recorder rather than raw JSONL.
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        r#"I(null, {"type":"frame","id":"sec","name":"Hero","x":0,"y":0,"width":390,"height":200,"children":[{"type":"text","id":"sec-t","content":"Hello"}]});"#
            .into(),
    )]);
    let mut sink = VecDocSink::new();
    let plan = f2_plan();
    let subtask = f2_subtask();
    let outcome = run_subtask(
        &subtask,
        &plan,
        &f2_request(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    )
    .await;

    assert!(
        outcome.node_count > 0,
        "subtask must produce nodes: {:?}",
        outcome.error
    );
    assert_eq!(
        outcome.inserted_root_ids.len(),
        1,
        "exactly one root inserted"
    );
    let id = &outcome.inserted_root_ids[0];
    assert_ne!(id, "sec", "id must be remapped, not the placeholder");
    // The remapped id must resolve in the live document tree.
    assert!(
        op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &op_editor_core::NodeId::new(id.clone()),
        )
        .is_some(),
        "remapped root id {id:?} must resolve in the live document"
    );
}
