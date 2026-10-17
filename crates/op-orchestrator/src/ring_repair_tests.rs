use super::*;
use serde_json::json;

/// The reproduction: a padded card holding `[75% label, progress arc, track]`
/// as flex siblings. Every pre-existing pass declines it — `radial_repair`
/// because the padded 320x160 card is not the ring's own wrapper — so the ring
/// renders as two circles in a row with the label off to one side.
fn screenshot_card() -> Value {
    json!({
        "type":"frame","id":"card","name":"Storage Card","layout":"horizontal","gap":12,
        "width":320,"height":160,"padding":[16,16],
        "children":[
            {"type":"text","id":"pct","content":"75%","fontSize":24},
            {"type":"ellipse","id":"progress","width":120,"height":120,
             "innerRadius":0.8,"sweepAngle":270},
            {"type":"ellipse","id":"track","width":120,"height":120,"innerRadius":0.8}
        ]
    })
}

fn wrap(mut value: Value) -> (bool, Value) {
    let changed = wrap_in_value(&mut value);
    (changed, value)
}

fn child_ids(v: &Value) -> Vec<&str> {
    children(v)
        .iter()
        .filter_map(|c| c.get("id").and_then(Value::as_str))
        .collect()
}

#[test]
fn flex_sibling_ring_is_extracted_into_a_concentric_wrapper() {
    let (changed, card) = wrap(screenshot_card());
    assert!(changed, "the screenshot fixture must be repaired");

    // The card keeps its own identity: layout, padding, size all untouched.
    assert_eq!(card["layout"], json!("horizontal"));
    assert_eq!(card["padding"], json!([16, 16]));
    assert_eq!(card["width"], json!(320));

    // The flex row no longer carries the two circles — just one wrapper.
    assert_eq!(child_ids(&card), ["card__ring-stack"]);

    let stack = &children(&card)[0];
    assert_eq!(stack["layout"], json!("none"), "wrapper must be an overlay");
    assert_eq!(stack["width"], json!(120.0));
    assert_eq!(stack["height"], json!(120.0));
}

#[test]
fn wrapper_children_are_concentric_and_in_canonical_paint_order() {
    let (_, card) = wrap(screenshot_card());
    let stack = &children(&card)[0];

    // Index 0 paints on TOP (the canvas walks children in reverse), so the
    // order is centre label, then progress arc, then the full track.
    assert_eq!(child_ids(stack), ["pct", "progress", "track"]);

    // Both arcs are the same size at the same coordinates — that is what
    // "concentric" means once they share a layout:none box.
    for id in ["progress", "track"] {
        let arc = children(stack)
            .iter()
            .find(|c| c["id"] == json!(id))
            .expect("arc present");
        assert_eq!(
            (arc["x"].clone(), arc["y"].clone()),
            (json!(0.0), json!(0.0))
        );
        assert_eq!(arc["width"], json!(120.0));
        assert_eq!(arc["height"], json!(120.0));
    }

    // The label is centred, and carries explicit pixels: under layout:none a
    // fill_container child would collapse to zero.
    let label = &children(stack)[0];
    assert!(label["x"].as_f64().unwrap() > 0.0);
    assert!(label["y"].as_f64().unwrap() > 0.0);
    assert!(label["width"].as_f64().is_some());
    assert!(label["height"].as_f64().is_some());
}

#[test]
fn wrapper_takes_the_first_arcs_slot_and_leaves_other_content_alone() {
    let section = json!({
        "type":"frame","id":"sec","layout":"vertical","gap":16,"width":360,"height":420,
        "children":[
            {"type":"text","id":"title","content":"Storage","fontSize":18},
            {"type":"ellipse","id":"progress","width":120,"height":120,
             "innerRadius":0.8,"sweepAngle":270},
            {"type":"ellipse","id":"track","width":120,"height":120,"innerRadius":0.8},
            {"type":"text","id":"caption","content":"12 GB of 16 GB used","fontSize":13}
        ]
    });
    let (changed, section) = wrap(section);
    assert!(changed);

    // Ring collapses into one wrapper AT THE FIRST ARC'S POSITION; the
    // heading and caption keep their own places in the column.
    assert_eq!(child_ids(&section), ["title", "sec__ring-stack", "caption"]);
    assert_eq!(section["layout"], json!("vertical"));
}

#[test]
fn ambiguous_text_is_not_adopted_as_centre_content() {
    // Two non-arc children: which (if either) is the centre label is intent,
    // not a fact. Only the arcs move.
    let card = json!({
        "type":"frame","id":"card","layout":"vertical","gap":8,"width":300,"height":300,
        "children":[
            {"type":"text","id":"title","content":"Used","fontSize":14},
            {"type":"text","id":"pct","content":"75%","fontSize":24},
            {"type":"ellipse","id":"progress","width":120,"height":120,
             "innerRadius":0.8,"sweepAngle":270},
            {"type":"ellipse","id":"track","width":120,"height":120,"innerRadius":0.8}
        ]
    });
    let (changed, card) = wrap(card);
    assert!(changed, "the arcs still need stacking");
    assert_eq!(child_ids(&card), ["title", "pct", "card__ring-stack"]);
    assert_eq!(child_ids(&children(&card)[2]), ["progress", "track"]);
}

#[test]
fn non_percentage_label_is_not_adopted() {
    let card = json!({
        "type":"frame","id":"card","layout":"horizontal","gap":8,"width":320,"height":300,
        "children":[
            {"type":"text","id":"label","content":"Disk usage","fontSize":14},
            {"type":"ellipse","id":"progress","width":120,"height":120,
             "innerRadius":0.8,"sweepAngle":270},
            {"type":"ellipse","id":"track","width":120,"height":120,"innerRadius":0.8}
        ]
    });
    let (changed, card) = wrap(card);
    assert!(changed);
    assert_eq!(child_ids(&card), ["label", "card__ring-stack"]);
    assert_eq!(child_ids(&children(&card)[1]), ["progress", "track"]);
}

// ── False-positive guards: legitimate structures that must NOT be touched ──

#[test]
fn decorative_dot_siblings_are_left_alone() {
    // Plain filled circles — no innerRadius, no sweepAngle. Not ring parts.
    let row = json!({
        "type":"frame","id":"dots","layout":"horizontal","gap":6,"width":200,"height":40,
        "padding":[8,8],
        "children":[
            {"type":"ellipse","id":"d1","width":8,"height":8,"fill":"#fff"},
            {"type":"ellipse","id":"d2","width":8,"height":8,"fill":"#999"},
            {"type":"ellipse","id":"d3","width":8,"height":8,"fill":"#999"}
        ]
    });
    let (changed, after) = wrap(row.clone());
    assert!(!changed, "carousel dots are not a ring");
    assert_eq!(after, row);
}

#[test]
fn chart_legend_dot_row_is_left_alone() {
    let legend = json!({
        "type":"frame","id":"legend","layout":"horizontal","gap":16,"width":320,"height":24,
        "padding":[4,8],
        "children":[
            {"type":"frame","id":"i1","layout":"horizontal","gap":6,"children":[
                {"type":"ellipse","id":"sw1","width":10,"height":10,"fill":"#4f46e5"},
                {"type":"text","id":"t1","content":"Used","fontSize":12}
            ]},
            {"type":"frame","id":"i2","layout":"horizontal","gap":6,"children":[
                {"type":"ellipse","id":"sw2","width":10,"height":10,"fill":"#a5b4fc"},
                {"type":"text","id":"t2","content":"Free","fontSize":12}
            ]}
        ]
    });
    let (changed, after) = wrap(legend.clone());
    assert!(!changed, "legend swatches are not ring fragments");
    assert_eq!(after, legend);
}

#[test]
fn single_ring_beside_a_label_is_left_alone() {
    // One track ellipse and a label: no progress arc, so no concentric pair
    // to stack — how to lay this out is the model's call.
    let row = json!({
        "type":"frame","id":"row","layout":"horizontal","gap":12,"width":320,"height":160,
        "padding":[16,16],
        "children":[
            {"type":"ellipse","id":"ring","width":120,"height":120,"innerRadius":0.8},
            {"type":"text","id":"pct","content":"75%","fontSize":24}
        ]
    });
    let (changed, after) = wrap(row.clone());
    assert!(!changed);
    assert_eq!(after, row);
}

#[test]
fn already_concentric_wrapper_is_left_alone() {
    let stack = json!({
        "type":"frame","id":"stack","layout":"none","width":120,"height":120,
        "children":[
            {"type":"text","id":"pct","content":"75%","x":33,"y":46,"width":54,"height":29},
            {"type":"ellipse","id":"progress","x":0,"y":0,"width":120,"height":120,
             "innerRadius":0.8,"sweepAngle":270},
            {"type":"ellipse","id":"track","x":0,"y":0,"width":120,"height":120,
             "innerRadius":0.8}
        ]
    });
    let (changed, after) = wrap(stack.clone());
    assert!(!changed, "an overlay stack is already correct");
    assert_eq!(after, stack);
}

#[test]
fn dedicated_ring_wrapper_in_flow_stays_with_radial_repair() {
    // A bare wrapper whose whole content IS the ring: `radial_repair` converts
    // this parent in place today. This pass must not race it.
    let wrapper = json!({
        "type":"frame","id":"ring","layout":"horizontal","gap":0,
        "children":[
            {"type":"text","id":"pct","content":"75%","fontSize":24},
            {"type":"ellipse","id":"progress","width":120,"height":120,
             "innerRadius":0.8,"sweepAngle":270},
            {"type":"ellipse","id":"track","width":120,"height":120,"innerRadius":0.8}
        ]
    });
    let (changed, after) = wrap(wrapper.clone());
    assert!(!changed, "radial_repair owns the dedicated-wrapper case");
    assert_eq!(after, wrapper);
}

#[test]
fn mismatched_gauge_diameters_are_not_treated_as_one_ring() {
    let row = json!({
        "type":"frame","id":"gauges","layout":"horizontal","gap":24,"width":400,"height":200,
        "padding":[16,16],
        "children":[
            {"type":"ellipse","id":"big","width":160,"height":160,
             "innerRadius":0.8,"sweepAngle":270},
            {"type":"ellipse","id":"small","width":64,"height":64,"innerRadius":0.8}
        ]
    });
    let (changed, after) = wrap(row.clone());
    assert!(!changed, "two differently sized gauges are not one ring");
    assert_eq!(after, row);
}

// ── Percentage-content predicate ──

#[test]
fn percentage_label_reads_content_not_names() {
    let pct = |content: &str| json!({"type":"text","id":"x","content":content});
    for good in ["75%", " 75% ", "8%", "99.5%", "100%"] {
        assert!(is_percentage_label(&pct(good)), "{good} is a percentage");
    }
    for bad in ["75", "%", "up 75% today", "75 %%", "abc%", ""] {
        assert!(!is_percentage_label(&pct(bad)), "{bad} is not a percentage");
    }
    // A NAME of "percent" proves nothing without percentage content.
    assert!(!is_percentage_label(
        &json!({"type":"text","id":"x","name":"percent","content":"Used"})
    ));
}

// ── Detection echo ──

#[test]
fn diagnostic_reports_the_violation_the_repair_acts_on() {
    let mut out = Vec::new();
    push_ring_fragment_diagnostics(&screenshot_card(), &mut out, 8);
    assert_eq!(out.len(), 1, "one violation reported");
    let message = &out[0];
    assert!(message.contains("Storage Card"));
    assert!(message.contains("FLEX SIBLINGS"));
    assert!(message.contains("120"));
}

#[test]
fn diagnostic_is_silent_on_legitimate_structures() {
    let legit = json!({
        "type":"frame","id":"root","layout":"vertical","children":[
            {"type":"frame","id":"dots","layout":"horizontal","width":200,"height":40,
             "children":[
                {"type":"ellipse","id":"d1","width":8,"height":8},
                {"type":"ellipse","id":"d2","width":8,"height":8}
             ]},
            {"type":"frame","id":"stack","layout":"none","width":120,"height":120,
             "children":[
                {"type":"ellipse","id":"p","x":0,"y":0,"width":120,"height":120,
                 "innerRadius":0.8,"sweepAngle":270},
                {"type":"ellipse","id":"t","x":0,"y":0,"width":120,"height":120,
                 "innerRadius":0.8}
             ]}
        ]
    });
    let mut out = Vec::new();
    push_ring_fragment_diagnostics(&legit, &mut out, 8);
    assert!(out.is_empty(), "no violation to report: {out:?}");
}

#[test]
fn diagnostic_respects_the_shared_budget() {
    let mut out = vec!["existing".to_string(); 8];
    push_ring_fragment_diagnostics(&screenshot_card(), &mut out, 8);
    assert_eq!(out.len(), 8, "budget already full — nothing appended");
}

// ── Schema round-trip ──

#[test]
fn repaired_tree_round_trips_through_the_canonical_schema() {
    let mut root: PenNode =
        serde_json::from_value(screenshot_card()).expect("fixture is valid schema");
    assert!(wrap_ring_fragments(&mut root), "repair applies via PenNode");

    let value = serde_json::to_value(&root).expect("serialize");
    let stack = &children(&value)[0];
    assert_eq!(stack["layout"], json!("none"));
    assert_eq!(child_ids(stack), ["pct", "progress", "track"]);
}

// ── End-to-end through the real cleanup pipeline ──

/// The whole point of the fix: the screenshot case must come out of
/// `run_cleanup_passes` as a concentric ring, on the pass sequence BOTH the
/// classic orchestrator and the agentic loop run.
#[test]
fn cleanup_pipeline_repairs_the_screenshot_case() {
    use crate::plan::{OrchestratorPlan, RootFrameSpec};
    use op_editor_core::EditorState;

    let root = json!({
        "type":"frame","id":"root","name":"Screen","layout":"vertical","gap":16,
        "width":390,"height":600,
        "children":[ screenshot_card() ]
    });
    let node: PenNode = serde_json::from_value(root).expect("valid schema");
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(node);

    let plan = OrchestratorPlan {
        shared_context: None,
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Screen".into(),
            width: 390.0,
            height: 600.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
        palette: None,
    };

    let mut summary = crate::repair_summary::RepairSummary::default();
    {
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        crate::cleanup::run_cleanup_passes_with_summary(&mut sink, &plan, &["root"], &mut summary);
    }

    let out = serde_json::to_value(&state.active_children()[0]).expect("serialize");
    // The card row must no longer hold two loose circles.
    let card = children(&out)
        .iter()
        .find(|c| c["name"] == json!("Storage Card"))
        .expect("card survived cleanup");
    let loose_arcs = children(card)
        .iter()
        .filter(|c| c["type"] == json!("ellipse"))
        .count();
    assert_eq!(loose_arcs, 0, "no arc may remain a flex sibling: {card}");

    // A concentric overlay wrapper now carries both arcs at one origin.
    let stack = children(card)
        .iter()
        .find(|c| c["layout"] == json!("none"))
        .expect("ring wrapper created");
    let arcs: Vec<&Value> = children(stack)
        .iter()
        .filter(|c| c["type"] == json!("ellipse"))
        .collect();
    assert_eq!(arcs.len(), 2, "both arcs stacked");
    assert_eq!(arcs[0]["x"], arcs[1]["x"], "arcs share an origin");
    assert_eq!(arcs[0]["y"], arcs[1]["y"], "arcs share an origin");

    // And the repair is accounted for in the user-facing quality credential.
    assert!(
        summary.repairs_for(crate::repair_summary::CheckCategory::Structure) > 0,
        "structural repair must be counted"
    );
}
