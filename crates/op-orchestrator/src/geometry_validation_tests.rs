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
    rects.insert(
        "card".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 0.0,
        },
    ); // collapsed
    rects.insert(
        "v".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 44.0,
        },
    ); // child HAS height
    rects.insert(
        "l".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 14.0,
        },
    );
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
    rects.insert(
        "c".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 0.0,
        },
    );
    rects.insert(
        "t".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 20.0,
        },
    );
    let mut cmds = Vec::new();
    collect_collapse_fixes(&c, &rects, &mut cmds);
    assert!(cmds.is_empty());
}

#[test]
fn healthy_fill_container_is_not_flagged() {
    // fill_container that resolved to a real height (it filled its ancestor) → ok.
    let c = json!({"type":"frame","id":"c","name":"C","layout":"vertical","height":"fill_container","children":[{"type":"text","id":"t","content":"x"}]});
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "c".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 300.0,
        },
    );
    rects.insert(
        "t".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 20.0,
        },
    );
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

/// ENGINE-CONTRACT sentinel: a `fill_container`-height child of a HUGGING parent
/// must resolve to a real size, not collapse — vertical main axis via grow,
/// horizontal cross axis via stretch (to the tallest sibling). The retirement of
/// the tree-shape `fix_circular_fill_height` demoter rests on this contract; if
/// jian regresses it, this fires long before a corpus render does.
#[test]
fn real_layout_fill_of_hug_parent_resolves_to_content_not_collapse() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;

    // Shape A: vertical hug parent + fill-height child (space_between, real content)
    // Shape B: horizontal hug row + fill-height KPI cards
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Root","width":800,"height":"fit_content","layout":"vertical","gap":24,"children":[
            {"type":"frame","id":"vparent","name":"VParent","layout":"vertical","width":"fill_container","height":"fit_content","children":[
                {"type":"frame","id":"vchild","name":"VChild","layout":"vertical","width":"fill_container","height":"fill_container","justifyContent":"space_between","children":[
                    {"type":"text","id":"t1","name":"T1","content":"Value 42","fontSize":28},
                    {"type":"text","id":"t2","name":"T2","content":"Label","fontSize":13}
                ]}
            ]},
            {"type":"frame","id":"hrow","name":"HRow","layout":"horizontal","gap":16,"width":"fill_container","height":"fit_content","children":[
                {"type":"frame","id":"card1","name":"Card1","layout":"vertical","width":"fill_container","height":"fill_container","justifyContent":"space_between","children":[
                    {"type":"text","id":"c1a","name":"C1A","content":"98.7%","fontSize":28},
                    {"type":"text","id":"c1b","name":"C1B","content":"Uptime","fontSize":13}
                ]},
                {"type":"frame","id":"card2","name":"Card2","layout":"vertical","width":"fill_container","height":120,"children":[
                    {"type":"text","id":"c2a","name":"C2A","content":"1,284","fontSize":28}
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
    let rects = resolved_rects(sink.state());
    let v = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    fn rect_of<'a>(
        v: &serde_json::Value,
        rects: &'a HashMap<String, Rect>,
        name: &str,
    ) -> Option<&'a Rect> {
        if v.get("name").and_then(|x| x.as_str()) == Some(name) {
            return v
                .get("id")
                .and_then(|x| x.as_str())
                .and_then(|id| rects.get(id));
        }
        v.get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .find_map(|c| rect_of(c, rects, name))
    }
    // Shape A: the fill child of a hugging vertical parent hugs its two text
    // lines (28px + 13px) instead of collapsing to ~0.
    let vchild = rect_of(&v, &rects, "VChild").expect("VChild resolved");
    assert!(
        vchild.h >= 40.0,
        "fill-of-hug vertical child must hug content, got h={}",
        vchild.h
    );
    // Shape B: the fill card cross-axis-stretches to its 120px numeric sibling.
    let card1 = rect_of(&v, &rects, "Card1").expect("Card1 resolved");
    assert!(
        (card1.h - 120.0).abs() < 1.0,
        "fill card must stretch to the tallest sibling, got h={}",
        card1.h
    );
    // Its stacked children must not overlap (the old percent-mapping collapse).
    let c1a = rect_of(&v, &rects, "C1A").expect("C1A resolved");
    let c1b = rect_of(&v, &rects, "C1B").expect("C1B resolved");
    assert!(
        c1a.h > 0.0 && c1b.h > 0.0,
        "card children carry real height"
    );
}

#[test]
fn real_layout_equalizes_luxe_cut_metric_card_row_heights() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    let stroke = || json!({"thickness": 1, "fill": [{"type": "solid", "color": "#E5E7EB"}]});
    let card = |id: &str, name: &str, title: &str| {
        json!({
            "type":"frame","id":id,"name":name,"layout":"vertical","gap":8,
            "width":"fill_container","height":"fit_content","padding":[16,16],
            "stroke": stroke(),
            "children":[
                {"type":"text","id":format!("{id}-title"),"name":format!("{name} Title"),
                 "content":title,"fontSize":15,"width":"fill_container","textGrowth":"fixed-width"},
                {"type":"text","id":format!("{id}-value"),"name":format!("{name} Value"),
                 "content":"$48,920","fontSize":28},
                {"type":"text","id":format!("{id}-label"),"name":format!("{name} Label"),
                 "content":"vs last month","fontSize":12}
            ]
        })
    };
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","name":"LUXE CUT Dashboard","width":960,"height":"fit_content","layout":"vertical","gap":24,"children":[
            {"type":"frame","id":"metrics","name":"Key Metrics","layout":"horizontal","gap":16,
             "width":"fill_container","height":"fit_content","children":[
                card("card1", "Metric Card 1", "Revenue"),
                card("card2", "Metric Card 2", "Average revenue per client visit this month"),
                card("card3", "Metric Card 3", "Bookings"),
                card("card4", "Metric Card 4", "Retention")
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

    let before_rects = resolved_rects(sink.state());
    let before = resolved_heights_by_name(
        sink.state(),
        &before_rects,
        &[
            "Metric Card 1",
            "Metric Card 2",
            "Metric Card 3",
            "Metric Card 4",
        ],
    );
    let before_min = before.iter().copied().fold(f64::INFINITY, f64::min);
    let before_max = before.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        before_max - before_min > 6.0,
        "fixture must be ragged before repair, got {before:?}"
    );

    let rounds = geometry_validate_and_fix(&mut sink, &root_id);
    assert!(rounds >= 1, "ragged card row must trigger a fix round");

    let after_rects = resolved_rects(sink.state());
    let after = resolved_heights_by_name(
        sink.state(),
        &after_rects,
        &[
            "Metric Card 1",
            "Metric Card 2",
            "Metric Card 3",
            "Metric Card 4",
        ],
    );
    let after_min = after.iter().copied().fold(f64::INFINITY, f64::min);
    let after_max = after.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        after_max - after_min <= 1.0,
        "cards must resolve to equal heights after repair, got {after:?}"
    );
}

#[test]
fn card_row_with_authored_numeric_child_height_is_left_untouched() {
    let row = json!({
        "type":"frame","id":"row","name":"Cards","layout":"horizontal","children":[
            stroked_card_json("c1", json!("fit_content")),
            stroked_card_json("c2", json!(180)),
            stroked_card_json("c3", json!("fit_content"))
        ]
    });
    let rects = card_row_rects([140.0, 180.0, 142.0]);
    let mut cmds = Vec::new();

    collect_card_row_height_fixes(&row, &rects, &mut cmds, false);

    assert!(
        cmds.is_empty(),
        "numeric child height is deliberate: {cmds:?}"
    );
}

#[test]
fn transparent_wrapper_row_is_not_equalized_as_cards() {
    let row = json!({
        "type":"frame","id":"row","name":"Wrappers","layout":"horizontal","children":[
            transparent_card_json("c1"),
            transparent_card_json("c2"),
            transparent_card_json("c3")
        ]
    });
    let rects = card_row_rects([140.0, 180.0, 142.0]);
    let mut cmds = Vec::new();

    collect_card_row_height_fixes(&row, &rects, &mut cmds, false);

    assert!(
        cmds.is_empty(),
        "transparent wrappers are not colored cards: {cmds:?}"
    );
}

#[test]
fn card_rows_inside_table_context_are_left_untouched() {
    let table = json!({
        "type":"frame","id":"table","name":"Table","layout":"vertical","children":[
            {"type":"frame","id":"row1","layout":"horizontal","children":[
                stroked_card_json("r1c1", json!("fit_content")),
                stroked_card_json("r1c2", json!("fit_content")),
                stroked_card_json("r1c3", json!("fit_content"))
            ]},
            {"type":"frame","id":"row2","layout":"horizontal","children":[
                stroked_card_json("r2c1", json!("fit_content")),
                stroked_card_json("r2c2", json!("fit_content")),
                stroked_card_json("r2c3", json!("fit_content"))
            ]}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    for (id, h) in [
        ("row1", 180.0),
        ("row2", 180.0),
        ("r1c1", 140.0),
        ("r1c2", 180.0),
        ("r1c3", 142.0),
        ("r2c1", 140.0),
        ("r2c2", 180.0),
        ("r2c3", 142.0),
    ] {
        rects.insert(
            id.to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h,
            },
        );
    }
    let mut cmds = Vec::new();

    collect_card_row_height_fixes(&table, &rects, &mut cmds, false);

    assert!(
        cmds.is_empty(),
        "table rows belong to table repair: {cmds:?}"
    );
}

fn resolved_heights_by_name(
    state: &op_editor_core::EditorState,
    rects: &HashMap<String, Rect>,
    names: &[&str],
) -> Vec<f64> {
    let v = serde_json::to_value(state.active_children()[0].clone()).unwrap();
    names
        .iter()
        .map(|name| {
            let id = find_id_by_name(&v, name).unwrap_or_else(|| panic!("{name} exists"));
            rects
                .get(&id)
                .unwrap_or_else(|| panic!("{name} resolved"))
                .h
        })
        .collect()
}

fn find_id_by_name(v: &serde_json::Value, name: &str) -> Option<String> {
    if v.get("name").and_then(|x| x.as_str()) == Some(name) {
        return v.get("id").and_then(|x| x.as_str()).map(String::from);
    }
    v.get("children")
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
        .find_map(|c| find_id_by_name(c, name))
}

fn stroked_card_json(id: &str, height: serde_json::Value) -> serde_json::Value {
    json!({
        "type":"frame","id":id,"name":id,"height":height,
        "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#E5E7EB"}]},
        "children":[]
    })
}

fn transparent_card_json(id: &str) -> serde_json::Value {
    json!({"type":"frame","id":id,"name":id,"height":"fit_content","children":[]})
}

fn card_row_rects(heights: [f64; 3]) -> HashMap<String, Rect> {
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "row".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 360.0,
            h: heights.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        },
    );
    for (id, h) in ["c1", "c2", "c3"].into_iter().zip(heights) {
        rects.insert(
            id.to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h,
            },
        );
    }
    rects
}

#[test]
fn text_overflow_fix_skips_absolute_positioned_parents() {
    // Under `layout: none` children are absolutely positioned — a text wider
    // than the parent is a positioning choice, not a flex overflow to repair.
    let block = json!({
        "type":"frame","id":"blk","name":"Canvas","layout":"none","width":200,"height":200,"children":[
            {"type":"text","id":"t","name":"T","content":"a very long decorative caption"}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "blk".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 200.0,
        },
    );
    rects.insert(
        "t".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 380.0,
            h: 20.0,
        },
    ); // wider than parent
    let mut cmds = Vec::new();
    collect_text_overflow_fixes(&block, &rects, &mut cmds);
    assert!(
        cmds.is_empty(),
        "no wrap ops under a layout:none parent, got {cmds:?}"
    );
}

#[test]
fn real_layout_reins_in_numeric_child_wider_than_its_row() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    // glm7's authored defect verbatim: an 800px-wide avatar bar inside a
    // ~550px appointment row — it spilled across the whole design and past
    // every tree-shape pass. The geometry loop must retarget it to
    // fill_container so it stays inside the row.
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Root","width":560,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"row","name":"Appt Row","layout":"horizontal","gap":14,"width":"fill_container","height":"fit_content","children":[
                {"type":"frame","id":"time","name":"Time","width":52,"height":36,"children":[]},
                {"type":"frame","id":"bar","name":"Avatar Bar","width":800,"height":36,"children":[]}
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

    let rounds = geometry_validate_and_fix(&mut sink, &root_id);
    assert!(rounds >= 1, "the overflow must trigger a fix round");

    let v = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    fn find<'a>(v: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
        if v.get("name").and_then(|x| x.as_str()) == Some(name) {
            return Some(v);
        }
        v.get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .find_map(|c| find(c, name))
    }
    let bar = find(&v, "Avatar Bar").expect("bar survives");
    assert_eq!(
        bar.get("width").and_then(|w| w.as_str()),
        Some("fill_container"),
        "oversized numeric child reined in, got {:?}",
        bar.get("width")
    );
    let time = find(&v, "Time").expect("time survives");
    assert_eq!(
        time.get("width").and_then(|w| w.as_f64()),
        Some(52.0),
        "the fitting fixed column is untouched"
    );
}

#[test]
fn jammed_text_columns_are_reported_but_flush_icons_are_not() {
    // Row of three cells: [date-cell][visits-cell] touch (0px apart, both carry
    // text → jam), while [icon][icon] flush contact is fine.
    let row = json!({
        "type":"frame","id":"row","name":"Row","layout":"horizontal","children":[
            {"type":"frame","id":"date","name":"Date","children":[{"type":"text","id":"dt","content":"Oct 24, 2024"}]},
            {"type":"frame","id":"visits","name":"Visits","children":[{"type":"text","id":"vt","content":"42"}]},
            {"type":"frame","id":"ic1","name":"Icon A","children":[]},
            {"type":"frame","id":"ic2","name":"Icon B","children":[]}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "row".into(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 40.0,
        },
    );
    rects.insert(
        "date".into(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 110.0,
            h: 40.0,
        },
    );
    rects.insert(
        "visits".into(),
        Rect {
            x: 110.0,
            y: 0.0,
            w: 30.0,
            h: 40.0,
        },
    ); // touches date
    rects.insert(
        "ic1".into(),
        Rect {
            x: 200.0,
            y: 0.0,
            w: 18.0,
            h: 18.0,
        },
    );
    rects.insert(
        "ic2".into(),
        Rect {
            x: 218.0,
            y: 0.0,
            w: 18.0,
            h: 18.0,
        },
    ); // flush icons
    let mut out = Vec::new();
    collect_sibling_jam_diagnostics(&row, &rects, &mut out);
    assert_eq!(out.len(), 1, "exactly the text jam is reported: {out:?}");
    assert!(out[0].contains("Date") && out[0].contains("Visits"));
}

#[test]
fn overlapping_siblings_are_reported_regardless_of_content() {
    let row = json!({
        "type":"frame","id":"row","name":"Row","layout":"horizontal","children":[
            {"type":"frame","id":"a","name":"Left","children":[]},
            {"type":"frame","id":"b","name":"Right","children":[]}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "row".into(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 300.0,
            h: 40.0,
        },
    );
    rects.insert(
        "a".into(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 40.0,
        },
    );
    rects.insert(
        "b".into(),
        Rect {
            x: 150.0,
            y: 0.0,
            w: 100.0,
            h: 40.0,
        },
    ); // 50px overlap
    let mut out = Vec::new();
    collect_sibling_jam_diagnostics(&row, &rects, &mut out);
    assert_eq!(out.len(), 1);
    assert!(out[0].contains("OVERLAP"), "got {out:?}");
}

#[test]
fn vertical_overlapping_siblings_are_reported() {
    let stack = json!({
        "type":"frame","id":"stack","name":"Stack","layout":"vertical","children":[
            {"type":"frame","id":"a","name":"Contact Block","children":[]},
            {"type":"frame","id":"b","name":"Footer","children":[]}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "a".into(),
        Rect {
            x: 0.0,
            y: 10.0,
            w: 240.0,
            h: 60.0,
        },
    );
    rects.insert(
        "b".into(),
        Rect {
            x: 0.0,
            y: 65.0,
            w: 240.0,
            h: 40.0,
        },
    ); // 5px vertical overlap
    let mut out = Vec::new();
    collect_sibling_jam_diagnostics(&stack, &rects, &mut out);
    assert_eq!(out.len(), 1);
    assert!(
        out[0].contains("Contact Block") && out[0].contains("Footer"),
        "got {out:?}"
    );
    assert!(out[0].contains("OVERLAP"), "got {out:?}");
}

#[test]
fn vertical_ring_badge_overlay_is_not_reported_as_an_overlap() {
    let stack = json!({
        "type":"frame","id":"stack","name":"Ring","layout":"vertical","children":[
            {"type":"ellipse","id":"e","width":36,"height":36},
            {"type":"text","id":"t","content":"2","fontSize":15}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "e".into(),
        Rect {
            x: 40.0,
            y: 0.0,
            w: 36.0,
            h: 36.0,
        },
    );
    rects.insert(
        "t".into(),
        Rect {
            x: 53.0,
            y: 9.0,
            w: 10.0,
            h: 18.0,
        },
    );
    let mut out = Vec::new();
    collect_sibling_jam_diagnostics(&stack, &rects, &mut out);
    assert!(out.is_empty(), "overlay must not be reported: {out:?}");
}

/// Manual repair harness (not part of the suite): load OP_REPAIR_IN, run the
/// whole-doc loop finalize (Class-A + cleanup + geometry), save OP_REPAIR_OUT.
/// `OP_REPAIR_IN=/path/in.op OP_REPAIR_OUT=/path/out.op cargo test -p
/// op-orchestrator repair_harness -- --ignored --nocapture`
#[test]
#[ignore]
fn repair_harness_finalizes_an_op_file() {
    let inp = std::env::var("OP_REPAIR_IN").expect("OP_REPAIR_IN");
    let out = std::env::var("OP_REPAIR_OUT").expect("OP_REPAIR_OUT");
    let text = std::fs::read_to_string(&inp).expect("read input");
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(&text).expect("parse .op");
    let mut state = op_editor_core::EditorState::from_document(doc);
    crate::loop_finalize::apply_loop_finalize(&mut state);
    std::fs::write(&out, serde_json::to_string_pretty(&state.doc).unwrap()).expect("write output");
    eprintln!("repaired {inp} -> {out}");
}

/// Manual harness variant: run ONLY the orchestrator's `finalize_design`
/// (no whole-doc Class-A prelude) — for bisecting orchestrator-vs-loop
/// finalize differences on a real file.
#[test]
#[ignore]
fn finalize_only_harness() {
    let inp = std::env::var("OP_REPAIR_IN").expect("OP_REPAIR_IN");
    let out = std::env::var("OP_REPAIR_OUT").expect("OP_REPAIR_OUT");
    let text = std::fs::read_to_string(&inp).expect("read input");
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(&text).expect("parse .op");
    let mut state = op_editor_core::EditorState::from_document(doc);
    use op_editor_core::PenNodeExt;
    let root_id = state.active_children()[0].id_str().to_string();
    let plan: crate::plan::OrchestratorPlan = serde_json::from_value(serde_json::json!({
        "rootFrame": {"id":"root","name":"Page","width":1200,"height":800,"layout":"vertical"},
        "subtasks": []
    }))
    .expect("stub plan");
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::cleanup::finalize_design(&mut sink, &plan, &[&root_id]);
    std::fs::write(&out, serde_json::to_string_pretty(&state.doc).unwrap()).expect("write");
    eprintln!("finalized {inp} -> {out}");
}

/// Manual probe: print resolved rects for nodes whose name matches
/// OP_PROBE_NAME inside OP_REPAIR_IN. Set OP_PROBE_UNDER=1 to print the
/// whole resolved subtree of every match (rows/cells are usually unnamed,
/// so matching the named ancestor and dumping under it is the useful mode).
#[test]
#[ignore]
fn resolved_rect_probe() {
    let inp = std::env::var("OP_REPAIR_IN").expect("OP_REPAIR_IN");
    let pat = std::env::var("OP_PROBE_NAME")
        .unwrap_or_default()
        .to_lowercase();
    let under = std::env::var("OP_PROBE_UNDER").is_ok();
    let text = std::fs::read_to_string(&inp).expect("read input");
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(&text).expect("parse .op");
    let state = op_editor_core::EditorState::from_document(doc);
    let rects = resolved_rects(&state);
    for root in state.active_children() {
        let v = serde_json::to_value(root).unwrap();
        #[allow(clippy::too_many_arguments)]
        fn walk(
            v: &serde_json::Value,
            rects: &HashMap<String, Rect>,
            pat: &str,
            under: bool,
            in_match: bool,
            depth: usize,
        ) {
            let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
            let nid = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
            let hit = !pat.is_empty()
                && (name.to_lowercase().contains(pat) || nid.eq_ignore_ascii_case(pat));
            if hit || (under && in_match) {
                let kind = v.get("type").and_then(|x| x.as_str()).unwrap_or("?");
                let label = if name.is_empty() { nid } else { name };
                if let Some(r) = rects.get(nid) {
                    eprintln!(
                        "{:indent$}{label} [{kind}]: x={:.2} y={:.2} w={:.2} h={:.2}",
                        "",
                        r.x,
                        r.y,
                        r.w,
                        r.h,
                        indent = depth
                    );
                } else {
                    eprintln!("{:indent$}{label} [{kind}]: <no rect>", "", indent = depth);
                }
            }
            for c in v
                .get("children")
                .and_then(|c| c.as_array())
                .into_iter()
                .flatten()
            {
                walk(c, rects, pat, under, in_match || hit, depth + 1);
            }
        }
        walk(&v, &rects, &pat, under, false, 0);
    }
}

/// Manual corpus replay: load p01.op..p52.op from OP_GEOMETRY_REPLAY_DIR, run
/// geometry_validate_and_fix on every active root, and assert the parsed doc is
/// unchanged. This is the dirty-diff gate for geometry-only replay.
#[test]
#[ignore]
fn replay_geometry_validate_corpus() {
    let dir = std::env::var("OP_GEOMETRY_REPLAY_DIR").expect("OP_GEOMETRY_REPLAY_DIR");
    let out_dir = std::env::var("OP_GEOMETRY_REPLAY_OUT_DIR").ok();
    if let Some(out_dir) = &out_dir {
        std::fs::create_dir_all(out_dir).expect("create replay out dir");
    }
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .expect("read replay dir")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_numbered_corpus_op)
        })
        .collect();
    files.sort();

    let mut dirty = Vec::new();
    let mut baseline_input_dirty = Vec::new();
    let mut baseline_rounds = 0usize;
    let mut current_rounds = 0usize;
    for path in &files {
        let text = std::fs::read_to_string(path).expect("read corpus op");
        let doc: jian_ops_schema::PenDocument = serde_json::from_str(&text).expect("parse op");
        let mut baseline_state = op_editor_core::EditorState::from_document(doc.clone());
        let mut current_state = op_editor_core::EditorState::from_document(doc);
        let before = serde_json::to_value(&current_state.doc).expect("before value");
        let root_ids: Vec<String> = current_state
            .active_children()
            .iter()
            .map(|root| {
                use op_editor_core::PenNodeExt;
                root.id_str().to_string()
            })
            .collect();
        let baseline_root_ids = root_ids.clone();
        {
            let mut sink = crate::loop_finalize::StateDocSink {
                state: &mut baseline_state,
            };
            for root_id in baseline_root_ids {
                baseline_rounds += geometry_validate_and_fix_without_card_rows(&mut sink, &root_id);
            }
        }
        {
            let mut sink = crate::loop_finalize::StateDocSink {
                state: &mut current_state,
            };
            for root_id in root_ids {
                current_rounds += geometry_validate_and_fix(&mut sink, &root_id);
            }
        }
        let baseline_after = serde_json::to_value(&baseline_state.doc).expect("baseline value");
        let current_after = serde_json::to_value(&current_state.doc).expect("current value");
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?")
            .to_string();
        if before != baseline_after {
            baseline_input_dirty.push(name.clone());
        }
        if baseline_after != current_after {
            if let Some(out_dir) = &out_dir {
                let out_path = std::path::Path::new(out_dir).join(format!("{name}.current.op"));
                std::fs::write(
                    out_path,
                    serde_json::to_string_pretty(&current_state.doc).expect("serialize dirty doc"),
                )
                .expect("write dirty doc");
            }
            dirty.push(name);
        }
    }

    eprintln!(
        "[GEOMETRY-REPLAY] checked={} baseline_input_dirty={} dirty={} baseline_rounds={} current_rounds={} dirty_files={:?} baseline_input_dirty_files={:?}",
        files.len(),
        baseline_input_dirty.len(),
        dirty.len(),
        baseline_rounds,
        current_rounds,
        dirty,
        baseline_input_dirty
    );
    assert_eq!(files.len(), 52, "expected p01.op..p52.op corpus");
    assert!(
        dirty.is_empty(),
        "dirty geometry replay files versus baseline: {dirty:?}"
    );
}

fn geometry_validate_and_fix_without_card_rows(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let mut rounds = 0;
    for _ in 0..MAX_ROUNDS {
        let rects = resolved_rects(sink.state());
        let cmds = {
            let Some(root) = op_editor_core::walkers::find_node(
                sink.state().active_children(),
                &NodeId::new(root_id.to_string()),
            ) else {
                break;
            };
            let Ok(v) = serde_json::to_value(root) else {
                break;
            };
            let mut cmds = Vec::new();
            collect_scale_ops(&v, &rects, &mut cmds);
            collect_collapse_fixes(&v, &rects, &mut cmds);
            collect_text_overflow_fixes(&v, &rects, &mut cmds);
            collect_frame_overflow_fixes(&v, &rects, &mut cmds);
            collect_row_gap_fixes(&v, &rects, &mut cmds);
            collect_row_overfull_fixes(&v, &rects, &mut cmds, false);
            cmds
        };
        if cmds.is_empty() {
            break;
        }
        for cmd in cmds {
            sink.apply(cmd);
        }
        rounds += 1;
    }
    rounds
}

fn is_numbered_corpus_op(name: &str) -> bool {
    name.len() == "p01.op".len()
        && name.starts_with('p')
        && name.ends_with(".op")
        && name[1..3].chars().all(|c| c.is_ascii_digit())
}

#[test]
fn page_level_columns_touching_are_not_a_jam() {
    // An app-shell's [Sidebar | Main] columns legitimately touch — tall
    // page-level columns must never be reported as jammed text cells.
    let row = json!({
        "type":"frame","id":"root","name":"Page","layout":"horizontal","children":[
            {"type":"frame","id":"sb","name":"Sidebar","children":[{"type":"text","id":"a","content":"Nav"}]},
            {"type":"frame","id":"mc","name":"Main","children":[{"type":"text","id":"b","content":"Body"}]}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "root".into(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 1200.0,
            h: 900.0,
        },
    );
    rects.insert(
        "sb".into(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 260.0,
            h: 900.0,
        },
    );
    rects.insert(
        "mc".into(),
        Rect {
            x: 260.0,
            y: 0.0,
            w: 940.0,
            h: 900.0,
        },
    );
    let mut out = Vec::new();
    collect_sibling_jam_diagnostics(&row, &rects, &mut out);
    assert!(out.is_empty(), "page columns are not a jam: {out:?}");
}

#[test]
fn real_layout_gap_fix_reaches_doubly_wrapped_jammed_rows() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    // p01's verbatim shape: table-named frame > unnamed vertical > unnamed
    // vertical > gap-less 4-cell text rows. The NAME gate never sees the rows;
    // the geometry gap fixer must prove the jam from resolved rects and inject
    // a gap regardless of nesting.
    let mkrow = |id: &str| {
        json!({"type":"frame","id":id,"name":null,"layout":"horizontal","width":"fill_container","height":48,"children":[
            {"type":"frame","id":format!("{id}a"),"width":200,"height":40,"children":[{"type":"text","id":format!("{id}at"),"content":"James Wilson","fontSize":14}]},
            {"type":"frame","id":format!("{id}b"),"width":130,"height":40,"children":[{"type":"text","id":format!("{id}bt"),"content":"Oct 24, 2024","fontSize":13}]},
            {"type":"frame","id":format!("{id}c"),"width":110,"height":40,"children":[{"type":"text","id":format!("{id}ct"),"content":"42","fontSize":13}]},
            {"type":"frame","id":format!("{id}d"),"width":100,"height":40,"children":[{"type":"text","id":format!("{id}dt"),"content":"VIP","fontSize":12}]}
        ]})
    };
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Client Directory Data Table","width":800,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"w1","layout":"vertical","width":"fill_container","height":"fit_content","children":[
                {"type":"frame","id":"w2","layout":"vertical","width":"fill_container","height":"fit_content","children":[
                    mkrow("r1"), mkrow("r2"), mkrow("r3")
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
    assert!(geometry_validate_and_fix(&mut sink, &root_id) >= 1);
    let v = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    fn rows_with_gap(v: &serde_json::Value, n: &mut usize) {
        if v.get("layout").and_then(|l| l.as_str()) == Some("horizontal")
            && v.get("gap").and_then(|g| g.as_f64()).unwrap_or(0.0) > 0.0
        {
            *n += 1;
        }
        for c in v
            .get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
        {
            rows_with_gap(c, n);
        }
    }
    let mut n = 0;
    rows_with_gap(&v, &mut n);
    assert!(n >= 3, "all three buried rows got a gap, found {n}");
}

#[test]
fn real_layout_shrinks_rigid_fit_child_overflowing_a_narrow_card() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    // p02's verbatim shape: an 80px card whose fit_content icon+text pair is
    // rigid at max-content (~150px) and paints over siblings. The fixer must
    // retarget it to fill_container (shrinkable); the text inside then wraps
    // via the text-overflow fixer on the next loop round.
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Hero Card","width":80,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"row","layout":"horizontal","gap":8,"width":"fill_container","height":"fit_content","children":[
                {"type":"frame","id":"pair","layout":"horizontal","gap":6,"width":"fit_content","height":"fit_content","children":[
                    {"type":"icon_font","id":"ic","iconFontName":"coffee","width":14,"height":14},
                    {"type":"text","id":"t","content":"Ethiopian Yirgacheffe pour-over","fontSize":13}
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
    fn find<'a>(v: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
        if v.get("name").and_then(|x| x.as_str()) == Some(name) {
            return Some(v);
        }
        v.get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .find_map(|c| find(c, name))
    }
    // The pair was renamed by id remap; find the frame that HOLDS the icon.
    fn find_pair(v: &serde_json::Value) -> Option<&serde_json::Value> {
        let kids = v.get("children").and_then(|c| c.as_array())?;
        if kids
            .iter()
            .any(|c| c.get("type").and_then(|t| t.as_str()) == Some("icon_font"))
        {
            return Some(v);
        }
        kids.iter().find_map(find_pair)
    }
    let _ = find; // silence potential unused in future edits
    let pair = find_pair(&v).expect("icon+text pair survives");
    assert_eq!(
        pair.get("width").and_then(|w| w.as_str()),
        Some("fill_container"),
        "rigid fit pair retargeted to fill, got {:?}",
        pair.get("width")
    );
}

#[test]
fn real_layout_wraps_text_pushed_past_the_row_edge_by_a_sibling() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    // p44's verbatim shape: a 116px centered row holding [36px ellipse, fit
    // text] — the text alone fits the row, but the PAIR overflows and the
    // text's right edge lands past the row edge. The width-only check missed
    // this; the right-edge check must wrap the text.
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Step Card","width":400,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"row","name":"Avatar Row","layout":"horizontal","width":116,"height":"fit_content","justifyContent":"center","alignItems":"center","children":[
                {"type":"ellipse","id":"av","width":36,"height":36},
                {"type":"text","id":"nm","name":"Name","content":"Personalize your workspace","fontSize":14}
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
    fn find<'a>(v: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
        if v.get("name").and_then(|x| x.as_str()) == Some(name) {
            return Some(v);
        }
        v.get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .find_map(|c| find(c, name))
    }
    let nm = find(&v, "Name").expect("text survives");
    assert_eq!(
        nm.get("width").and_then(|w| w.as_str()),
        Some("fill_container"),
        "pair-overflowed text wrapped, got {:?}",
        nm.get("width")
    );
}

#[test]
fn ring_badge_overlay_is_not_reported_as_an_overlap() {
    // A step-ring: ellipse + a short number stacked ON it (center inside) —
    // an intentional overlay, not an overflow accident.
    let row = json!({
        "type":"frame","id":"row","name":"Ring","layout":"horizontal","children":[
            {"type":"ellipse","id":"e","width":36,"height":36},
            {"type":"text","id":"t","content":"2","fontSize":15}
        ]
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "row".into(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 116.0,
            h: 36.0,
        },
    );
    rects.insert(
        "e".into(),
        Rect {
            x: 40.0,
            y: 0.0,
            w: 36.0,
            h: 36.0,
        },
    );
    rects.insert(
        "t".into(),
        Rect {
            x: 53.0,
            y: 9.0,
            w: 10.0,
            h: 18.0,
        },
    ); // centered on the ring
    let mut out = Vec::new();
    collect_sibling_jam_diagnostics(&row, &rects, &mut out);
    assert!(out.is_empty(), "overlay must not be reported: {out:?}");
}

#[test]
fn real_layout_overfull_top_bar_flexifies_until_it_fits() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    // test0703.op (MAISON) verbatim shape: a space_between top bar whose fit
    // title block + fit actions cluster (280px search + date + button) sum
    // wider than the row — the title ran into the search box and the button
    // clipped at the page edge. No single child is wider than the row, so
    // the per-child fixers are blind; the overfull fixer must flexify the
    // widest rigid child (the cluster, then its search) until everything's
    // right edge is back inside the row.
    let doc = r##"{
        "type":"frame","id":"root","name":"Page","width":700,"height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"bar","name":"Top Bar","layout":"horizontal","width":"fill_container","height":"fit_content",
             "justifyContent":"space_between","alignItems":"center","children":[
                {"type":"frame","id":"title","name":"Title Block","layout":"horizontal","gap":12,"width":"fit_content","height":"fit_content","children":[
                    {"type":"text","id":"t1","content":"MANAGEMENT","fontSize":12},
                    {"type":"text","id":"t2","content":"Client Management Suite","fontSize":34}
                ]},
                {"type":"frame","id":"cluster","name":"Right Cluster","layout":"horizontal","gap":24,"width":"fit_content","height":"fit_content","alignItems":"center","children":[
                    {"type":"frame","id":"search","name":"Global Search","layout":"horizontal","gap":8,"width":280,"height":40,"children":[
                        {"type":"text","id":"ph","content":"Search clients...","fontSize":13}
                    ]},
                    {"type":"text","id":"date","content":"Wed, Oct 25","fontSize":13},
                    {"type":"frame","id":"cta","name":"Add Client","layout":"horizontal","width":120,"height":40,"children":[]}
                ]}
            ]}
        ]
    }"##;
    let root: PenNode = serde_json::from_str(doc).expect("valid root");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state().active_children()[0].id_str().to_string();
    let rounds = geometry_validate_and_fix(&mut sink, &root_id);
    assert!(rounds >= 1, "overfull bar must trigger at least one round");

    // Geometry proof: every descendant's right edge sits inside the bar's.
    let rects = resolved_rects(sink.state());
    let v = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    let bar = v["children"][0]["id"].as_str().unwrap();
    let bar_right = {
        let r = &rects[bar];
        r.x + r.w
    };
    fn walk(v: &serde_json::Value, out: &mut Vec<String>) {
        if let Some(id) = v.get("id").and_then(|x| x.as_str()) {
            out.push(id.to_string());
        }
        for c in v
            .get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
        {
            walk(c, out);
        }
    }
    let mut ids = Vec::new();
    walk(&v["children"][0], &mut ids);
    for id in ids {
        if let Some(r) = rects.get(&id) {
            assert!(
                r.x + r.w <= bar_right + 2.0,
                "{id} still hangs past the bar: right={} bar_right={bar_right}",
                r.x + r.w
            );
        }
    }
}

#[test]
fn real_layout_wraps_a_stack_pushed_past_its_cell_by_an_avatar() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    // ATELIER's verbatim shape: a 120px name cell holding [36px avatar, fit
    // name stack]. The stack alone (93px) fits the cell, but avatar + gap
    // push its tail 21px into the NEXT column — the width-only check
    // acquitted it. The right-edge check must flexify the stack; the text
    // inside then wraps on the following round.
    let doc = r##"{
        "type":"frame","id":"page","name":"Page","width":400,"height":300,"layout":"vertical","children":[
            {"type":"frame","id":"row","name":"Row","width":"fill_container","height":"fit_content","layout":"horizontal","gap":24,"children":[
                {"type":"frame","id":"cell","name":"Cell Client","width":120,"height":"fit_content","layout":"horizontal","gap":12,"alignItems":"center","children":[
                    {"type":"frame","id":"av","name":"Avatar","width":36,"height":36,"children":[]},
                    {"type":"frame","id":"stack","name":"Name Stack","width":"fit_content","height":"fit_content","layout":"vertical","gap":2,"children":[
                        {"type":"text","id":"nm","content":"Maximilian Thornebury-Ashworth","fontSize":14,"fontWeight":600},
                        {"type":"text","id":"tier","content":"VIP Member","fontSize":11}
                    ]}
                ]},
                {"type":"frame","id":"contact","name":"Cell Contact","width":"fill_container","height":"fit_content","layout":"vertical","children":[
                    {"type":"text","id":"em","content":"j.thorne@mail.com","fontSize":13}
                ]}
            ]}
        ]
    }"##;
    let root: PenNode = serde_json::from_str(doc).expect("valid root");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state().active_children()[0].id_str().to_string();
    let rounds = geometry_validate_and_fix(&mut sink, &root_id);
    assert!(rounds >= 1, "pushed-out stack must trigger a fix round");

    // Geometry proof: the name stack's right edge is back inside the cell.
    let rects = resolved_rects(sink.state());
    let v = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    fn find_id(v: &serde_json::Value, name: &str) -> Option<String> {
        if v.get("name").and_then(|x| x.as_str()) == Some(name) {
            return v.get("id").and_then(|x| x.as_str()).map(String::from);
        }
        v.get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .find_map(|c| find_id(c, name))
    }
    let cell = &rects[&find_id(&v, "Cell Client").unwrap()];
    let stack = &rects[&find_id(&v, "Name Stack").unwrap()];
    assert!(
        stack.x + stack.w <= cell.x + cell.w + 2.0,
        "stack tail back inside the cell: stack_right={} cell_right={}",
        stack.x + stack.w,
        cell.x + cell.w
    );
}

#[test]
fn late_section_after_bottom_nav_is_echoed_for_the_model() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "name": "Explore",
            "width": 375, "height": "fit_content", "layout": "vertical",
            "children": [
                { "type": "frame", "id": "nav", "name": "Bottom Navigation Bar",
                  "role": "bottom-tab-bar", "width": "fill_container", "height": 72 },
                { "type": "frame", "id": "hdr", "name": "Header & Search",
                  "width": "fill_container", "height": "fit_content" }
            ]
        }]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = super::geometry_diagnostics(&state);
    assert!(
        issues
            .iter()
            .any(|i| i.contains("Header & Search") && i.contains("AFTER the bottom tab bar")),
        "late section must be echoed: {issues:?}"
    );
}

#[test]
fn desktop_roots_and_nav_last_mobile_roots_emit_no_nav_order_echo() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [
            { "type": "frame", "id": "m", "name": "Mobile", "width": 390, "height": 844,
              "children": [
                { "type": "frame", "id": "c", "name": "Content", "width": "fill_container",
                  "height": 400 },
                { "type": "frame", "id": "nav", "name": "Bottom Tab Bar",
                  "role": "bottom-tab-bar", "width": "fill_container", "height": 72 }
              ] }
        ]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = super::geometry_diagnostics(&state);
    assert!(
        !issues
            .iter()
            .any(|i| i.contains("AFTER the bottom tab bar")),
        "nav-last root must not echo: {issues:?}"
    );
}

/// GLM-5.2 measured (test0711-1.op): a 300px-tall image inside a 42px
/// "Avatar" strip painted across half the header. The width-overflow echo
/// is blind to the vertical axis — this echo covers it.
#[test]
fn image_much_taller_than_its_parent_is_echoed_vertically() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "avatar", "name": "Avatar",
                  "width": "fill_container", "height": 42, "layout": "horizontal",
                  "children": [
                    { "type": "image", "id": "img", "name": "woman face headshot", "src": "",
                      "width": "fill_container", "height": 300 }
                  ] },
                { "type": "frame", "id": "body", "name": "Body",
                  "width": "fill_container", "height": "fill_container" }
            ]
        }]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = super::geometry_diagnostics(&state);
    assert!(
        issues
            .iter()
            .any(|i| i.contains("woman face headshot") && i.contains("inflates")),
        "vertical spill must be echoed: {issues:?}"
    );
}

/// `clipContent` parents are intentional croppers — no vertical-spill noise.
#[test]
fn clipping_parent_suppresses_vertical_spill_echo() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "avatar", "name": "Avatar", "clipContent": true,
                  "width": 44, "height": 44, "layout": "horizontal",
                  "children": [
                    { "type": "image", "id": "img", "name": "man face headshot", "src": "",
                      "width": "fill_container", "height": 300 }
                  ] }
            ]
        }]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = super::geometry_diagnostics(&state);
    assert!(
        !issues
            .iter()
            .any(|i| i.contains("inflates") || i.contains("resolved")),
        "clipContent crops on purpose — no echo expected: {issues:?}"
    );
}

/// A 400x300 enrichment image inside a declared 358x170 card cover — jian
/// inflates the card instead of overflowing, so only the declared-size check
/// catches it. The image is retargeted to fill its slot.
#[test]
fn oversized_image_child_is_clamped_to_fill_its_slot() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "cover", "name": "Card Cover",
                  "width": 358, "height": 170, "layout": "vertical",
                  "children": [
                    { "type": "image", "id": "img", "name": "midnight city neon", "src": "",
                      "width": 400, "height": 300 }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    super::geometry_validate_and_fix(&mut sink, "root");

    fn find_by_id<'a>(
        node: &'a jian_ops_schema::node::PenNode,
        id: &str,
    ) -> Option<&'a jian_ops_schema::node::PenNode> {
        use op_editor_core::PenNodeExt;
        if node.id_str() == id {
            return Some(node);
        }
        node.children()?.iter().find_map(|c| find_by_id(c, id))
    }
    let root = &state.active_children()[0];
    let img = find_by_id(root, "img").expect("img");
    {
        use op_editor_core::PenNodeExt;
        assert!(
            img.width_px().is_none() && img.height_px().is_none(),
            "oversized image switches to fill_container on both axes"
        );
    }
}

/// test0711-22 00:44 shape: a fill×fill image inside a `layout:"none"`
/// Cover — `fill_container` is meaningless in an absolute container and the
/// engine painted the cover as a thin right-edge sliver. The image is
/// pinned to the parent's resolved rect.
#[test]
fn fill_image_in_absolute_container_is_pinned_to_parent_rect() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 402, "height": 874, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "cover", "name": "Cover",
                  "width": 160, "height": 160, "layout": "none", "clipContent": true,
                  "children": [
                    { "type": "image", "id": "img", "name": "album art", "src": "",
                      "width": "fill_container", "height": "fill_container" }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    super::geometry_validate_and_fix(&mut sink, "root");

    fn find_by_id<'a>(
        node: &'a jian_ops_schema::node::PenNode,
        id: &str,
    ) -> Option<&'a jian_ops_schema::node::PenNode> {
        use op_editor_core::PenNodeExt;
        if node.id_str() == id {
            return Some(node);
        }
        node.children()?.iter().find_map(|c| find_by_id(c, id))
    }
    let root = &state.active_children()[0];
    let img = find_by_id(root, "img").expect("img");
    {
        use op_editor_core::PenNodeExt;
        assert_eq!(img.width_px(), Some(160.0), "pinned to parent width");
        assert_eq!(img.height_px(), Some(160.0), "pinned to parent height");
    }
}

/// One-off forensic harness: `OP_FORENSIC_FILE=<path> cargo test -p
/// op-orchestrator forensic_resolved_rects -- --ignored --nocapture`
#[test]
#[ignore]
fn forensic_resolved_rects() {
    let Ok(path) = std::env::var("OP_FORENSIC_FILE") else {
        return;
    };
    let json = std::fs::read_to_string(&path).expect("read file");
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(&json).expect("parse");
    let mut state = op_editor_core::EditorState::from_document(doc);
    // `OP_FORENSIC_FIX=1` additionally runs the repair loop on every page
    // root, so the rect dump below shows the POST-fix geometry.
    if std::env::var("OP_FORENSIC_FIX").as_deref() == Ok("1") {
        use op_editor_core::PenNodeExt as _;
        let roots: Vec<String> = state
            .active_children()
            .iter()
            .map(|n| n.id_str().to_string())
            .collect();
        let mut sink = crate::test_support::VecDocSink::new();
        std::mem::swap(&mut sink.state, &mut state);
        for root in roots {
            let rounds = super::geometry_validate_and_fix(&mut sink, &root);
            eprintln!(
                "FIX ROUNDS for {root}: {rounds} ({} commands)",
                sink.applied.len()
            );
        }
        std::mem::swap(&mut sink.state, &mut state);
    }
    let issues = super::geometry_diagnostics(&state);
    eprintln!("DIAGNOSTICS ({}):", issues.len());
    for issue in &issues {
        eprintln!("  - {issue}");
    }
    let scene = op_pen_loader::editor_state_to_layout_scene(&state);
    fn dump(nodes: &[jian_scene::layout_scene::SceneNode], depth: usize) {
        for n in nodes {
            let b = n.aggregate_bounds();
            let kind = format!("{:?}", n.kind);
            eprintln!(
                "{}{} [{kind}] x={:.0} y={:.0} w={:.0} h={:.0}",
                "  ".repeat(depth),
                n.id,
                b.origin.x,
                b.origin.y,
                b.size.x,
                b.size.y
            );
            if depth < 4 {
                dump(&n.children, depth + 1);
            }
        }
    }
    for page in &scene.pages {
        dump(&page.children, 0);
    }
}

/// test0711-2-ds: a card row declared 156 tall whose children resolve 165 —
/// the 9px overshoot hid the artist line's bottom under the next section.
/// Small overshoots grow the frame; the big-inflation class stays an echo.
#[test]
fn slightly_short_fixed_frame_grows_to_fit_its_children() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "rail", "name": "Card Rail",
                  "width": "fill_container", "height": 156, "layout": "horizontal", "gap": 12,
                  "children": [
                    { "type": "frame", "id": "card", "width": 140, "height": 156,
                      "layout": "vertical", "gap": 8,
                      "children": [
                        { "type": "frame", "id": "cover", "width": 140, "height": 120 },
                        { "type": "text", "id": "t1", "content": "Blinding Lights",
                          "width": "fit_content", "height": 18 },
                        { "type": "text", "id": "t2", "content": "The Weeknd",
                          "width": "fit_content", "height": 15 }
                      ] }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    super::geometry_validate_and_fix(&mut sink, "root");

    fn find_by_id<'a>(
        node: &'a jian_ops_schema::node::PenNode,
        id: &str,
    ) -> Option<&'a jian_ops_schema::node::PenNode> {
        use op_editor_core::PenNodeExt;
        if node.id_str() == id {
            return Some(node);
        }
        node.children()?.iter().find_map(|c| find_by_id(c, id))
    }
    let root = &state.active_children()[0];
    let card = find_by_id(root, "card").expect("card");
    {
        use op_editor_core::PenNodeExt;
        assert!(
            card.height_px().is_some_and(|h| h > 156.0),
            "card grew to cover its children, got {:?}",
            card.height_px()
        );
    }
}

// ── starved rail-card fixes ──

fn rail_card(id: &str, card_w: serde_json::Value, inner_w: f64) -> serde_json::Value {
    json!({
        "type": "frame", "id": id, "name": id, "width": card_w,
        "height": "fill_container", "layout": "vertical", "clipContent": true,
        "children": [
            { "type": "frame", "id": format!("{id}-img"), "width": inner_w, "height": 190 },
            { "type": "frame", "id": format!("{id}-label"), "width": inner_w + 28.0, "height": 56 }
        ]
    })
}

/// The measured test0711-1-glm shape: a 5-card horizontal rail, every card
/// width fill_container (~58px share) while carrying 160px fixed content.
fn starved_rail() -> serde_json::Value {
    let cards: Vec<serde_json::Value> = (0..5)
        .map(|i| rail_card(&format!("card{i}"), json!("fill_container"), 160.0))
        .collect();
    json!({
        "type": "frame", "id": "rail", "name": "PD Rail", "width": "fill_container",
        "layout": "horizontal", "gap": 12, "justifyContent": "space_between",
        "children": cards
    })
}

fn starved_rail_rects() -> std::collections::HashMap<String, Rect> {
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "rail".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 342.0,
            h: 254.0,
        },
    );
    for i in 0..5 {
        rects.insert(
            format!("card{i}"),
            Rect {
                x: i as f64 * 70.0,
                y: 0.0,
                w: 58.0,
                h: 254.0,
            },
        );
    }
    rects
}

#[test]
fn starved_rail_cards_hug_and_rail_becomes_scroller() {
    let mut cmds = Vec::new();
    collect_starved_rail_card_fixes(&starved_rail(), &starved_rail_rects(), &mut cmds);
    let hugged: Vec<&str> = cmds
        .iter()
        .filter_map(|c| match c {
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Keyword(k),
            } if property == "width" && k == "fit_content" => Some(node_id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(hugged.len(), 5, "all five cards hug: {cmds:?}");
    let rail_clipped = cmds.iter().any(|c| {
        matches!(c,
        EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Bool(true) }
            if node_id.as_str() == "rail" && property == "clipContent")
    });
    assert!(
        rail_clipped,
        "rail marked scroller so the overfull flexifier skips it"
    );
    let spread_dropped = cmds.iter().any(|c| {
        matches!(c,
        EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Keyword(k) }
            if node_id.as_str() == "rail" && property == "justifyContent" && k == "start")
    });
    assert!(spread_dropped, "space_between falls back to start+gap");
}

#[test]
fn two_fill_columns_with_fitting_content_are_untouched() {
    // An app-shell's two fill columns whose fixed content FITS the share —
    // and below RAIL_MIN_CARDS anyway. Must not become a scroller.
    let cols: Vec<serde_json::Value> = (0..2)
        .map(|i| rail_card(&format!("col{i}"), json!("fill_container"), 300.0))
        .collect();
    let shell = json!({
        "type": "frame", "id": "shell", "width": "fill_container",
        "layout": "horizontal", "gap": 24, "children": cols
    });
    let mut rects = std::collections::HashMap::new();
    rects.insert(
        "shell".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        },
    );
    for i in 0..2 {
        rects.insert(
            format!("col{i}"),
            Rect {
                x: i as f64 * 400.0,
                y: 0.0,
                w: 388.0,
                h: 600.0,
            },
        );
    }
    let mut cmds = Vec::new();
    collect_starved_rail_card_fixes(&shell, &rects, &mut cmds);
    assert!(cmds.is_empty(), "fitting columns untouched: {cmds:?}");
}

#[test]
fn already_clipped_rail_is_left_alone() {
    let mut rail = starved_rail();
    rail["clipContent"] = json!(true);
    let mut cmds = Vec::new();
    collect_starved_rail_card_fixes(&rail, &starved_rail_rects(), &mut cmds);
    assert!(
        cmds.is_empty(),
        "an authored scroller is intentional: {cmds:?}"
    );
}

#[test]
fn rail_with_one_flexible_card_is_not_forced_to_hug() {
    // One card has NO fixed content (a genuine flex spacer/card) — the
    // all-starved gate must hold the repair back.
    let mut rail = starved_rail();
    rail["children"][2]["children"] = json!([]);
    let mut cmds = Vec::new();
    collect_starved_rail_card_fixes(&rail, &starved_rail_rects(), &mut cmds);
    assert!(cmds.is_empty(), "mixed rail left to the echo: {cmds:?}");
}
