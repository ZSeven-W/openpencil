//! Table-overflow detection + scale-math tests (known resolved widths) plus
//! the end-to-end real-layout scale/wrap cases.

use super::*;

#[test]
fn overflowing_fixed_columns_scale_down() {
    let table = overflowing_table();
    let mut rects = std::collections::HashMap::new();
    // Every row resolves to the 800px container width.
    for rid in ["hdr", "r1", "r2"] {
        rects.insert(
            rid.to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 40.0,
            },
        );
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
    rects.insert(
        "hdr".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 40.0,
        },
    );
    assert!(table_overflow_scale(&table, &rects).is_none());
}

#[test]
fn unnamed_table_shape_still_scales() {
    // The gate is STRUCTURE (≥2 rows of ≥3 cells) + geometric proof, not the
    // name — a "VIP Client List" shipped a starved 6px flex column because a
    // name gate only trusted `table`-named frames.
    let w = || json!(240);
    let widths = [w(), w(), w(), w(), w()];
    let tbl = json!({
        "type": "frame", "id": "anon", "layout": "vertical", "children": [
            row("n1", &widths), row("n2", &widths)
        ]
    });
    let mut rects = std::collections::HashMap::new();
    for rid in ["n1", "n2"] {
        rects.insert(
            rid.to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 40.0,
            },
        );
    }
    assert!(table_overflow_scale(&tbl, &rects).is_some());
}

#[test]
fn single_overflowing_row_is_not_a_table() {
    // One wide row (a toolbar, a hero strip) is not table-shaped — no scaling.
    let w = || json!(240);
    let widths = [w(), w(), w(), w(), w()];
    let strip = json!({
        "type": "frame", "id": "hero", "layout": "vertical", "children": [
            row("only", &widths)
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "only".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 40.0,
        },
    );
    assert!(table_overflow_scale(&strip, &rects).is_none());
}

#[test]
fn separated_mobile_chrome_rows_are_not_a_table() {
    // gallery-wander's destination screen: the content wrapper contained a
    // three-item top bar and a three-item bottom tab bar with business sections
    // between them. Counting horizontal children alone called this a table and
    // emitted "3 columns cannot fit a 295px row".
    let fixed = |id: &str| cell(id, json!(36));
    let top_bar = json!({
        "type": "frame", "id": "top", "layout": "horizontal", "children": [
            fixed("back"),
            { "type": "text", "id": "title", "width": "fit_content", "content": "Destination Details" },
            fixed("bookmark")
        ]
    });
    let nav_item = |id: &str| {
        json!({ "type": "frame", "id": id, "layout": "vertical",
                "width": "fill_container", "children": [] })
    };
    let content = json!({
        "type": "frame", "id": "content", "layout": "vertical", "children": [
            top_bar,
            { "type": "frame", "id": "hero", "layout": "vertical", "children": [] },
            { "type": "frame", "id": "body", "layout": "vertical", "children": [] },
            { "type": "frame", "id": "nav", "layout": "horizontal", "children": [
                nav_item("trips"), nav_item("destination"), nav_item("saved")
            ]}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    for id in ["top", "nav"] {
        rects.insert(
            id.to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 295.0,
                h: 72.0,
            },
        );
    }

    assert!(!is_table_shape(&content));
    assert!(table_overflow_scale(&content, &rects).is_none());
    assert!(table_columns_exceed_width(&content, &rects).is_none());
}

#[test]
fn contiguous_unnamed_rows_with_matching_width_modes_remain_a_table() {
    let widths = [json!(180), json!("fill_container"), json!(120), json!(96)];
    let table = json!({
        "type": "frame", "id": "records", "layout": "vertical", "children": [
            row("header", &widths),
            row("record-a", &widths),
            row("record-b", &widths)
        ]
    });

    assert!(is_table_shape(&table));
    assert_eq!(table_rows(&table).len(), 3);
}

#[test]
fn padded_row_with_text_fill_column_triggers_on_inner_width() {
    // test0703.op's exact failure shape: 860px rows padded [12,16] (inner
    // 828), fixed columns 220+120+140+166+96 = 742 + 5×16 gaps = 822, one
    // fill_container contact column CARRYING TEXT. Nothing overflows — the
    // flex column just starves to 6px and its email shreds vertically. The
    // padding-aware + text-floor math must catch it.
    let cells = |rid: &str| {
        json!([
            cell(&format!("{rid}-name"), json!(220)),
            { "type": "frame", "id": format!("{rid}-contact"), "width": "fill_container",
              "children": [ { "type": "text", "id": format!("{rid}-email"), "content": "a.sterling@email.com" } ] },
            cell(&format!("{rid}-visit"), json!(120)),
            cell(&format!("{rid}-barber"), json!(140)),
            cell(&format!("{rid}-status"), json!(166)),
            cell(&format!("{rid}-actions"), json!(96)),
        ])
    };
    let mk_row = |rid: &str| {
        json!({ "type": "frame", "id": rid, "layout": "horizontal", "gap": 16,
                "padding": [12, 16], "children": cells(rid) })
    };
    let tbl = json!({
        "type": "frame", "id": "vip", "name": "VIP Client List", "layout": "vertical",
        "children": [ mk_row("r1"), mk_row("r2"), mk_row("r3") ]
    });
    let mut rects = std::collections::HashMap::new();
    for rid in ["r1", "r2", "r3"] {
        rects.insert(
            rid.to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 860.0,
                h: 40.0,
            },
        );
    }
    let scale = table_overflow_scale(&tbl, &rects).expect("starved flex column detected");
    // Fixed 742 + gaps 80 must shrink until the text column gets its 120px
    // floor inside the 828px inner width: scale ≈ (828-120)*0.97/822 ≈ 0.835.
    assert!(
        (0.7..0.9).contains(&scale),
        "expected a moderate rescale, got {scale}"
    );
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
    rects.insert(
        "hdr".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 40.0,
        },
    );
    assert!(table_overflow_scale(&table, &rects).is_none());
}

#[test]
fn collect_scale_ops_scales_every_row_and_gap() {
    let table = overflowing_table();
    let mut rects = std::collections::HashMap::new();
    for rid in ["hdr", "r1", "r2"] {
        rects.insert(
            rid.to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 40.0,
            },
        );
    }
    let sink = crate::test_support::VecDocSink::new();
    let mut ops = Vec::new();
    collect_scale_ops(
        crate::types::DocSink::state(&sink),
        &table,
        &rects,
        &mut ops,
    );
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

#[test]
fn real_layout_wraps_text_overflowing_a_constrained_block() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    // A narrow 220px sidebar row: [name-block(fill_container) holding a long
    // fit_content name, time-block(fit_content)]. The fill block's min:0 lets it
    // shrink below the name, so the name overflows into the time column. The fix
    // must wrap the name to its block. Runs the REAL jian layout.
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"sidebar","name":"Sidebar","layout":"vertical","width":220,"height":"fit_content","children":[
            {"type":"frame","id":"row","name":"Row","layout":"horizontal","gap":8,"width":"fill_container","height":"fit_content","children":[
                {"type":"frame","id":"nameblock","name":"NameBlock","layout":"vertical","width":"fill_container","height":"fit_content","children":[
                    {"type":"text","id":"name","name":"Name","content":"Alexander Wellington Montgomery","fontSize":15}
                ]},
                {"type":"frame","id":"timeblock","name":"TimeBlock","layout":"vertical","width":"fit_content","height":"fit_content","children":[
                    {"type":"text","id":"time","name":"Time","content":"9:00 AM","fontSize":13}
                ]}
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

    geometry_validate_and_fix(&mut sink, &root_id);

    let v = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    // Find by the `name` field — `InsertSubtree` remaps authored ids.
    fn find<'a>(v: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
        if v.get("name").and_then(|x| x.as_str()) == Some(name) {
            return Some(v);
        }
        for c in v
            .get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
        {
            if let Some(r) = find(c, name) {
                return Some(r);
            }
        }
        None
    }
    let name = find(&v, "Name").expect("name text survives");
    assert_eq!(
        name.get("width").and_then(|w| w.as_str()),
        Some("fill_container"),
        "overflowing name → width fill_container"
    );
    assert_eq!(
        name.get("textGrowth").and_then(|g| g.as_str()),
        Some("fixed-width"),
        "overflowing name → textGrowth fixed-width; got {:?}",
        name.get("textGrowth")
    );
    let time = find(&v, "Time").expect("time text survives");
    assert_ne!(
        time.get("textGrowth").and_then(|g| g.as_str()),
        Some("fixed-width"),
        "the fitting time text must NOT be wrapped"
    );
}
