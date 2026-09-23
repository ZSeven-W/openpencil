//! motion50 fix 4: a tiny square frame over a single ellipse is a status dot /
//! swatch ring whose author forgot the corner radius — the cleanup pass must
//! round it, and the geometry diagnostics must be able to flag it.

use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

/// The lane0/app-08 subtree, verbatim structure: `置顶头像容器/在线绿点` is a
/// 16×16 `$--background` frame with NO cornerRadius whose only child is the
/// 10×10 `#16A34A` ellipse — it rendered as a white square stamped on the
/// avatar corner. The document's OTHER dots carry `cornerRadius: 7` and are
/// fine, which is how the miss was spotted.
fn app08_root() -> Value {
    json!({
        "type": "frame", "id": "conv-root", "name": "会话列表",
        "x": 0, "y": 0, "width": 800, "height": 600,
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        "children": [
            { "type": "frame", "id": "pinned-avatar", "name": "置顶头像容器",
              "width": 56, "height": 56,
              "children": [
                  { "type": "frame", "id": "dot", "name": "在线绿点",
                    "width": 16, "height": 16,
                    "fill": [{ "type": "solid", "color": "$--background" }],
                    "children": [
                        { "type": "ellipse", "id": "dot-ellipse", "name": "在线状态圆",
                          "width": 10, "height": 10,
                          "fill": [{ "type": "solid", "color": "#16A34A" }] }
                    ] }
              ] }
        ]
    })
}

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "conv-root".into(),
            name: "会话列表".into(),
            width: 800.0,
            height: 600.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn insert_root(value: Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    let root: PenNode = serde_json::from_value(value).expect("root json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

fn find_node<'a>(node: &'a PenNode, name: &str) -> Option<&'a PenNode> {
    if node.base().name.as_deref() == Some(name) {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, name))
}

fn named_node_value(sink: &VecDocSink, name: &str) -> Value {
    let root = sink.state.active_children().first().expect("root exists");
    let node = find_node(root, name).expect("named node exists");
    serde_json::to_value(node).expect("node json")
}

#[test]
fn cleanup_rounds_the_square_dot_wrapper() {
    let mut sink = insert_root(app08_root());
    run_cleanup_passes(&mut sink, &plan(), &["conv-root"]);
    let dot = named_node_value(&sink, "在线绿点");
    assert_eq!(
        dot.get("cornerRadius").and_then(Value::as_f64),
        Some(8.0),
        "the 16×16 dot wrapper must be rounded to width/2, got {dot}"
    );
    // The ellipse child stays untouched.
    let ellipse = {
        let node = find_node(sink.state.active_children().first().unwrap(), "在线状态圆")
            .expect("ellipse exists");
        serde_json::to_value(node).expect("ellipse json")
    };
    assert_eq!(ellipse.get("width").and_then(Value::as_f64), Some(10.0));
    assert!(ellipse.get("cornerRadius").is_none());
}

#[test]
fn cleanup_leaves_the_correctly_authored_dots_alone() {
    // The same document's good dots: 14×14 with cornerRadius 7 (exactly
    // circular over a 9×9 ellipse) — the pass must not touch them.
    let mut root = app08_root();
    {
        let dot = &mut root["children"][0]["children"][0];
        dot["width"] = json!(14);
        dot["height"] = json!(14);
        dot["cornerRadius"] = json!(7.0);
        dot["children"][0]["width"] = json!(9);
        dot["children"][0]["height"] = json!(9);
    }
    let mut sink = insert_root(root);
    run_cleanup_passes(&mut sink, &plan(), &["conv-root"]);
    let dot = named_node_value(&sink, "在线绿点");
    assert_eq!(dot.get("cornerRadius").and_then(Value::as_f64), Some(7.0));
}

#[test]
fn geometry_diagnostics_flag_the_square_ring_wrapper() {
    let sink = insert_root(app08_root());
    let issues = crate::geometry_validation::geometry_diagnostics(&sink.state);
    assert!(
        issues
            .iter()
            .any(|line| line.contains("square-ring-wrapper")),
        "diagnostics must report the square ring wrapper, got {issues:?}"
    );
}
