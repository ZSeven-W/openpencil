use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

use crate::orchestration_self_check::{
    auto_fix_fixable_issues, check_generated_nodes, check_value_forest, SelfCheckReport,
};

fn has_radial_issue(report: &SelfCheckReport) -> bool {
    report
        .issues
        .iter()
        .any(|issue| issue.code == "radial-stack-not-concentric")
}

fn visible_arc(id: &str, size: f64, sweep: f64) -> Value {
    json!({
        "type": "ellipse",
        "id": id,
        "width": size,
        "height": size,
        "innerRadius": 0.82,
        "startAngle": -90,
        "sweepAngle": sweep,
        "fill": [{"type": "solid", "color": "#22C55E"}]
    })
}

fn fixed_ring(layout: Option<&str>) -> Value {
    let mut ring = json!({
        "type": "frame",
        "id": "ring",
        "name": "Steps Ring",
        "width": 120,
        "height": 120,
        "children": [
            visible_arc("track", 120.0, 360.0),
            visible_arc("progress", 116.0, 264.0),
            {
                "type": "frame",
                "id": "centre",
                "name": "Ring Centre",
                "width": 80,
                "height": 44,
                "layout": "vertical",
                "children": [
                    {"type": "text", "id": "value", "content": "8,432"},
                    {"type": "text", "id": "label", "content": "steps"}
                ]
            }
        ]
    });
    if let Some(layout) = layout {
        ring["layout"] = Value::String(layout.to_string());
    }
    ring
}

fn canonical_fixed_ring() -> Value {
    let mut ring = fixed_ring(Some("none"));
    ring["gap"] = json!(0);
    ring["justifyContent"] = json!("start");
    ring["alignItems"] = json!("start");
    let mut children = ring["children"]
        .take()
        .as_array()
        .cloned()
        .expect("ring children");
    let mut track = children.remove(0);
    let mut progress = children.remove(0);
    let mut centre = children.remove(0);
    track["x"] = json!(0);
    track["y"] = json!(0);
    progress["x"] = json!(2);
    progress["y"] = json!(2);
    centre["x"] = json!(20);
    centre["y"] = json!(38);
    ring["children"] = json!([centre, progress, track]);
    ring
}

fn find_id<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
    if let Some(values) = value.as_array() {
        return values.iter().find_map(|value| find_id(value, id));
    }
    if value.get("id").and_then(Value::as_str) == Some(id) {
        return Some(value);
    }
    value
        .get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|child| find_id(child, id))
}

#[test]
fn self_check_flags_direct_radial_arcs_in_explicit_or_default_flow() {
    for layout in [None, Some("horizontal"), Some("vertical")] {
        let forest = json!([fixed_ring(layout)]);
        let report = check_value_forest(&forest, 390.0);
        assert!(
            has_radial_issue(&report),
            "layout {layout:?} must be rejected: {report:?}"
        );
    }

    let mut positioned = fixed_ring(Some("horizontal"));
    for child in positioned["children"]
        .as_array_mut()
        .expect("ring children")
        .iter_mut()
        .take(2)
    {
        child["x"] = json!(12);
        child["y"] = json!(12);
    }
    let report = check_value_forest(&json!([positioned]), 390.0);
    assert!(
        has_radial_issue(&report),
        "authored arc x/y does not make a flex stack safe: {report:?}"
    );
}

#[test]
fn radial_rejection_message_names_the_wrapper_and_arc_mistakes_separately() {
    // Mistake 1: the wrapper is still in flex flow. The model must be told
    // to overlay it, not to guess at "explicit concentric coordinates".
    let flow = fixed_ring(Some("vertical"));
    let report = check_value_forest(&json!([flow]), 390.0);
    let message = report
        .issues
        .iter()
        .find(|issue| issue.code == "radial-stack-not-concentric")
        .expect("flow wrapper must be rejected")
        .message
        .clone();
    assert!(
        message.contains("set the wrapper layout to none"),
        "flow-wrapper mistake must carry the overlay instruction: {message}"
    );

    // Mistake 2: the wrapper is already `layout:none` and the arcs carry
    // explicit coordinates, but one arc does not share the wrapper's
    // centre. The instruction must target the arc geometry, and must not
    // waste the retry on a wrapper that is already correct.
    let mut off_centre = fixed_ring(Some("none"));
    off_centre["gap"] = json!(0);
    off_centre["justifyContent"] = json!("start");
    off_centre["alignItems"] = json!("start");
    let mut children = off_centre["children"]
        .take()
        .as_array()
        .cloned()
        .expect("children");
    let mut track = children.remove(0);
    let mut progress = children.remove(0);
    let mut centre = children.remove(0);
    track["x"] = json!(0);
    track["y"] = json!(0);
    progress["x"] = json!(10);
    progress["y"] = json!(2);
    centre["x"] = json!(20);
    centre["y"] = json!(38);
    off_centre["children"] = json!([centre, progress, track]);
    let report = check_value_forest(&json!([off_centre]), 390.0);
    assert!(
        has_radial_issue(&report),
        "off-centre arc must stay rejected: {report:?}"
    );
    let message = report
        .issues
        .iter()
        .find(|issue| issue.code == "radial-stack-not-concentric")
        .expect("off-centre arc must be rejected")
        .message
        .clone();
    assert!(
        message.contains(
            "give every arc the same width/height and x=(wrapper-w)/2, \
             y=(wrapper-h)/2 in a fixed square layout:none wrapper"
        ),
        "arc mistake must carry the concentric-arc instruction: {message}"
    );
    assert!(
        !message.contains("set the wrapper layout to none"),
        "the wrapper is already none — that instruction would mislead: {message}"
    );
}

#[test]
fn self_check_accepts_canonical_stack_and_ignores_unrelated_ellipse_shapes() {
    let forest = json!([
        canonical_fixed_ring(),
        {
            "type": "frame",
            "id": "plain-circles",
            "layout": "horizontal",
            "children": [
                {"type": "ellipse", "id": "plain-1", "width": 32, "height": 32},
                {"type": "ellipse", "id": "plain-2", "width": 32, "height": 32}
            ]
        },
        {
            "type": "frame",
            "id": "separate-cards",
            "layout": "horizontal",
            "children": [
                {"type": "frame", "id": "card-1", "children": [visible_arc("arc-1", 40.0, 180.0)]},
                {"type": "frame", "id": "card-2", "children": [visible_arc("arc-2", 40.0, 180.0)]}
            ]
        }
    ]);

    let report = check_value_forest(&forest, 390.0);
    assert!(
        !has_radial_issue(&report),
        "non-flow or low-confidence circles must pass: {report:?}"
    );
}

#[test]
fn auto_fix_overlays_safe_nested_authored_ring_before_insert() {
    let mut authored_ring = fixed_ring(Some("horizontal"));
    for child in authored_ring["children"]
        .as_array_mut()
        .expect("ring children")
        .iter_mut()
        .take(2)
    {
        child["x"] = json!(12);
        child["y"] = json!(12);
    }
    let mut nodes: Vec<PenNode> = serde_json::from_value(json!([{
        "type": "frame",
        "id": "section",
        "name": "Activity Section",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "children": [authored_ring]
    }]))
    .expect("valid authored forest");

    let before = check_generated_nodes(&nodes, 390.0);
    assert!(has_radial_issue(&before), "precondition: {before:?}");

    assert!(
        auto_fix_fixable_issues(&mut nodes, 390.0),
        "safe authored ring must be auto-fixed"
    );

    let repaired = serde_json::to_value(&nodes).expect("serialize repaired forest");
    let ring = find_id(&repaired, "ring").expect("nested ring");
    assert_eq!(ring["layout"], json!("none"));
    assert_eq!(ring["gap"].as_f64(), Some(0.0));
    let track = find_id(ring, "track").expect("track");
    assert_eq!(
        (track["x"].as_f64(), track["y"].as_f64()),
        (Some(0.0), Some(0.0))
    );
    let progress = find_id(ring, "progress").expect("progress");
    assert_eq!(
        (progress["x"].as_f64(), progress["y"].as_f64()),
        (Some(2.0), Some(2.0))
    );
    let centre = find_id(ring, "centre").expect("centre");
    assert_eq!(
        (centre["x"].as_f64(), centre["y"].as_f64()),
        (Some(20.0), Some(38.0))
    );
    let order: Vec<&str> = ring["children"]
        .as_array()
        .expect("repaired children")
        .iter()
        .filter_map(|child| child.get("id").and_then(Value::as_str))
        .collect();
    assert_eq!(order, ["centre", "progress", "track"]);

    let after = check_generated_nodes(&nodes, 390.0);
    assert!(
        !after.has_fatal(),
        "repaired ring must pass pre-insert self-check: {after:?}"
    );
}

#[test]
fn non_square_wrapper_and_padded_wrapper_are_now_centred_instead_of_retried() {
    // Concentricity is a pure geometric contract: a wrapper's aspect ratio
    // and its padding value used to make the strict tier-1 patch decline
    // outright (it also resizes/reorders, so it wants a confident square
    // ring before touching anything) — but neither actually prevents
    // computing a correct centred position, so the lenient tier-2 pass
    // (radial_repair_force_center) must now salvage both instead of
    // leaving the subtask to retry.
    // (wrapper mutation, expected track x/y, expected progress x/y) — track
    // is the 120x120 arc, progress the 116x116 arc (`fixed_ring`), so a
    // correct centred fix does NOT put them at the same point; it puts
    // each at the centre of its own size against the wrapper's full box.
    type Point = (f64, f64);
    type Case = (fn(&mut Value), Point, Point);
    let cases: [Case; 2] = [
        (|ring| ring["width"] = json!(240), (60.0, 0.0), (62.0, 2.0)),
        (|ring| ring["padding"] = json!(8), (0.0, 0.0), (2.0, 2.0)),
    ];

    for (index, (mutate, expected_track, expected_progress)) in cases.into_iter().enumerate() {
        let mut ring = fixed_ring(Some("horizontal"));
        mutate(&mut ring);
        let mut nodes: Vec<PenNode> = serde_json::from_value(json!([ring])).expect("valid forest");
        let before = check_generated_nodes(&nodes, 390.0);
        assert!(
            has_radial_issue(&before),
            "case {index} precondition: {before:?}"
        );

        assert!(
            auto_fix_fixable_issues(&mut nodes, 390.0),
            "case {index} must be salvaged, not retried"
        );

        let repaired = serde_json::to_value(&nodes).expect("serialize repaired forest");
        let ring = find_id(&repaired, "ring").expect("ring");
        assert_eq!(ring["layout"], json!("none"), "case {index}");
        let track = find_id(ring, "track").expect("track");
        let progress = find_id(ring, "progress").expect("progress");
        assert_eq!(
            (track["x"].as_f64(), track["y"].as_f64()),
            (Some(expected_track.0), Some(expected_track.1)),
            "case {index}: track"
        );
        assert_eq!(
            (progress["x"].as_f64(), progress["y"].as_f64()),
            (Some(expected_progress.0), Some(expected_progress.1)),
            "case {index}: progress"
        );

        let after = check_generated_nodes(&nodes, 390.0);
        assert!(
            !after.has_fatal(),
            "case {index} must pass self-check after the lenient fix: {after:?}"
        );
    }
}

#[test]
fn explicit_but_unrepairable_radial_shapes_are_rejected_without_guessing() {
    let cases = [{
        let mut ring = fixed_ring(Some("horizontal"));
        ring["children"][1]["width"] = json!(60);
        ring["children"][1]["height"] = json!(60);
        ring
    }];

    for (index, ring) in cases.into_iter().enumerate() {
        let mut nodes: Vec<PenNode> =
            serde_json::from_value(json!([ring])).expect("valid unsafe forest");
        let before = check_generated_nodes(&nodes, 390.0);
        assert!(
            has_radial_issue(&before),
            "case {index} must retry: {before:?}"
        );
        assert!(
            !auto_fix_fixable_issues(&mut nodes, 390.0),
            "case {index} must not be guessed"
        );
        let after = check_generated_nodes(&nodes, 390.0);
        assert!(
            has_radial_issue(&after),
            "case {index} must remain fatal: {after:?}"
        );
    }
}

#[test]
fn high_confidence_but_unsafe_stacks_remain_fatal_for_retry() {
    let cases = [
        {
            let mut ring = fixed_ring(Some("horizontal"));
            ring["children"][2] = json!({
                "type": "image",
                "id": "unmeasured-centre",
                "src": ""
            });
            ring
        },
        {
            let mut ring = fixed_ring(Some("horizontal"));
            ring["children"][2]["width"] = json!(160);
            ring
        },
    ];

    for (index, ring) in cases.into_iter().enumerate() {
        let mut nodes: Vec<PenNode> =
            serde_json::from_value(json!([ring])).expect("valid unsafe forest");
        let before = check_generated_nodes(&nodes, 390.0);
        assert!(
            has_radial_issue(&before),
            "case {index} must be fatal before repair: {before:?}"
        );
        assert!(
            !auto_fix_fixable_issues(&mut nodes, 390.0),
            "case {index} must not be guessed"
        );
        let after = check_generated_nodes(&nodes, 390.0);
        assert!(
            has_radial_issue(&after),
            "case {index} must remain fatal: {after:?}"
        );
    }
}

#[test]
fn layout_none_with_missing_coordinates_or_wrong_painter_order_is_fixed_preinsert() {
    let mut nodes: Vec<PenNode> =
        serde_json::from_value(json!([fixed_ring(Some("none"))])).expect("valid ring");
    assert!(has_radial_issue(&check_generated_nodes(&nodes, 390.0)));

    assert!(auto_fix_fixable_issues(&mut nodes, 390.0));

    let repaired = serde_json::to_value(&nodes).expect("serialize repaired ring");
    let ring = find_id(&repaired, "ring").expect("ring");
    let order: Vec<&str> = ring["children"]
        .as_array()
        .expect("ring children")
        .iter()
        .filter_map(|child| child.get("id").and_then(Value::as_str))
        .collect();
    assert_eq!(order, ["centre", "progress", "track"]);
    assert!(!check_generated_nodes(&nodes, 390.0).has_fatal());
}

#[test]
fn unnamed_partial_arc_pairs_infer_larger_sweep_as_track_and_reorder_preinsert() {
    for progress_sweep in [200, 185] {
        let mut nodes: Vec<PenNode> = serde_json::from_value(json!([{
            "type": "frame",
            "id": format!("ring-{progress_sweep}"),
            "width": 64,
            "height": 64,
            "layout": "none",
            "children": [
                {"type":"ellipse","id":format!("large-{progress_sweep}"),
                 "x":0,"y":0,"width":64,"height":64,"innerRadius":0.72,
                 "startAngle":135,"sweepAngle":270},
                {"type":"ellipse","id":format!("small-{progress_sweep}"),
                 "x":0,"y":0,"width":64,"height":64,"innerRadius":0.72,
                 "startAngle":135,"sweepAngle":progress_sweep},
                {"type":"frame","id":format!("centre-{progress_sweep}"),
                 "x":0,"y":0,"width":64,"height":64,"layout":"horizontal"}
            ]
        }]))
        .expect("valid unnamed partial ring");

        assert!(has_radial_issue(&check_generated_nodes(&nodes, 390.0)));
        assert!(auto_fix_fixable_issues(&mut nodes, 390.0));

        let repaired = serde_json::to_value(&nodes).expect("serialize repaired ring");
        let ring = find_id(&repaired, &format!("ring-{progress_sweep}")).expect("ring");
        let order: Vec<&str> = ring["children"]
            .as_array()
            .expect("ring children")
            .iter()
            .filter_map(|child| child.get("id").and_then(Value::as_str))
            .collect();
        assert_eq!(
            order,
            [
                format!("centre-{progress_sweep}"),
                format!("small-{progress_sweep}"),
                format!("large-{progress_sweep}"),
            ],
            "{progress_sweep}° progress must paint above the inferred 270° track"
        );
        assert!(!check_generated_nodes(&nodes, 390.0).has_fatal());
    }
}

#[test]
fn unmeasurable_nested_centre_content_is_retried_instead_of_clipped() {
    let mut ring = fixed_ring(Some("horizontal"));
    ring["children"][2] = json!({
        "type": "frame",
        "id": "centre",
        "layout": "horizontal",
        "padding": [6, 10],
        "children": [
            {"type": "text", "id": "value", "content": "82%"},
            {"type": "image", "id": "unknown-icon", "src": ""}
        ]
    });
    let mut nodes: Vec<PenNode> = serde_json::from_value(json!([ring])).expect("valid ring");

    assert!(has_radial_issue(&check_generated_nodes(&nodes, 390.0)));
    assert!(!auto_fix_fixable_issues(&mut nodes, 390.0));
}

#[test]
fn multiple_direct_centre_labels_are_grouped_before_insert() {
    let mut ring = fixed_ring(Some("horizontal"));
    let track = ring["children"][0].clone();
    let progress = ring["children"][1].clone();
    ring["children"] = json!([
        track,
        progress,
        {"type": "text", "id": "value", "content": "8,432", "width": 64, "height": 24},
        {"type": "text", "id": "unit", "content": "steps", "width": 40, "height": 18}
    ]);
    let mut nodes: Vec<PenNode> = serde_json::from_value(json!([ring])).expect("valid ring");

    assert!(has_radial_issue(&check_generated_nodes(&nodes, 390.0)));
    assert!(auto_fix_fixable_issues(&mut nodes, 390.0));
    let repaired = serde_json::to_value(&nodes).expect("serialize repaired ring");
    let ring = find_id(&repaired, "ring").expect("ring");
    let order: Vec<&str> = ring["children"]
        .as_array()
        .expect("ring children")
        .iter()
        .filter_map(|child| child.get("id").and_then(Value::as_str))
        .collect();
    assert_eq!(order, ["ring__radial-centre", "progress", "track"]);
    assert!(!check_generated_nodes(&nodes, 390.0).has_fatal());
}

#[test]
fn missing_centre_layout_is_measured_as_the_jian_default_row() {
    let mut ring = fixed_ring(Some("horizontal"));
    ring["children"][2] = json!({
        "type": "frame",
        "id": "centre",
        "children": [
            {"type": "text", "id": "value", "content": "82"},
            {"type": "text", "id": "unit", "content": "%"}
        ]
    });
    let mut nodes: Vec<PenNode> = serde_json::from_value(json!([ring])).expect("valid ring");

    assert!(auto_fix_fixable_issues(&mut nodes, 390.0));

    let repaired = serde_json::to_value(&nodes).expect("serialize repaired ring");
    let centre = find_id(&repaired, "centre").expect("centre");
    let width = centre["width"].as_f64().expect("measured width");
    let height = centre["height"].as_f64().expect("measured height");
    assert!(
        width > height,
        "default row measurement must be wider than tall"
    );
    let after = check_generated_nodes(&nodes, 390.0);
    assert!(
        !after.has_fatal(),
        "repaired default-row centre must pass self-check: {after:?}; repaired={repaired}"
    );
}

#[test]
fn semantic_names_preserve_full_progress_and_stroked_track_detection() {
    let mut full = fixed_ring(Some("horizontal"));
    full["children"][1]["sweepAngle"] = json!(360);
    let mut full_nodes: Vec<PenNode> =
        serde_json::from_value(json!([full])).expect("valid full ring");
    assert!(has_radial_issue(&check_generated_nodes(&full_nodes, 390.0)));
    assert!(auto_fix_fixable_issues(&mut full_nodes, 390.0));

    let mut stroked = fixed_ring(Some("horizontal"));
    let track = stroked["children"][0]
        .as_object_mut()
        .expect("track object");
    track.remove("innerRadius");
    track.remove("sweepAngle");
    track.insert(
        "stroke".into(),
        json!({"thickness": 10, "fill": [{"type": "solid", "color": "#123827"}]}),
    );
    let mut stroked_nodes: Vec<PenNode> =
        serde_json::from_value(json!([stroked])).expect("valid stroked ring");
    assert!(has_radial_issue(&check_generated_nodes(
        &stroked_nodes,
        390.0
    )));
    assert!(auto_fix_fixable_issues(&mut stroked_nodes, 390.0));
}

#[test]
fn pure_repair_walks_object_subtrees_without_editor_state() {
    let mut root = json!({
        "type": "frame",
        "id": "root",
        "layout": "vertical",
        "children": [{
            "type": "frame",
            "id": "nested",
            "layout": "vertical",
            "children": [fixed_ring(None)]
        }]
    });

    assert!(crate::radial_repair::repair_authored_radial_stacks(
        &mut root
    ));
    assert_eq!(
        find_id(&root, "ring").expect("deep ring")["layout"],
        json!("none")
    );
    assert_eq!(root["layout"], json!("vertical"));
}

/// One "Progress Ring" wrapper reconstructed verbatim from the real
/// `savings-goals` subtask script recovered from `0717-6-cc.log` (attempt
/// 2's full script text — attempt 1's own raw nodes weren't logged, only
/// the fatal issue codes for 3 rings at n14/n27/n40, and attempt 2 never
/// reached self-check because of an unrelated JS syntax error). Same
/// `ringSize`, `innerRadius`, `startAngle`, and `Ring Center` box the model
/// actually authored for each goal card. The one addition — `padding: 4`
/// on the ring wrapper — is the plausible source of attempt 1's rejection:
/// this exact structure without it already round-trips through the
/// existing strict tier-1 patch (see
/// `auto_fix_overlays_safe_nested_authored_ring_before_insert` above), so
/// something about attempt 1's copy must have tripped one of tier 1's
/// confidence gates for the fix to have failed twice in a row; padding is
/// the most representative example both because this same script pads
/// every card (`padding: 20`) and because it's exactly the gate this task
/// converts to tier 2.
fn savings_goal_ring(suffix: &str, pct: u32) -> Value {
    json!({
        "type": "frame",
        "id": format!("ring-{suffix}"),
        "name": "Progress Ring",
        "width": 56,
        "height": 56,
        "layout": "none",
        "padding": 4,
        "children": [
            {"type": "ellipse", "id": format!("track-{suffix}"), "name": "Ring Track",
             "width": 56, "height": 56, "innerRadius": 0.82,
             "fill": [{"type": "solid", "color": "#F1F5F9"}]},
            {"type": "ellipse", "id": format!("progress-{suffix}"), "name": "Ring Progress",
             "width": 56, "height": 56, "innerRadius": 0.82, "startAngle": -90,
             "sweepAngle": (360 * pct / 100),
             "fill": [{"type": "solid", "color": "#0D9488"}]},
            {"type": "frame", "id": format!("centre-{suffix}"), "name": "Ring Center",
             "width": 40, "height": 20, "layout": "horizontal",
             "alignItems": "center", "justifyContent": "center",
             "children": [
                {"type": "text", "id": format!("pct-{suffix}"),
                 "content": format!("{pct}%"), "width": "fit_content", "textGrowth": "auto"}
             ]}
        ]
    })
}

#[test]
fn savings_goal_rings_from_the_0717_6_cc_incident_are_salvaged_by_auto_fix() {
    let rail = json!({
        "type": "frame",
        "id": "goals-rail",
        "name": "Savings Goals Rail",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "gap": 12,
        "children": [
            savings_goal_ring("emergency-fund", 72),
            savings_goal_ring("new-car", 45),
            savings_goal_ring("vacation", 88),
        ]
    });
    let mut nodes: Vec<PenNode> = serde_json::from_value(json!([rail])).expect("valid rail");

    let before = check_generated_nodes(&nodes, 390.0);
    assert!(
        has_radial_issue(&before),
        "precondition: must reproduce the fatal rejection: {before:?}"
    );

    assert!(
        auto_fix_fixable_issues(&mut nodes, 390.0),
        "the real savings-goals rings must be salvaged on attempt 1, not \
         fall through to a minimal-skills salvage rung"
    );

    let repaired = serde_json::to_value(&nodes).expect("serialize repaired rail");
    for suffix in ["emergency-fund", "new-car", "vacation"] {
        let ring = find_id(&repaired, &format!("ring-{suffix}")).expect("ring");
        let track = find_id(ring, &format!("track-{suffix}")).expect("track");
        let progress = find_id(ring, &format!("progress-{suffix}")).expect("progress");
        assert_eq!(
            (track["x"].as_f64(), track["y"].as_f64()),
            (Some(0.0), Some(0.0)),
            "ring {suffix}: 56x56 track centred in the 56x56 wrapper sits at the origin"
        );
        assert_eq!(
            (track["x"].as_f64(), track["y"].as_f64()),
            (progress["x"].as_f64(), progress["y"].as_f64()),
            "ring {suffix}: track and progress are the same size, so they share one point"
        );
        let centre = find_id(ring, &format!("centre-{suffix}")).expect("centre");
        assert_eq!(
            (centre["x"].as_f64(), centre["y"].as_f64()),
            (Some(8.0), Some(18.0)),
            "ring {suffix}: the 40x20 Ring Center the model authored centres at (8, 18)"
        );
    }

    let after = check_generated_nodes(&nodes, 390.0);
    assert!(
        !after.has_fatal(),
        "salvaged rail must pass self-check on the same attempt: {after:?}"
    );
}
