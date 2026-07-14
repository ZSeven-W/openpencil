use crate::cleanup::run_cleanup_passes;
use crate::geometry_validation::geometry_diagnostics;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct TestRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 1200.0,
            height: 800.0,
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
    let root: PenNode = serde_json::from_value(value).expect("valid root");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink
}

fn run_cleanup(sink: &mut VecDocSink) {
    let root_id = sink.state().active_children()[0].id_str().to_string();
    run_cleanup_passes(sink, &plan(), &[&root_id]);
}

fn active_root_json(sink: &VecDocSink) -> Value {
    serde_json::to_value(sink.state().active_children()[0].clone()).expect("root json")
}

fn find_by_name<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    if v.get("name").and_then(Value::as_str) == Some(name) {
        return Some(v);
    }
    v.get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|c| find_by_name(c, name))
}

fn resolved_rects(sink: &VecDocSink) -> HashMap<String, TestRect> {
    let scene = op_pen_loader::editor_state_to_layout_scene(sink.state());
    let mut out = HashMap::new();
    fn walk(nodes: &[jian_scene::layout_scene::SceneNode], out: &mut HashMap<String, TestRect>) {
        for node in nodes {
            let b = node.aggregate_bounds();
            out.insert(
                node.id.clone(),
                TestRect {
                    x: f64::from(b.origin.x),
                    y: f64::from(b.origin.y),
                    w: f64::from(b.size.x),
                    h: f64::from(b.size.y),
                },
            );
            walk(&node.children, out);
        }
    }
    for page in &scene.pages {
        walk(&page.children, &mut out);
    }
    out
}

#[test]
fn cleanup_centers_direct_arc_ellipse_stack_and_label() {
    let mut sink = insert_root(json!({
        "type":"frame","id":"root","name":"Root","width":240,"height":240,"layout":"vertical","children":[
            {"type":"frame","id":"donut","name":"Donut Chart","width":180,"height":180,"layout":"horizontal","gap":0,"children":[
                {"type":"ellipse","id":"seg1","name":"Segment 1","width":180,"height":180,"startAngle":0,"sweepAngle":158,"innerRadius":0.7},
                {"type":"ellipse","id":"seg2","name":"Segment 2","width":180,"height":180,"startAngle":158,"sweepAngle":115,"innerRadius":0.7},
                {"type":"ellipse","id":"seg3","name":"Segment 3","width":180,"height":180,"startAngle":273,"sweepAngle":65,"innerRadius":0.7},
                {"type":"ellipse","id":"seg4","name":"Segment 4","width":180,"height":180,"startAngle":338,"sweepAngle":22,"innerRadius":0.7},
                {"type":"frame","id":"label","name":"Center Label","width":"fill_container","height":"fit_content","layout":"vertical","gap":4,"children":[
                    {"type":"text","id":"pct","name":"Percent","content":"72%","width":"fill_container","height":"fit_content","fontSize":22,"textGrowth":"fixed-width"},
                    {"type":"text","id":"sub","name":"Subtitle","content":"Complete","width":"fill_container","height":"fit_content","fontSize":11,"textGrowth":"fixed-width"}
                ]}
            ]}
        ]
    }));

    run_cleanup(&mut sink);

    let root = active_root_json(&sink);
    let donut = find_by_name(&root, "Donut Chart").expect("donut exists");
    assert_eq!(donut["layout"], json!("none"));

    let rects = resolved_rects(&sink);
    let parent_id = donut.get("id").and_then(Value::as_str).expect("donut id");
    let parent = rects.get(parent_id).expect("donut rect");
    let expected = (parent.x + parent.w / 2.0, parent.y + parent.h / 2.0);
    for name in [
        "Segment 1",
        "Segment 2",
        "Segment 3",
        "Segment 4",
        "Center Label",
    ] {
        let node = find_by_name(&root, name).unwrap_or_else(|| panic!("{name} exists"));
        let id = node.get("id").and_then(Value::as_str).expect("node id");
        let r = rects.get(id).unwrap_or_else(|| panic!("{id} rect"));
        let center = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        // 4px: single-line font metrics still differ ~1-2px per platform
        // font stack; on a 180px ring that is invisible. Wrap-induced
        // offsets (12px+) stay caught.
        assert!(
            (center.0 - expected.0).abs() <= 4.0 && (center.1 - expected.1).abs() <= 4.0,
            "{name} center {:?} must match parent center {:?}",
            center,
            expected
        );
    }
}

#[test]
fn cleanup_does_not_repair_single_arc_or_wrapped_card_arcs() {
    let mut sink = insert_root(json!({
        "type":"frame","id":"root","name":"Root","width":360,"height":220,"layout":"vertical","children":[
            {"type":"frame","id":"single","name":"Single Arc","width":120,"height":120,"layout":"horizontal","children":[
                {"type":"ellipse","id":"only","name":"Only Arc","width":120,"height":120,"sweepAngle":180,"innerRadius":0.7}
            ]},
            {"type":"frame","id":"cards","name":"Cards Row","width":300,"height":120,"layout":"horizontal","children":[
                {"type":"frame","id":"card1","name":"Card 1","width":120,"height":120,"children":[
                    {"type":"ellipse","id":"arc1","name":"Arc 1","width":80,"height":80,"sweepAngle":120,"innerRadius":0.7}
                ]},
                {"type":"frame","id":"card2","name":"Card 2","width":120,"height":120,"children":[
                    {"type":"ellipse","id":"arc2","name":"Arc 2","width":80,"height":80,"sweepAngle":120,"innerRadius":0.7}
                ]}
            ]}
        ]
    }));

    run_cleanup(&mut sink);

    let root = active_root_json(&sink);
    assert_eq!(
        find_by_name(&root, "Single Arc").unwrap()["layout"],
        json!("horizontal")
    );
    assert_eq!(
        find_by_name(&root, "Cards Row").unwrap()["layout"],
        json!("horizontal")
    );
}

#[test]
fn cleanup_does_not_stack_independent_partial_gauges_in_one_row() {
    let mut sink = insert_root(json!({
        "type":"frame","id":"root","name":"Root","width":375,"height":220,"layout":"vertical","children":[
            {"type":"frame","id":"gauges","name":"Independent Gauges","width":136,"height":48,
             "layout":"horizontal","gap":8,"children":[
                {"type":"ellipse","id":"gauge-a","name":"Gauge A","width":40,"height":40,
                 "innerRadius":0.75,"startAngle":-90,"sweepAngle":180},
                {"type":"ellipse","id":"gauge-b","name":"Gauge B","width":40,"height":40,
                 "innerRadius":0.75,"startAngle":-90,"sweepAngle":180},
                {"type":"text","id":"label","name":"Gauge Label","width":40,"height":20,"content":"Goals"}
             ]}
        ]
    }));

    run_cleanup(&mut sink);

    let root = active_root_json(&sink);
    let row = find_by_name(&root, "Independent Gauges").expect("gauge row");
    assert_eq!(row["layout"], json!("horizontal"));
}

#[test]
fn cleanup_recenters_radial_stack_after_geometry_changes_wrapper_width() {
    let mut sink = insert_root(json!({
        "type":"frame","id":"root","name":"Root","width":375,"height":812,
        "layout":"vertical","padding":[0,24],"children":[
            {"type":"frame","id":"card","name":"Activity Card","width":"fill_container",
             "height":"fit_content","layout":"vertical","padding":20,"gap":20,"children":[
                {"type":"frame","id":"ring","name":"Ring","width":320,"height":120,
                 "layout":"horizontal","gap":0,"alignItems":"center","justifyContent":"center","children":[
                    {"type":"ellipse","id":"track","name":"Ring Track","width":120,"height":120,
                     "innerRadius":0.86},
                    {"type":"ellipse","id":"progress","name":"Ring Progress","width":120,"height":120,
                     "innerRadius":0.86,"startAngle":-90,"sweepAngle":264},
                    {"type":"frame","id":"center","name":"Ring Center","width":98,"height":43,
                     "layout":"vertical","children":[
                        {"type":"text","id":"value","content":"8,420","fontSize":24},
                        {"type":"text","id":"label","content":"steps","fontSize":12}
                    ]}
                ]}
            ]}
        ]
    }));

    run_cleanup(&mut sink);

    let root = active_root_json(&sink);
    let ring = find_by_name(&root, "Ring").expect("ring exists");
    assert_eq!(ring["layout"], json!("none"));
    assert_eq!(
        ring["width"],
        json!("fill_container"),
        "geometry must exercise the late numeric-to-fill width change"
    );

    let rects = resolved_rects(&sink);
    let ring_id = ring.get("id").and_then(Value::as_str).expect("ring id");
    let parent = rects.get(ring_id).expect("ring rect");
    let expected = (parent.x + parent.w / 2.0, parent.y + parent.h / 2.0);
    for name in ["Ring Track", "Ring Progress", "Ring Center"] {
        let child = find_by_name(&root, name).unwrap_or_else(|| panic!("{name} exists"));
        let child_id = child.get("id").and_then(Value::as_str).expect("child id");
        let rect = rects
            .get(child_id)
            .unwrap_or_else(|| panic!("{child_id} rect"));
        let center = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        assert!(
            (center.0 - expected.0).abs() <= 1.0 && (center.1 - expected.1).abs() <= 1.0,
            "{name} center {center:?} must match final wrapper center {expected:?}"
        );
    }
}

#[test]
fn cleanup_deletes_only_empty_decorated_small_frame_stubs() {
    let mut sink = insert_root(json!({
        "type":"frame","id":"root","name":"Root","width":390,"height":360,"layout":"vertical","children":[
            {"type":"frame","id":"discount","name":"Discount Badge 1","padding":[5,10],"cornerRadius":9999,
             "fill":[{"type":"solid","color":"#F97316"}],"children":[]},
            {"type":"frame","id":"book","name":"Book Btn 1","padding":[8,14],"cornerRadius":9999,
             "fill":[{"type":"solid","color":"#E5E7EB"}],"children":[]},
            {"type":"ellipse","id":"dot","name":"Status Dot","width":6,"height":6,"fill":[{"type":"solid","color":"#22C55E"}]},
            {"type":"frame","id":"badge","name":"Real Badge","width":58,"height":24,"padding":[5,10],"cornerRadius":9999,
             "fill":[{"type":"solid","color":"#F97316"}],"children":[{"type":"text","id":"badge-t","content":"-25%"}]},
            {"type":"frame","id":"skeleton","name":"Skeleton","width":120,"height":80,"cornerRadius":12,
             "fill":[{"type":"solid","color":"#E5E7EB"}],"children":[]},
            {"type":"frame","id":"spacer","name":"Spacer","width":20,"height":20,"padding":8,"children":[]}
        ]
    }));

    run_cleanup(&mut sink);

    let root = active_root_json(&sink);
    assert!(find_by_name(&root, "Discount Badge 1").is_none());
    assert!(find_by_name(&root, "Book Btn 1").is_none());
    assert!(find_by_name(&root, "Status Dot").is_some());
    assert!(find_by_name(&root, "Real Badge").is_some());
    assert!(find_by_name(&root, "Skeleton").is_some());
    assert!(find_by_name(&root, "Spacer").is_some());
}

#[test]
fn geometry_diagnostics_reports_empty_decorated_frame_stub() {
    let sink = insert_root(json!({
        "type":"frame","id":"root","name":"Root","width":390,"height":160,"layout":"vertical","children":[
            {"type":"frame","id":"discount","name":"Discount Badge 1","width":58,"height":24,"padding":[5,10],"cornerRadius":9999,
             "fill":[{"type":"solid","color":"#F97316"}],"children":[]}
        ]
    }));

    let diagnostics = geometry_diagnostics(sink.state());

    assert!(
        diagnostics.iter().any(|line| {
            line.contains("Discount Badge 1") && line.contains("empty decorated frame")
        }),
        "diagnostics must mention the empty decorated frame: {diagnostics:?}"
    );
}
