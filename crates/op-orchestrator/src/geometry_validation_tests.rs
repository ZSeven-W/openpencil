//! Tests for the geometry-driven table-overflow fix. The detection + scale math
//! is exercised against a KNOWN resolved row width (no real layout pass needed);
//! end-to-end behaviour is verified by rendering a generated design.

use super::*;
use serde_json::json;

fn cell(id: &str, w: serde_json::Value) -> serde_json::Value {
    json!({ "type": "frame", "id": id, "name": id, "width": w, "children": [] })
}

fn row(id: &str, widths: &[serde_json::Value]) -> serde_json::Value {
    let cells: Vec<serde_json::Value> = widths
        .iter()
        .enumerate()
        .map(|(i, w)| cell(&format!("{id}-c{i}"), w.clone()))
        .collect();
    json!({ "type": "frame", "id": id, "name": "Row", "layout": "horizontal", "gap": 16, "children": cells })
}

/// Table with 5 fixed columns of 240 (sum 1200) + 4 gaps × 16 = 64 → 1264 needed
/// in a resolved row width of 800: must scale down.
fn overflowing_table() -> serde_json::Value {
    let w = || json!(240);
    let widths = [w(), w(), w(), w(), w()];
    json!({
        "type": "frame", "id": "tbl", "name": "Client Table", "layout": "vertical", "children": [
            row("hdr", &widths), row("r1", &widths), row("r2", &widths)
        ]
    })
}

#[test]
fn overflowing_fixed_columns_scale_down() {
    let table = overflowing_table();
    let mut rects = std::collections::HashMap::new();
    // Every row resolves to the 800px container width.
    for rid in ["hdr", "r1", "r2"] {
        rects.insert(rid.to_string(), Rect { w: 800.0, h: 40.0 });
    }
    let scale = table_overflow_scale(&table, &rects).expect("overflow detected");
    assert!(
        scale < 0.75,
        "1264 needed in 800 → scale well below 1 (got {scale})"
    );
    assert!(scale >= MIN_SCALE);
}

#[test]
fn fitting_table_is_not_scaled() {
    // 5 × 120 = 600 + 64 gaps = 664 in an 800 row → fits, no scale.
    let w = || json!(120);
    let widths = [w(), w(), w(), w(), w()];
    let table = json!({
        "type": "frame", "id": "tbl", "name": "Data Table", "layout": "vertical", "children": [
            row("hdr", &widths), row("r1", &widths)
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert("hdr".to_string(), Rect { w: 800.0, h: 40.0 });
    assert!(table_overflow_scale(&table, &rects).is_none());
}

#[test]
fn non_table_named_container_is_ignored() {
    // A "Navigation" column with wide rows is NOT a table — never scaled.
    let w = || json!(240);
    let widths = [w(), w(), w(), w(), w()];
    let nav = json!({
        "type": "frame", "id": "nav", "name": "Navigation", "layout": "vertical", "children": [
            row("n1", &widths), row("n2", &widths)
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert("n1".to_string(), Rect { w: 800.0, h: 40.0 });
    assert!(table_overflow_scale(&nav, &rects).is_none());
}

#[test]
fn all_flex_table_is_not_scaled() {
    // Columns already fill_container → nothing fixed to overflow.
    let f = || json!("fill_container");
    let widths = [f(), f(), f()];
    let table = json!({
        "type": "frame", "id": "tbl", "name": "Client Table", "layout": "vertical", "children": [
            row("hdr", &widths), row("r1", &widths)
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert("hdr".to_string(), Rect { w: 400.0, h: 40.0 });
    assert!(table_overflow_scale(&table, &rects).is_none());
}

#[test]
fn collect_scale_ops_scales_every_row_and_gap() {
    let table = overflowing_table();
    let mut rects = std::collections::HashMap::new();
    for rid in ["hdr", "r1", "r2"] {
        rects.insert(rid.to_string(), Rect { w: 800.0, h: 40.0 });
    }
    let mut ops = Vec::new();
    collect_scale_ops(&table, &rects, &mut ops);
    // 3 rows × 5 fixed cells = 15 UpdateNode(width) ops + 3 SetNodeLayoutProp(gap).
    let width_ops = ops
        .iter()
        .filter(|c| matches!(c, EditorCommand::UpdateNode { width: Some(_), .. }))
        .count();
    let gap_ops = ops
        .iter()
        .filter(
            |c| matches!(c, EditorCommand::SetNodeLayoutProp { property, .. } if property == "gap"),
        )
        .count();
    assert_eq!(width_ops, 15, "every fixed cell of every row rescaled");
    assert_eq!(gap_ops, 3, "every row gap rescaled");
    let scaled_ok = ops.iter().any(
        |c| matches!(c, EditorCommand::UpdateNode { width: Some(w), .. } if *w < 240 && *w > 80),
    );
    assert!(scaled_ok, "a cell scaled to a sane width");
}

#[test]
fn real_layout_scales_overflowing_table_end_to_end() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    // A 800-wide root holding a fill_container table whose 5 fixed 240px columns
    // (1200 + gaps) overflow the resolved 800px row. Exercises the REAL jian
    // layout (`editor_state_to_layout_scene`) + real `SetNodeLayoutProp` apply.
    let mkcells = |p: &str| -> Vec<serde_json::Value> {
        (0..5)
            .map(|i| {
                json!({"type":"frame","id":format!("{p}-c{i}"),"name":"Cell","width":240,"height":20,"children":[]})
            })
            .collect()
    };
    let mkrow = |id: &str| {
        json!({"type":"frame","id":id,"name":"Row","layout":"horizontal","gap":16,
               "width":"fill_container","height":24,"children":mkcells(id)})
    };
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Root","width":800,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"tbl","name":"Client Table","layout":"vertical","width":"fill_container","children":[
                mkrow("hdr"), mkrow("r1"), mkrow("r2")
            ]}
        ]
    }))
    .expect("valid root");

    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state().active_children()[0].id_str().to_string();

    assert!(
        fix_table_column_overflow(&mut sink, &root_id),
        "the overflowing table must be rescaled via the real layout"
    );

    // Every table cell width must now be BELOW the authored 240 (scaled to fit).
    let v = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    let mut max_cell_w = 0.0_f64;
    fn walk(v: &serde_json::Value, max: &mut f64) {
        if v.get("layout").and_then(|l| l.as_str()) == Some("horizontal") {
            if let Some(kids) = v.get("children").and_then(|c| c.as_array()) {
                for cell in kids {
                    if let Some(w) = cell.get("width").and_then(|x| x.as_f64()) {
                        if w > *max {
                            *max = w;
                        }
                    }
                }
            }
        }
        if let Some(kids) = v.get("children").and_then(|c| c.as_array()) {
            for c in kids {
                walk(c, max);
            }
        }
    }
    walk(&v, &mut max_cell_w);
    assert!(
        max_cell_w > 60.0 && max_cell_w < 240.0,
        "columns scaled to fit (max cell width = {max_cell_w}, authored 240)"
    );
}

// ── collapse detector ──

fn kw_op(cmds: &[EditorCommand], prop: &str, val: &str) -> bool {
    cmds.iter().any(|c| {
        matches!(
            c,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == prop && k == val
        )
    })
}

#[test]
fn collapsed_fill_container_is_demoted() {
    // A card declaring fill_container height that RESOLVED to ~0 while its 44px
    // value text still has real height → collapse → hug + top-pack.
    let card = json!({
        "type":"frame","id":"card","name":"Card","layout":"vertical","height":"fill_container",
        "justifyContent":"space_between","children":[
            {"type":"text","id":"v","content":"1,248"},{"type":"text","id":"l","content":"TOTAL"}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert("card".to_string(), Rect { w: 200.0, h: 0.0 }); // collapsed
    rects.insert("v".to_string(), Rect { w: 200.0, h: 44.0 }); // child HAS height
    rects.insert("l".to_string(), Rect { w: 200.0, h: 14.0 });
    let mut cmds = Vec::new();
    collect_collapse_fixes(&card, &rects, &mut cmds);
    assert!(
        kw_op(&cmds, "height", "fit_content"),
        "height demoted to hug"
    );
    assert!(
        kw_op(&cmds, "justifyContent", "start"),
        "distribution neutralized"
    );
}

#[test]
fn fit_content_zero_height_is_not_a_collapse() {
    // A fit_content container at 0 resolved height is intentionally empty — only
    // a fill_container that collapsed is broken.
    let c = json!({"type":"frame","id":"c","name":"C","layout":"vertical","height":"fit_content","children":[{"type":"text","id":"t","content":"x"}]});
    let mut rects = std::collections::HashMap::new();
    rects.insert("c".to_string(), Rect { w: 100.0, h: 0.0 });
    rects.insert("t".to_string(), Rect { w: 100.0, h: 20.0 });
    let mut cmds = Vec::new();
    collect_collapse_fixes(&c, &rects, &mut cmds);
    assert!(cmds.is_empty());
}

#[test]
fn healthy_fill_container_is_not_flagged() {
    // fill_container that resolved to a real height (it filled its ancestor) → ok.
    let c = json!({"type":"frame","id":"c","name":"C","layout":"vertical","height":"fill_container","children":[{"type":"text","id":"t","content":"x"}]});
    let mut rects = std::collections::HashMap::new();
    rects.insert("c".to_string(), Rect { w: 100.0, h: 300.0 });
    rects.insert("t".to_string(), Rect { w: 100.0, h: 20.0 });
    let mut cmds = Vec::new();
    collect_collapse_fixes(&c, &rects, &mut cmds);
    assert!(cmds.is_empty());
}

#[test]
fn loop_entry_fixes_overflowing_table() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    let mkcells = |p: &str| -> Vec<serde_json::Value> {
        (0..5)
            .map(|i| json!({"type":"frame","id":format!("{p}-c{i}"),"name":"Cell","width":240,"height":20,"children":[]}))
            .collect()
    };
    let mkrow = |id: &str| json!({"type":"frame","id":id,"name":"Row","layout":"horizontal","gap":16,"width":"fill_container","height":24,"children":mkcells(id)});
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Root","width":800,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"tbl","name":"Client Table","layout":"vertical","width":"fill_container","children":[
                mkrow("hdr"), mkrow("r1"), mkrow("r2")
            ]}
        ]
    })).expect("valid root");

    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state().active_children()[0].id_str().to_string();

    let rounds = geometry_validate_and_fix(&mut sink, &root_id);
    assert!(rounds >= 1, "the loop applied at least one fix round");
}
