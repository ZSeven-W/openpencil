use super::radial_repair_sink::{radial_stack_repair, Rect};
use super::*;
use op_editor_core::{EditorCommand, EditorState};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn layout_none_repair_centres_against_resolved_child_size() {
    let ring = json!({
        "type":"frame","id":"ring","width":"fill_container","height":120,"layout":"none",
        "children":[
            {"type":"frame","id":"center","width":98,"height":43},
            {"type":"ellipse","id":"progress","width":120,"height":120,
             "innerRadius":0.86,"sweepAngle":264},
            {"type":"ellipse","id":"track","width":120,"height":120,"innerRadius":0.86}
        ]
    });
    let rects = HashMap::from([
        ("ring".to_string(), Rect { w: 287.0, h: 120.0 }),
        ("center".to_string(), Rect { w: 98.0, h: 47.0 }),
    ]);

    let commands = radial_stack_repair(&ring, &rects).expect("radial repair");
    let center_update = commands.iter().find_map(|command| match command {
        EditorCommand::UpdateNode {
            node_id,
            x,
            y,
            width,
            height,
            ..
        } if node_id.as_str() == "center" => Some((*x, *y, *width, *height)),
        _ => None,
    });

    assert_eq!(
        center_update,
        Some((Some(95), Some(37), None, None)),
        "position must use the final 47px layout height without rewriting the authored size"
    );
}

/// The motion50 app-01 "breath ring" shape: a fixed `layout:none` wrapper
/// whose direct children are TWO centre texts (the big countdown number
/// above, the remaining-rounds caption below), a partial progress arc, and
/// a full track. `arc_layer` recognises the arcs through their names, so
/// `radial_layers` classifies them as a progress+track pair while both
/// texts land in `centres` — the exact structure the retired
/// `centres.len() > 1` ambiguity check rejected four times in a row.
fn breath_ring(progress_x: f64, layout: &str) -> Value {
    json!({
        "type":"frame","id":"breath-ring","name":"呼吸圆环",
        "width":220,"height":220,"layout":layout,
        "gap":0,"justifyContent":"start","alignItems":"start",
        "children":[
            {"type":"text","id":"timer","name":"计时",
             "x":70,"y":80,"width":80,"height":40,"content":"05"},
            {"type":"text","id":"rounds","name":"剩余轮次",
             "x":60,"y":124,"width":100,"height":18,"content":"第 3 / 7 轮"},
            {"type":"ellipse","id":"progress","name":"Ring Progress",
             "x":progress_x,"y":0,"width":220,"height":220,
             "innerRadius":0.86,"startAngle":-90,"sweepAngle":264},
            {"type":"ellipse","id":"track","name":"Ring Track",
             "x":0,"y":0,"width":220,"height":220,"innerRadius":0.86}
        ]
    })
}

fn child_by_id<'a>(v: &'a Value, id: &str) -> &'a Value {
    v["children"]
        .as_array()
        .expect("children")
        .iter()
        .find(|child| child["id"] == id)
        .unwrap_or_else(|| panic!("missing child {id}"))
}

#[test]
fn multi_centre_breath_ring_is_not_ambiguous_centre_content() {
    // Red proof (pre-fix behaviour): this exact fixture was the app-01
    // false fatal — `is_authored_radial_stack_unsafe` returned true and
    // `repair_authored_radial_stacks` refused to touch it.
    let mut repaired = breath_ring(0.0, "none");
    repair_authored_radial_stacks(&mut repaired);
    assert!(
        !is_authored_radial_stack_unsafe(&repaired),
        "two centre texts with authored positions are an intended layout, \
         not an ambiguous painter order"
    );
    for (id, (x, y)) in [("timer", (70, 80)), ("rounds", (60, 124))] {
        assert_eq!(
            (
                child_by_id(&repaired, id)["x"].as_f64(),
                child_by_id(&repaired, id)["y"].as_f64()
            ),
            (Some(x as f64), Some(y as f64)),
            "centre text {id} must keep its authored position"
        );
    }
}

#[test]
fn breath_ring_with_off_centre_arc_is_pulled_back_without_moving_texts() {
    let mut node = breath_ring(10.0, "none");
    assert!(
        is_authored_radial_stack_unsafe(&node),
        "an arc that does not share the wrapper centre is genuinely unsafe"
    );
    assert!(repair_authored_radial_stacks(&mut node));
    assert_eq!(
        (
            child_by_id(&node, "progress")["x"].as_f64(),
            child_by_id(&node, "progress")["y"].as_f64()
        ),
        (Some(0.0), Some(0.0)),
        "the off-centre arc must be re-centred"
    );
    assert_eq!(child_by_id(&node, "timer")["x"].as_f64(), Some(70.0));
    assert_eq!(child_by_id(&node, "rounds")["y"].as_f64(), Some(124.0));
    assert!(!is_authored_radial_stack_unsafe(&node));
}

#[test]
fn breath_ring_in_a_flow_wrapper_is_converted_to_none_with_texts_untouched() {
    let mut node = breath_ring(0.0, "vertical");
    assert!(is_authored_radial_stack_unsafe(&node));
    assert!(repair_authored_radial_stacks(&mut node));
    assert_eq!(node["layout"], json!("none"), "wrapper must be overlaid");
    for id in ["progress", "track"] {
        assert_eq!(
            (
                child_by_id(&node, id)["x"].as_f64(),
                child_by_id(&node, id)["y"].as_f64()
            ),
            (Some(0.0), Some(0.0)),
            "arc {id} must be concentric with the wrapper box"
        );
    }
    assert_eq!(child_by_id(&node, "timer")["x"].as_f64(), Some(70.0));
    assert_eq!(child_by_id(&node, "rounds")["y"].as_f64(), Some(124.0));
    assert!(!is_authored_radial_stack_unsafe(&node));
}

#[test]
fn late_repair_reorders_unnamed_partial_pairs_to_canonical_painter_order() {
    for progress_sweep in [200, 185] {
        let ring_id = format!("ring-{progress_sweep}");
        let ring: jian_ops_schema::node::PenNode = serde_json::from_value(json!({
            "type":"frame","id":ring_id,"width":64,"height":64,"layout":"none",
            "children":[
                {"type":"ellipse","id":format!("large-{progress_sweep}"),
                 "x":0,"y":0,"width":64,"height":64,"innerRadius":0.72,
                 "startAngle":135,"sweepAngle":270},
                {"type":"ellipse","id":format!("small-{progress_sweep}"),
                 "x":0,"y":0,"width":64,"height":64,"innerRadius":0.72,
                 "startAngle":135,"sweepAngle":progress_sweep},
                {"type":"frame","id":format!("centre-{progress_sweep}"),
                 "x":0,"y":0,"width":64,"height":64,"layout":"horizontal"}
            ]
        }))
        .expect("valid unnamed partial ring");
        let mut state = EditorState::new();
        state.active_children_mut().clear();
        state.active_children_mut().push(ring);
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };

        assert!(repair_radial_stacks(&mut sink, &ring_id));

        let repaired = serde_json::to_value(&sink.state.active_children()[0])
            .expect("serialize repaired ring");
        let order: Vec<&str> = repaired["children"]
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
            ]
        );
    }
}
