//! B3 tests for:
//!   - `addChild` structural fix → `EditorCommand::InsertSubtree`
//!   - `extract_name_base` lowercase normalisation (B2 nit)
//!   - `auto_fix_parent_layout_after_add_child` called after addChild
//!   - status-bar protection on the addChild parent
//!
//! Wired via `#[cfg(test)] #[path = "validation_fixes_b3_tests.rs"] mod tests_b3;`
//! in `validation_fixes.rs`.

use super::*;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::json;

// ── helpers ──────────────────────────────────────────────────────────────────

fn sink_with_json_nodes(nodes: Vec<serde_json::Value>) -> VecDocSink {
    let mut sink = VecDocSink::new();
    let pen_nodes: Vec<jian_ops_schema::node::PenNode> = nodes
        .iter()
        .map(|v| serde_json::from_value(v.clone()).expect("node json"))
        .collect();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: pen_nodes,
        parent_id: NodeId::NONE,
    });
    sink.applied.clear();
    sink
}

fn frame_node(id: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut base = json!({
        "type": "frame",
        "id": id,
        "name": id,
        "x": 0, "y": 0,
        "width": 100, "height": 100,
        "children": []
    });
    if let (Some(obj), Some(extra_obj)) = (base.as_object_mut(), extra.as_object()) {
        for (k, v) in extra_obj {
            obj.insert(k.clone(), v.clone());
        }
    }
    base
}

// ── B2 nit: extract_name_base lowercases before matching ─────────────────────

/// "Nav ITEM" and "Tab Item" should produce the same name base ("item")
/// after lowercasing, matching the TS `toLowerCase()` behaviour.
#[test]
fn extract_name_base_lowercases_last_word() {
    // Build two frames with mixed-case names that should share name-base "item".
    // "Nav ITEM" → last word "ITEM" → lowercased "item"
    // "Tab item" → last word "item" → lowercased "item"
    // They should match in auto_fix_parent_layout_after_add_child.

    let p1 = json!({
        "type": "frame", "id": "n1", "name": "Nav ITEM",
        "x": 0, "y": 0, "width": 100, "height": 40,
        "layout": "horizontal",
        "justifyContent": "space_between",
        "children": [
            {"type": "frame", "id": "n2", "name": "c1",
             "x": 0, "y": 0, "width": 40, "height": 40, "children": []},
            {"type": "frame", "id": "n3", "name": "c2",
             "x": 40, "y": 0, "width": 40, "height": 40, "children": []}
        ]
    });
    let p2 = json!({
        "type": "frame", "id": "n4", "name": "Tab item",
        "x": 0, "y": 60, "width": 100, "height": 40,
        "layout": "horizontal",
        "justifyContent": "start",
        "gap": 4,
        "children": [
            {"type": "frame", "id": "n5", "name": "c3",
             "x": 0, "y": 0, "width": 40, "height": 40, "children": []},
            {"type": "frame", "id": "n6", "name": "c4",
             "x": 40, "y": 0, "width": 40, "height": 40, "children": []}
        ]
    });
    let mut sink = sink_with_json_nodes(vec![p1, p2]);
    // "Nav ITEM" should match "Tab item" because both produce name-base "item"
    // after lowercasing the last word.
    let applied = auto_fix_parent_layout_after_add_child(
        &mut sink,
        "n1",
        Some("space_between"),
        Some("horizontal"),
        "Nav ITEM",
        "frame",
    );
    // Should match (both → "item"), apply justifyContent + gap.
    assert_eq!(
        applied, 2,
        "Nav ITEM + Tab item should match via lowercased name base"
    );
}

/// Single-word mixed-case names: "ITEM" and "item" must match.
#[test]
fn extract_name_base_single_word_uppercased() {
    let p1 = json!({
        "type": "frame", "id": "n1", "name": "ITEM",
        "x": 0, "y": 0, "width": 100, "height": 40,
        "layout": "vertical",
        "justifyContent": "space_between",
        "children": [
            {"type": "frame", "id": "n2", "name": "c1",
             "x": 0, "y": 0, "width": 80, "height": 20, "children": []},
            {"type": "frame", "id": "n3", "name": "c2",
             "x": 0, "y": 20, "width": 80, "height": 20, "children": []}
        ]
    });
    let p2 = json!({
        "type": "frame", "id": "n4", "name": "item",
        "x": 0, "y": 60, "width": 100, "height": 40,
        "layout": "vertical",
        "justifyContent": "start",
        "gap": 8,
        "children": [
            {"type": "frame", "id": "n5", "name": "c3",
             "x": 0, "y": 0, "width": 80, "height": 20, "children": []},
            {"type": "frame", "id": "n6", "name": "c4",
             "x": 0, "y": 20, "width": 80, "height": 20, "children": []}
        ]
    });
    let mut sink = sink_with_json_nodes(vec![p1, p2]);
    let applied = auto_fix_parent_layout_after_add_child(
        &mut sink,
        "n1",
        Some("space_between"),
        Some("vertical"),
        "ITEM",
        "frame",
    );
    assert_eq!(
        applied, 2,
        "ITEM + item should match via lowercased name base"
    );
}

// ── addChild: basic frame spec → InsertSubtree ────────────────────────────────

/// `addChild` with `{type:"frame", name:"X", width:100, height:50}` →
/// emits `EditorCommand::InsertSubtree` under the parent.
#[test]
fn add_child_frame_spec_emits_insert_subtree() {
    let parent = frame_node("n1", json!({}));
    let mut sink = sink_with_json_nodes(vec![parent]);

    let sf = vec![StructuralFix::AddChild {
        parent_id: "n1".to_string(),
        index: None,
        spec: json!({
            "type": "frame",
            "name": "X",
            "width": 100,
            "height": 50
        }),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(result.applied, 1, "expected 1 applied fix");
    assert!(
        result.errors.is_empty(),
        "expected no errors, got: {:?}",
        result.errors
    );
    // Should emit InsertSubtree with parent_id = "n1"
    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::InsertSubtree { parent_id, nodes }
            if parent_id.as_str() == "n1" && !nodes.is_empty()
        )),
        "expected InsertSubtree under n1, got: {:?}",
        sink.applied
    );
}

/// `addChild` with `{type:"text", name:"Label", content:"Hello", fontSize:14}` →
/// emits `InsertSubtree` whose node is a text variant.
#[test]
fn add_child_text_spec_emits_insert_subtree_text() {
    let parent = frame_node("n1", json!({}));
    let mut sink = sink_with_json_nodes(vec![parent]);

    let sf = vec![StructuralFix::AddChild {
        parent_id: "n1".to_string(),
        index: None,
        spec: json!({
            "type": "text",
            "name": "Label",
            "content": "Hello",
            "fontSize": 14
        }),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(result.applied, 1, "expected text child to be inserted");
    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::InsertSubtree { parent_id, nodes }
            if parent_id.as_str() == "n1" && !nodes.is_empty()
        )),
        "expected InsertSubtree under n1"
    );
}

/// `addChild` for a missing parent → skipped with error.
#[test]
fn add_child_missing_parent_is_skipped() {
    let mut sink = VecDocSink::new(); // empty document
    let sf = vec![StructuralFix::AddChild {
        parent_id: "ghost".to_string(),
        index: None,
        spec: json!({ "type": "frame", "name": "X" }),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(result.applied, 0);
    assert_eq!(result.errors.len(), 1);
    assert!(
        result.errors[0].contains("ghost"),
        "error should mention the missing parent id"
    );
}

/// `addChild` on a status-bar parent → skipped (protected).
#[test]
fn add_child_status_bar_parent_is_protected() {
    let status_bar = json!({
        "type": "frame",
        "id": "n1",
        "name": "Status Bar",
        "role": "status-bar",
        "x": 0, "y": 0,
        "width": 390, "height": 44,
        "children": []
    });
    let mut sink = sink_with_json_nodes(vec![status_bar]);

    let sf = vec![StructuralFix::AddChild {
        parent_id: "n1".to_string(),
        index: None,
        spec: json!({ "type": "frame", "name": "Child" }),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(
        result.applied, 0,
        "status-bar parent must not accept addChild"
    );
    assert_eq!(result.errors.len(), 1);
    assert!(
        result.errors[0].contains("status-bar"),
        "error should mention status-bar"
    );
    // No InsertSubtree should have been emitted.
    assert!(
        sink.applied
            .iter()
            .all(|c| !matches!(c, EditorCommand::InsertSubtree { .. })),
        "no InsertSubtree should be emitted for status-bar parent"
    );
}

/// `addChild` with `fillColor` → the built node includes a fill.
#[test]
fn add_child_with_fill_color_builds_fill() {
    let parent = frame_node("n1", json!({}));
    let mut sink = sink_with_json_nodes(vec![parent]);

    let sf = vec![StructuralFix::AddChild {
        parent_id: "n1".to_string(),
        index: None,
        spec: json!({
            "type": "frame",
            "name": "Colored",
            "width": 40,
            "height": 40,
            "fillColor": "#FF0000"
        }),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(result.applied, 1);
    // Verify the inserted node carries a fill — check via state.
    let state = &sink.state;
    let parent_node =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n1".to_string()))
            .expect("parent n1 must exist");
    let children = match parent_node {
        jian_ops_schema::node::PenNode::Frame(f) => {
            f.children.as_ref().cloned().unwrap_or_default()
        }
        _ => vec![],
    };
    assert_eq!(children.len(), 1, "parent should now have 1 child");
    let child = &children[0];
    if let jian_ops_schema::node::PenNode::Frame(f) = child {
        let fill = f.container.fill.as_ref().expect("child should have fill");
        assert!(!fill.is_empty(), "fill should be non-empty");
        if let jian_ops_schema::style::PenFill::Solid(body) = &fill[0] {
            assert_eq!(body.color, "#FF0000", "fill color should be #FF0000");
        } else {
            panic!("expected solid fill");
        }
    } else {
        panic!("child should be a frame");
    }
}

/// `addChild` with a `path` type (icon placeholder) → emits `InsertSubtree`
/// with width/height defaulting to 18 when not specified in spec.
#[test]
fn add_child_path_type_uses_default_icon_size() {
    let parent = frame_node("n1", json!({}));
    let mut sink = sink_with_json_nodes(vec![parent]);

    let sf = vec![StructuralFix::AddChild {
        parent_id: "n1".to_string(),
        index: None,
        spec: json!({
            "type": "path",
            "name": "star"
        }),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(result.applied, 1, "path spec should produce a node");
    // Verify the inserted path node has width=18, height=18 (default icon size).
    let state = &sink.state;
    let parent_node =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n1".to_string()))
            .expect("parent n1 must exist");
    let children = match parent_node {
        jian_ops_schema::node::PenNode::Frame(f) => {
            f.children.as_ref().cloned().unwrap_or_default()
        }
        _ => vec![],
    };
    assert_eq!(children.len(), 1, "parent should now have 1 child");
    if let jian_ops_schema::node::PenNode::Path(p) = &children[0] {
        let w = p.width.as_ref().expect("path should have width");
        let h = p.height.as_ref().expect("path should have height");
        assert!(
            matches!(w, jian_ops_schema::sizing::SizingBehavior::Number(n) if (*n - 18.0).abs() < 0.01),
            "default icon width should be 18"
        );
        assert!(
            matches!(h, jian_ops_schema::sizing::SizingBehavior::Number(n) if (*n - 18.0).abs() < 0.01),
            "default icon height should be 18"
        );
    } else {
        panic!("inserted node should be a path");
    }
}

/// `addChild` with `cornerRadius` → the built frame node has cornerRadius set.
#[test]
fn add_child_frame_with_corner_radius() {
    let parent = frame_node("n1", json!({}));
    let mut sink = sink_with_json_nodes(vec![parent]);

    let sf = vec![StructuralFix::AddChild {
        parent_id: "n1".to_string(),
        index: None,
        spec: json!({
            "type": "frame",
            "name": "Rounded",
            "width": 80,
            "height": 80,
            "cornerRadius": 8
        }),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(result.applied, 1);
    let state = &sink.state;
    let parent_node =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n1".to_string()))
            .expect("parent n1");
    let children = match parent_node {
        jian_ops_schema::node::PenNode::Frame(f) => {
            f.children.as_ref().cloned().unwrap_or_default()
        }
        _ => vec![],
    };
    assert_eq!(children.len(), 1);
    if let jian_ops_schema::node::PenNode::Frame(f) = &children[0] {
        let cr = f
            .container
            .corner_radius
            .as_ref()
            .expect("should have cornerRadius");
        if let jian_ops_schema::node::CornerRadius::Uniform(v) = cr {
            assert!((*v - 8.0).abs() < 0.01, "cornerRadius should be 8");
        } else {
            panic!("expected uniform cornerRadius");
        }
    } else {
        panic!("child should be frame");
    }
}

/// `addChild` triggers `auto_fix_parent_layout_after_add_child` when the
/// parent had a non-start `justifyContent` and a sibling with same name base
/// exists.
#[test]
fn add_child_calls_auto_fix_parent_layout() {
    // p1 has justify "space_between", p2 has same name base + "start" justify.
    // After addChild to p1, auto_fix should update p1 to "start".
    let p1 = json!({
        "type": "frame", "id": "n1", "name": "Nav Item",
        "x": 0, "y": 0, "width": 100, "height": 40,
        "layout": "horizontal",
        "justifyContent": "space_between",
        "children": [
            {"type": "frame", "id": "n2", "name": "existing",
             "x": 0, "y": 0, "width": 40, "height": 40, "children": []}
        ]
    });
    let p2 = json!({
        "type": "frame", "id": "n3", "name": "Tab Item",
        "x": 0, "y": 60, "width": 100, "height": 40,
        "layout": "horizontal",
        "justifyContent": "start",
        "gap": 4,
        "children": [
            {"type": "frame", "id": "n4", "name": "c1",
             "x": 0, "y": 0, "width": 40, "height": 40, "children": []},
            {"type": "frame", "id": "n5", "name": "c2",
             "x": 40, "y": 0, "width": 40, "height": 40, "children": []}
        ]
    });
    let mut sink = sink_with_json_nodes(vec![p1, p2]);

    let sf = vec![StructuralFix::AddChild {
        parent_id: "n1".to_string(),
        index: None,
        spec: json!({
            "type": "frame",
            "name": "New Child",
            "width": 40,
            "height": 40
        }),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    // 1 addChild applied (InsertSubtree) + auto_fix applies SetNodeLayoutProp
    assert_eq!(result.applied, 1, "addChild should be applied");
    // auto_fix emits SetNodeLayoutProp commands — verify at least justifyContent.
    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeLayoutProp { node_id, property, .. }
            if node_id.as_str() == "n1" && property == "justifyContent"
        )),
        "auto_fix should emit justifyContent update on n1, got: {:?}",
        sink.applied
    );
}

/// `addChild` with `rectangle` type → emits `InsertSubtree` with a rectangle node.
#[test]
fn add_child_rectangle_type_is_built() {
    let parent = frame_node("n1", json!({}));
    let mut sink = sink_with_json_nodes(vec![parent]);

    let sf = vec![StructuralFix::AddChild {
        parent_id: "n1".to_string(),
        index: None,
        spec: json!({
            "type": "rectangle",
            "name": "Divider",
            "width": "fill_container",
            "height": 1,
            "fillColor": "#CBD5E1"
        }),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(result.applied, 1);
    let state = &sink.state;
    let parent_node =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n1".to_string()))
            .expect("parent n1");
    let children = match parent_node {
        jian_ops_schema::node::PenNode::Frame(f) => {
            f.children.as_ref().cloned().unwrap_or_default()
        }
        _ => vec![],
    };
    assert_eq!(children.len(), 1, "parent should have 1 child");
    assert!(
        matches!(children[0], jian_ops_schema::node::PenNode::Rectangle(_)),
        "child should be a rectangle"
    );
}
