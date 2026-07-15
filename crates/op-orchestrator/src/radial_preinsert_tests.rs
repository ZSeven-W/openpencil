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
fn explicit_but_unrepairable_radial_shapes_are_rejected_without_guessing() {
    let cases = [
        {
            let mut ring = fixed_ring(Some("horizontal"));
            ring["width"] = json!("fill_container");
            ring
        },
        {
            let mut ring = fixed_ring(Some("horizontal"));
            ring["width"] = json!(240);
            ring
        },
        {
            let mut ring = fixed_ring(Some("horizontal"));
            ring["children"][1]["width"] = json!(60);
            ring["children"][1]["height"] = json!(60);
            ring
        },
    ];

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
            ring["padding"] = json!(8);
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
fn multiple_direct_centre_labels_are_fatal_instead_of_escaping_detection() {
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
    assert!(!auto_fix_fixable_issues(&mut nodes, 390.0));
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
