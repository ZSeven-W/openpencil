use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use serde_json::json;

fn text_child(id: String) -> serde_json::Value {
    json!({
        "type": "text",
        "id": id,
        "name": "Item",
        "content": "Item"
    })
}

fn children(count: usize, prefix: &str) -> Vec<serde_json::Value> {
    (0..count)
        .map(|idx| text_child(format!("{prefix}-{idx}")))
        .collect()
}

fn root_frame(
    id: &str,
    name: Option<&str>,
    x: Option<f64>,
    y: Option<f64>,
    width: f64,
    height: serde_json::Value,
    child_count: usize,
) -> serde_json::Value {
    let mut root = json!({
        "type": "frame",
        "id": id,
        "width": width,
        "height": height,
        "children": children(child_count, id)
    });
    let obj = root.as_object_mut().expect("root object");
    if let Some(name) = name {
        obj.insert("name".to_string(), json!(name));
    }
    if let Some(x) = x {
        obj.insert("x".to_string(), json!(x));
    }
    if let Some(y) = y {
        obj.insert("y".to_string(), json!(y));
    }
    root
}

fn sink_with_roots(roots: Vec<serde_json::Value>) -> VecDocSink {
    let nodes: Vec<PenNode> = serde_json::from_value(json!(roots)).expect("valid roots");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes,
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
    }
}

fn run_cleanup_for_active_roots(sink: &mut VecDocSink) {
    let root_ids: Vec<String> = sink
        .state
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();
    let root_id_refs: Vec<&str> = root_ids.iter().map(String::as_str).collect();
    run_cleanup_passes(sink, &plan(), &root_id_refs);
}

#[test]
fn removes_sparse_overlapping_same_named_duplicate_root() {
    let mut sink = sink_with_roots(vec![
        root_frame("stub", Some("Explore"), None, None, 390.0, json!(844), 5),
        root_frame(
            "real",
            Some("Explore"),
            None,
            None,
            390.0,
            json!("fit_content"),
            168,
        ),
    ]);

    run_cleanup_for_active_roots(&mut sink);

    let roots = sink.state.active_children();
    assert_eq!(roots.len(), 1, "sparse duplicate top-level root removed");
    assert_eq!(roots[0].base().name.as_deref(), Some("Explore"));
    assert_eq!(count_descendants(&roots[0]), 168);
}

#[test]
fn keeps_same_named_side_by_side_authored_roots() {
    let mut sink = sink_with_roots(vec![
        root_frame(
            "stub",
            Some("Explore"),
            Some(0.0),
            Some(0.0),
            390.0,
            json!(844),
            5,
        ),
        root_frame(
            "real",
            Some("Explore"),
            Some(480.0),
            Some(0.0),
            390.0,
            json!(844),
            168,
        ),
    ]);

    run_cleanup_for_active_roots(&mut sink);

    assert_eq!(
        sink.state.active_children().len(),
        2,
        "authored, non-overlapping artboards must remain separate"
    );
}

#[test]
fn keeps_single_root() {
    let mut sink = sink_with_roots(vec![root_frame(
        "real",
        Some("Explore"),
        None,
        None,
        390.0,
        json!("fit_content"),
        168,
    )]);

    run_cleanup_for_active_roots(&mut sink);

    let roots = sink.state.active_children();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].base().name.as_deref(), Some("Explore"));
    assert_eq!(count_descendants(&roots[0]), 168);
}

#[test]
fn keeps_same_named_overlapping_roots_with_similar_descendant_counts() {
    let mut sink = sink_with_roots(vec![
        root_frame("first", Some("Explore"), None, None, 390.0, json!(844), 96),
        root_frame(
            "second",
            Some("Explore"),
            None,
            None,
            390.0,
            json!(844),
            104,
        ),
    ]);

    run_cleanup_for_active_roots(&mut sink);

    assert_eq!(
        sink.state.active_children().len(),
        2,
        "two real artboards with similar richness must not be deduped"
    );
}
