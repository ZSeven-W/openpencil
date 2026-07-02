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
    rects.insert(
        "n1".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 40.0,
        },
    );
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
/// OP_PROBE_NAME inside OP_REPAIR_IN.
#[test]
#[ignore]
fn resolved_rect_probe() {
    let inp = std::env::var("OP_REPAIR_IN").expect("OP_REPAIR_IN");
    let pat = std::env::var("OP_PROBE_NAME")
        .unwrap_or_default()
        .to_lowercase();
    let text = std::fs::read_to_string(&inp).expect("read input");
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(&text).expect("parse .op");
    let state = op_editor_core::EditorState::from_document(doc);
    let rects = resolved_rects(&state);
    for root in state.active_children() {
        let v = serde_json::to_value(root).unwrap();
        fn walk(v: &serde_json::Value, rects: &HashMap<String, Rect>, pat: &str, depth: usize) {
            let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
            if !pat.is_empty() && name.to_lowercase().contains(pat) {
                if let Some(r) = v
                    .get("id")
                    .and_then(|x| x.as_str())
                    .and_then(|id| rects.get(id))
                {
                    eprintln!(
                        "{:indent$}{name}: x={:.0} y={:.0} w={:.0} h={:.0}",
                        "",
                        r.x,
                        r.y,
                        r.w,
                        r.h,
                        indent = depth
                    );
                }
            }
            for c in v
                .get("children")
                .and_then(|c| c.as_array())
                .into_iter()
                .flatten()
            {
                walk(c, rects, pat, depth + 1);
            }
        }
        walk(&v, &rects, &pat, 0);
    }
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
    fn find_pair<'a>(v: &'a serde_json::Value) -> Option<&'a serde_json::Value> {
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
