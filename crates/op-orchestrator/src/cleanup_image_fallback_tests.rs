use super::*;

use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::EditorCommand;
use serde_json::json;

#[test]
fn map_template_adapter_resolves_dark_background_variable() {
    let mut sink = sink_with_image();
    sink.state.doc.variables = Some(
        serde_json::from_value(json!({
            "--background": {"type":"color", "value":"#101820"}
        }))
        .unwrap(),
    );
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("root"),
        page_id: None,
        patch_json: json!({"fill":[{"type":"solid","color":"$--background"}]}).to_string(),
    });
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("failed"),
        page_id: None,
        patch_json: json!({"imageSearchQuery":"map-template:delivery", "width":327, "height":260})
            .to_string(),
    });
    repair_map_placeholders(
        &mut sink,
        "root",
        &mut RepairSummary::default(),
        &mut RepairCounter::new(),
    );
    let node = walkers::find_node(sink.state.active_children(), &NodeId::new("failed")).unwrap();
    let node = serde_json::to_value(node).unwrap();
    let ground = node["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "map-ground")
        .unwrap();
    assert_eq!(ground["fill"][0]["color"], "#18232B");
}

fn sink_with_image() -> VecDocSink {
    let mut sink = VecDocSink::new();
    let root: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "image",
            "id": "failed",
            "src": "placeholder://image-search-failed",
            "imageSearchQuery": "jump squat exercise",
            "width": 56,
            "height": 56
        }]
    }))
    .expect("valid fallback fixture");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![root],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

#[test]
fn fallback_adapter_records_node_and_policy_checkpoint() {
    let mut sink = sink_with_image();
    let mut counter = RepairCounter::new();
    let mut counting = counter.wrap(&mut sink);
    let mut summary = RepairSummary::default();
    repair_image_fallback_policy(&mut counting, &mut summary, &mut counter);
    assert_eq!(summary.repairs_for(CheckCategory::Structure), 1);
    assert!(summary
        .records()
        .iter()
        .any(|record| record.pass == "image-fallback-policy"
            && record.node_id == "failed"
            && record.detail.contains("thumb")));
}
