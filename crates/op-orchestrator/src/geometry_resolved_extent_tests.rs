use super::resolved_node_height;
use crate::cleanup::run_cleanup_passes;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::json;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        shared_context: None,
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Wrapped Copy".into(),
            width: 320.0,
            height: 40.0,
            layout: Some("vertical".into()),
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
        palette: None,
    }
}

#[test]
fn cleanup_grows_fixed_root_to_resolved_wrapped_text_bottom() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Long Landing Page",
        "width": 320,
        "height": 40,
        "layout": "vertical",
        "children": [{
            "type": "text",
            "id": "copy",
            "name": "Wrapped Copy",
            "content": "A deliberately long fixed-width sentence that wraps across several lines in the real layout engine.",
            "width": 88,
            "textGrowth": "fixed-width",
            "fontSize": 18,
            "lineHeight": 1.5
        }]
    }))
    .expect("wrapped text root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();

    let overflowing_extent =
        resolved_node_height(&sink.state, &root_id).expect("resolved content extent");
    assert!(
        overflowing_extent > 40.0,
        "wrapped text must resolve past the fixed root, got {overflowing_extent}px"
    );

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    let declared = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.id_str() == root_id)
        .and_then(PenNodeExt::height_px)
        .expect("numeric root height");
    let resolved =
        resolved_node_height(&sink.state, &root_id).expect("post-cleanup resolved content extent");
    assert!(declared > 40.0, "cleanup must grow the fixed 40px root");
    assert!(
        declared + 0.5 >= resolved,
        "declared {declared}px must contain descendant bottom at {resolved}px"
    );
}
