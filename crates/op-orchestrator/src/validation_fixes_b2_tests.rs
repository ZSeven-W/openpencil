//! B2 tests for `apply_validation_fixes` + `auto_fix_parent_layout_after_add_child`.
//!
//! Wired via `#[cfg(test)] #[path = "validation_fixes_b2_tests.rs"] mod tests_b2;`
//! in `validation_fixes.rs`.

use super::*;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::json;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Build a VecDocSink whose active page contains the given nodes
/// (inserted via InsertSubtree).
fn sink_with_json_nodes(nodes: Vec<serde_json::Value>) -> VecDocSink {
    let mut sink = VecDocSink::new();
    let pen_nodes: Vec<jian_ops_schema::node::PenNode> = nodes
        .iter()
        .map(|v| serde_json::from_value(v.clone()).expect("node json"))
        .collect();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: pen_nodes,
        parent_id: NodeId::NONE,
        page_id: None,
    });
    // Clear the applied log so tests only see fix commands.
    sink.applied.clear();
    sink
}

/// Build a minimal frame node JSON.
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

/// Build a minimal text node JSON.
fn text_node(id: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut base = json!({
        "type": "text",
        "id": id,
        "name": id,
        "x": 0, "y": 0,
        "width": 100,
        "content": "Hello"
    });
    if let (Some(obj), Some(extra_obj)) = (base.as_object_mut(), extra.as_object()) {
        for (k, v) in extra_obj {
            obj.insert(k.clone(), v.clone());
        }
    }
    base
}

/// Assert that the applied commands contain at least one matching command.
fn has_cmd<F: Fn(&EditorCommand) -> bool>(sink: &VecDocSink, pred: F, label: &str) {
    assert!(
        sink.applied.iter().any(pred),
        "expected command matching '{label}', got: {:?}",
        sink.applied
    );
}

// ── B2 Step 1 tests ───────────────────────────────────────────────────────────

/// `width: 100` numeric fix → emits `UpdateNode` with `width = 100`.
#[test]
fn width_number_fix_emits_update_node() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "width".to_string(),
        value: json!(100),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1, "expected 1 applied fix");
    has_cmd(
        &sink,
        |c| {
            matches!(
                c,
                EditorCommand::UpdateNode {
                    width: Some(100),
                    ..
                }
            )
        },
        "UpdateNode{width:100}",
    );
}

/// `height: 48` fix → emits `UpdateNode` with `height = 48`.
#[test]
fn height_number_fix_emits_update_node() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "height".to_string(),
        value: json!(48),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| {
            matches!(
                c,
                EditorCommand::UpdateNode {
                    height: Some(48),
                    ..
                }
            )
        },
        "UpdateNode{height:48}",
    );
}

/// `fillColor: "#FF0000"` → emits `SetNodeFillHex`.
#[test]
fn fill_color_fix_emits_set_node_fill_hex() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "fillColor".to_string(),
        value: json!("#FF0000"),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| matches!(c, EditorCommand::SetNodeFillHex { hex, .. } if hex == "#FF0000"),
        "SetNodeFillHex{#FF0000}",
    );
}

/// `strokeColor: "#0000FF"` → emits `SetNodeStrokeHex`.
#[test]
fn stroke_color_fix_emits_set_node_stroke_hex() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "strokeColor".to_string(),
        value: json!("#0000FF"),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| matches!(c, EditorCommand::SetNodeStrokeHex { hex, .. } if hex == "#0000FF"),
        "SetNodeStrokeHex{#0000FF}",
    );
}

/// `strokeWidth: 2` → emits `SetNodeStrokeWidth`.
#[test]
fn stroke_width_fix_emits_set_node_stroke_width() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "strokeWidth".to_string(),
        value: json!(2),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| matches!(c, EditorCommand::SetNodeStrokeWidth { width, .. } if (*width - 2.0).abs() < 0.01),
        "SetNodeStrokeWidth{2}",
    );
}

/// fit_content → pixel guard: refuse to convert when layout container
/// has > 1 children.
#[test]
fn fit_content_to_pixel_guard_refuses_on_multi_child_container() {
    // Frame with horizontal layout and 2 children, height = "fit_content".
    // InsertSubtree remaps ids: parent→n1, child0→n2, child1→n3.
    let parent = json!({
        "type": "frame",
        "id": "n1",
        "name": "parent",
        "x": 0, "y": 0,
        "width": 200,
        "height": "fit_content",
        "layout": "horizontal",
        "children": [
            {"type": "frame", "id": "n2", "name": "c1", "x": 0, "y": 0,
             "width": 80, "height": 48, "children": []},
            {"type": "frame", "id": "n3", "name": "c2", "x": 80, "y": 0,
             "width": 80, "height": 48, "children": []}
        ]
    });
    let mut sink = sink_with_json_nodes(vec![parent]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "height".to_string(),
        value: json!(60), // trying to convert fit_content → 60px
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    // The fix must be skipped (0 applied, 1 error)
    assert_eq!(
        result.applied, 0,
        "should have skipped the fit_content→px conversion"
    );
    assert_eq!(result.errors.len(), 1, "should have 1 error entry");
    assert!(
        result.errors[0].contains("fit_content"),
        "error should mention fit_content, got: {}",
        result.errors[0]
    );
    // No UpdateNode command should have been emitted.
    assert!(
        sink.applied
            .iter()
            .all(|c| !matches!(c, EditorCommand::UpdateNode { .. })),
        "no UpdateNode should be emitted"
    );
}

/// fit_content guard does NOT block when container has ≤ 1 child.
#[test]
fn fit_content_to_pixel_allowed_on_single_child_container() {
    // InsertSubtree remaps ids: parent→n1, child→n2.
    let parent = json!({
        "type": "frame",
        "id": "n1",
        "name": "parent",
        "x": 0, "y": 0,
        "width": 200,
        "height": "fit_content",
        "layout": "vertical",
        "children": [
            {"type": "frame", "id": "n2", "name": "c1", "x": 0, "y": 0,
             "width": 80, "height": 48, "children": []}
        ]
    });
    let mut sink = sink_with_json_nodes(vec![parent]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "height".to_string(),
        value: json!(60),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    // Only 1 child → guard does NOT fire → fix is applied.
    assert_eq!(result.applied, 1, "should have applied the fix");
}

/// `removeNode` on a status-bar node is a no-op (protected).
#[test]
fn remove_node_status_bar_is_protected() {
    // Frame with role = "status-bar". InsertSubtree remaps id → n1.
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
    let sf = vec![StructuralFix::RemoveNode {
        node_id: "n1".to_string(),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(result.applied, 0, "status-bar must not be removed");
    assert_eq!(
        result.errors.len(),
        1,
        "should have 1 error for protected node"
    );
    assert!(
        result.errors[0].contains("status-bar"),
        "error should mention status-bar"
    );
    // No DeleteNode command emitted.
    assert!(
        sink.applied
            .iter()
            .all(|c| !matches!(c, EditorCommand::DeleteNode { .. })),
        "no DeleteNode should be emitted for status-bar"
    );
}

/// `removeNode` on a normal (non-status-bar) node emits `DeleteNode`.
#[test]
fn remove_node_normal_emits_delete_node() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let sf = vec![StructuralFix::RemoveNode {
        node_id: "n1".to_string(),
    }];
    let result = apply_validation_fixes(&mut sink, &[], &sf);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| matches!(c, EditorCommand::DeleteNode { node_id, .. } if node_id.as_str() == "n1"),
        "DeleteNode{n1}",
    );
}

/// Fix for a node that does not exist → skipped with error.
#[test]
fn fix_for_missing_node_is_skipped() {
    let mut sink = VecDocSink::new(); // empty document
    let fixes = vec![ValidationFix {
        node_id: "missing-node".to_string(),
        property: "width".to_string(),
        value: json!(100),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 0);
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].contains("not found"));
}

/// Non-whitelisted property → skipped with error.
#[test]
fn non_whitelisted_property_is_skipped() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "unknownProp".to_string(),
        value: json!(42),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 0);
    assert_eq!(result.errors.len(), 1);
}

/// Invalid value for a property → skipped with error.
#[test]
fn invalid_value_is_skipped() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "fillColor".to_string(),
        value: json!("not-a-hex-color"),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 0);
    assert_eq!(result.errors.len(), 1);
}

/// Empty fix lists → zero applied + no errors.
#[test]
fn empty_fixes_returns_zero() {
    let mut sink = VecDocSink::new();
    let result = apply_validation_fixes(&mut sink, &[], &[]);
    assert_eq!(result.applied, 0);
    assert!(result.errors.is_empty());
}

/// Multiple fixes applied in order.
#[test]
fn multiple_fixes_applied_in_order() {
    let mut sink = sink_with_json_nodes(vec![
        frame_node("n1", json!({})),
        frame_node("n2", json!({})),
    ]);
    let fixes = vec![
        ValidationFix {
            node_id: "n1".to_string(),
            property: "fillColor".to_string(),
            value: json!("#AABBCC"),
        },
        ValidationFix {
            node_id: "n2".to_string(),
            property: "strokeColor".to_string(),
            value: json!("#112233"),
        },
    ];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 2);
    assert!(result.errors.is_empty());
}

/// `cornerRadius: 8` → emits `SetNodeCornerRadius`.
#[test]
fn corner_radius_fix_emits_set_corner_radius() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "cornerRadius".to_string(),
        value: json!(8),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| matches!(c, EditorCommand::SetNodeCornerRadius { radius, .. } if (*radius - 8.0).abs() < 0.01),
        "SetNodeCornerRadius{8}",
    );
}

/// `fontSize: 16` → emits `SetNodeFontSize`.
#[test]
fn font_size_fix_emits_set_font_size() {
    // InsertSubtree remaps id → n1.
    let mut sink = sink_with_json_nodes(vec![text_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "fontSize".to_string(),
        value: json!(16),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| matches!(c, EditorCommand::SetNodeFontSize { font_size, .. } if (*font_size - 16.0).abs() < 0.01),
        "SetNodeFontSize{16}",
    );
}

/// `fontWeight: 700` → emits `SetNodeFontWeight`.
#[test]
fn font_weight_fix_emits_set_font_weight() {
    // InsertSubtree remaps id → n1.
    let mut sink = sink_with_json_nodes(vec![text_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "fontWeight".to_string(),
        value: json!(700u64),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| {
            matches!(
                c,
                EditorCommand::SetNodeFontWeight {
                    font_weight: 700,
                    ..
                }
            )
        },
        "SetNodeFontWeight{700}",
    );
}

/// `width: "fit_content"` keyword → emits `SetNodeLayoutProp`.
#[test]
fn width_keyword_fix_emits_layout_prop() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "width".to_string(),
        value: json!("fit_content"),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| {
            matches!(c, EditorCommand::SetNodeLayoutProp { property, value, .. }
                if property == "width"
                    && matches!(value, op_editor_core::LayoutPropValue::Keyword(k) if k == "fit_content"))
        },
        "SetNodeLayoutProp{width:fit_content}",
    );
}

/// `alignItems: "center"` → emits `SetNodeLayoutProp`.
#[test]
fn align_items_fix_emits_layout_prop() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "alignItems".to_string(),
        value: json!("center"),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| {
            matches!(c, EditorCommand::SetNodeLayoutProp { property, value, .. }
                if property == "alignItems"
                    && matches!(value, op_editor_core::LayoutPropValue::Keyword(k) if k == "center"))
        },
        "SetNodeLayoutProp{alignItems:center}",
    );
}

/// `gap: 16` → emits `SetNodeLayoutProp`.
#[test]
fn gap_fix_emits_layout_prop() {
    let mut sink = sink_with_json_nodes(vec![frame_node("n1", json!({}))]);
    let fixes = vec![ValidationFix {
        node_id: "n1".to_string(),
        property: "gap".to_string(),
        value: json!(16),
    }];
    let result = apply_validation_fixes(&mut sink, &fixes, &[]);
    assert_eq!(result.applied, 1);
    has_cmd(
        &sink,
        |c| {
            matches!(c, EditorCommand::SetNodeLayoutProp { property, value, .. }
                if property == "gap"
                    && matches!(value, op_editor_core::LayoutPropValue::Number(n) if (*n - 16.0).abs() < 0.01))
        },
        "SetNodeLayoutProp{gap:16}",
    );
}

// ── auto_fix_parent_layout_after_add_child ─────────────────────────────────

/// When parent had justify "start", the function exits immediately (no fixes).
#[test]
fn auto_fix_parent_layout_start_justify_is_noop() {
    // InsertSubtree remaps: p1→n1, c1→n2.
    let p1 = json!({
        "type": "frame", "id": "n1", "name": "Nav Item",
        "x": 0, "y": 0, "width": 100, "height": 40,
        "layout": "horizontal",
        "justifyContent": "start",
        "gap": 8,
        "children": [
            {"type": "frame", "id": "n2", "name": "c1",
             "x": 0, "y": 0, "width": 40, "height": 40, "children": []}
        ]
    });
    let mut sink = sink_with_json_nodes(vec![p1]);
    let applied = auto_fix_parent_layout_after_add_child(
        &mut sink,
        "n1",
        Some("start"),
        Some("horizontal"),
        "Nav Item",
        "frame",
    );
    assert_eq!(applied, 0, "start justify should be a no-op");
    // No SetNodeLayoutProp commands emitted.
    assert!(
        sink.applied.is_empty(),
        "no commands should be emitted for start justify"
    );
}

/// When parent had no justify, the function exits immediately (no fixes).
#[test]
fn auto_fix_parent_layout_no_justify_is_noop() {
    // InsertSubtree remaps: node→n1.
    let p1 = frame_node("n1", json!({"name": "Nav Item", "layout": "horizontal"}));
    let mut sink = sink_with_json_nodes(vec![p1]);
    let applied = auto_fix_parent_layout_after_add_child(
        &mut sink,
        "n1",
        None, // no justify
        Some("horizontal"),
        "Nav Item",
        "frame",
    );
    assert_eq!(applied, 0);
}

/// When a sibling container with same name base + layout exists, the parent's
/// justifyContent is updated to match the sibling.
#[test]
fn auto_fix_parent_layout_copies_sibling_justify() {
    // InsertSubtree remaps ids (DFS order, seed=1 on empty doc):
    //   p1→n1, c1→n2, c2→n3  (p1 + its 2 children)
    //   p2→n4, c3→n5, c4→n6  (p2 + its 2 children)
    //
    // p1 has justify "space_between" before add, now has 2 children.
    // p2 ("Tab Item") has same name-base "item", same type + layout, "start" justify.
    // child counts: p1=2, p2=2 → 2 > 2 is false → proceed.
    // Expect: justifyContent on n1 → "start" + gap on n1 → 4.

    let p1 = json!({
        "type": "frame", "id": "n1", "name": "Nav Item",
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
    // Sibling p2 with same name base ("Item"), same type + layout, "start" justify.
    let p2 = json!({
        "type": "frame", "id": "n4", "name": "Tab Item",
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
    let applied = auto_fix_parent_layout_after_add_child(
        &mut sink,
        "n1",                  // remapped parent id
        Some("space_between"), // parent justify before add
        Some("horizontal"),
        "Nav Item", // parent name before
        "frame",
    );
    // "Nav Item" → last word "Item" → base "item".
    // "Tab Item" → last word "Item" → base "item". They match.
    // current_child_count(n1)=2, candidate(n4).child_count=2 → 2 > 2 is false → proceed.
    // n4 justify = "start" ≠ parent_justify_before "space_between" → update.
    // n4 gap = 4.0 → set gap on n1.
    assert_eq!(applied, 2, "should apply justifyContent + gap updates");
    // Verify commands were emitted.
    has_cmd(
        &sink,
        |c| {
            matches!(c, EditorCommand::SetNodeLayoutProp { node_id, property, value }
                if node_id.as_str() == "n1"
                    && property == "justifyContent"
                    && matches!(value, op_editor_core::LayoutPropValue::Keyword(k) if k == "start"))
        },
        "SetNodeLayoutProp{n1: justifyContent=start}",
    );
    has_cmd(
        &sink,
        |c| {
            matches!(c, EditorCommand::SetNodeLayoutProp { node_id, property, value }
                if node_id.as_str() == "n1"
                    && property == "gap"
                    && matches!(value, op_editor_core::LayoutPropValue::Number(n) if (*n - 4.0).abs() < 0.01))
        },
        "SetNodeLayoutProp{n1: gap=4}",
    );
}

/// When the parent has more children than the sibling candidate,
/// the function skips that candidate (TS guard: currentChildCount > candChildCount → return).
#[test]
fn auto_fix_parent_layout_skips_when_more_children_than_candidate() {
    // InsertSubtree remaps ids (DFS, seed=1 on empty doc):
    //   p1→n1, c1→n2, c2→n3, c3→n4  (p1 + 3 children)
    //   p2→n5, c4→n6, c5→n7          (p2 + 2 children)
    let p1 = json!({
        "type": "frame", "id": "n1", "name": "Nav Item",
        "x": 0, "y": 0, "width": 100, "height": 40,
        "layout": "horizontal",
        "justifyContent": "space_between",
        "children": [
            {"type": "frame", "id": "n2", "name": "c1",
             "x": 0, "y": 0, "width": 30, "height": 40, "children": []},
            {"type": "frame", "id": "n3", "name": "c2",
             "x": 30, "y": 0, "width": 30, "height": 40, "children": []},
            {"type": "frame", "id": "n4", "name": "c3",
             "x": 60, "y": 0, "width": 30, "height": 40, "children": []}
        ]
    });
    let p2 = json!({
        "type": "frame", "id": "n5", "name": "Tab Item",
        "x": 0, "y": 60, "width": 100, "height": 40,
        "layout": "horizontal",
        "justifyContent": "start",
        "children": [
            {"type": "frame", "id": "n6", "name": "c4",
             "x": 0, "y": 0, "width": 40, "height": 40, "children": []},
            {"type": "frame", "id": "n7", "name": "c5",
             "x": 40, "y": 0, "width": 40, "height": 40, "children": []}
        ]
    });
    let mut sink = sink_with_json_nodes(vec![p1, p2]);
    let applied = auto_fix_parent_layout_after_add_child(
        &mut sink,
        "n1",
        Some("space_between"),
        Some("horizontal"),
        "Nav Item",
        "frame",
    );
    // n1 has 3 children, n5 has 2 → 3 > 2 → guard fires → return 0
    assert_eq!(
        applied, 0,
        "should skip when parent has more children than candidate"
    );
    assert!(sink.applied.is_empty(), "no commands should be emitted");
}
