//! Unit tests for the stacked overlapping-text repair predicate and its
//! diagnostic twin. The app-01 fixture is copied verbatim from
//! ring-fix-proof-0921/app-01 (`本次时长卡-数字窗`), the app-05 one from the
//! `Calorie Count Step` stack.

use super::*;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};

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

fn root_id(sink: &VecDocSink) -> String {
    sink.state().active_children()[0].id_str().to_string()
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

/// The ring-fix-proof-0921/app-01 hit, verbatim: a `layout:none` window
/// whose `0` start text sits at the same x/y as the `12` end text.
fn app01_duration_card() -> Value {
    json!({
        "type": "frame", "id": "nroot", "name": "Root", "width": 375, "height": 812,
        "layout": "vertical", "children": [
            {
                "type": "frame", "id": "n486", "name": "本次时长卡-数字行",
                "width": "fill_container", "height": "fit_content",
                "layout": "horizontal", "gap": 8.0, "alignItems": "end", "children": [
                    {
                        "type": "frame", "id": "n487", "name": "本次时长卡-数字窗",
                        "width": 64.0, "height": 52.0, "layout": "none", "clipContent": true,
                        "children": [
                            {
                                "type": "text", "id": "n488", "name": "本次时长卡-数字-起始",
                                "x": 0.0, "y": 2.0,
                                "animations": [
                                    {
                                        "trigger": "mount",
                                        "keyframes": [
                                            { "offset": 0.0, "values": { "translateY": 0 } },
                                            { "offset": 0.35, "values": { "translateY": 0 } },
                                            { "offset": 1.0, "values": { "translateY": 64 } }
                                        ],
                                        "durationMs": 1100, "delayMs": 260,
                                        "easing": "emphasizedDecelerate"
                                    }
                                ],
                                "width": 64.0, "height": "fit_content", "content": "0",
                                "fontFamily": "Inter", "fontSize": 48.0, "fontWeight": 600,
                                "letterSpacing": -0.5, "lineHeight": 1.0, "textAlign": "center",
                                "textGrowth": "fixed-width",
                                "fill": [{ "type": "solid", "color": "$--foreground" }]
                            },
                            {
                                "type": "text", "id": "n489", "name": "本次时长卡-数字",
                                "x": 0.0, "y": 2.0,
                                "animations": [
                                    {
                                        "trigger": "mount",
                                        "keyframes": [
                                            { "offset": 0.0, "values": { "translateY": -64 } },
                                            { "offset": 0.35, "values": { "translateY": -64 } },
                                            { "offset": 1.0, "values": { "translateY": 0 } }
                                        ],
                                        "durationMs": 1100, "delayMs": 260,
                                        "easing": "emphasizedDecelerate"
                                    }
                                ],
                                "width": 64.0, "height": "fit_content", "content": "12",
                                "fontFamily": "Inter", "fontSize": 48.0, "fontWeight": 600,
                                "letterSpacing": -0.5, "lineHeight": 1.0, "textAlign": "center",
                                "textGrowth": "fixed-width",
                                "fill": [{ "type": "solid", "color": "$--foreground" }]
                            }
                        ]
                    },
                    {
                        "type": "text", "id": "n490", "name": "本次时长卡-单位",
                        "width": "fit_content", "height": "fit_content", "content": "分钟",
                        "fontFamily": "Inter", "fontSize": 14.0, "fontWeight": 500
                    }
                ]
            }
        ]
    })
}

fn counter_step(name: &str, content: &str) -> Value {
    json!({
        "type": "text", "id": format!("step-{}", name), "name": name,
        "x": 0.0, "y": 0.0,
        "width": "fit_content", "height": "fit_content", "content": content,
        "fontFamily": "Space Grotesk", "fontSize": 48.0, "fontWeight": 800,
        "letterSpacing": -2.0, "lineHeight": 1.0, "textGrowth": "auto",
        "fill": [{ "type": "solid", "color": "$--color-error-foreground" }]
    })
}

/// The app-01 hit: the `0` start node is hidden, the `12` end node keeps its
/// authored state (no opacity touch), coordinates untouched, edits applied.
#[test]
fn app01_duration_window_hides_start_keeps_end() {
    let mut sink = insert_root(app01_duration_card());

    let rid = root_id(&sink);
    let changed = repair_stacked_overlapping_texts(&mut sink, &rid);

    assert!(changed, "the pass must report edits for the app-01 window");
    let root = active_root_json(&sink);
    let start = find_by_name(&root, "本次时长卡-数字-起始").expect("start exists");
    assert_eq!(start["opacity"], json!(0.0), "start value must be hidden");
    assert_eq!(start["x"], json!(0.0));
    assert_eq!(start["y"], json!(2.0));
    let end = find_by_name(&root, "本次时长卡-数字").expect("end exists");
    assert_eq!(end["content"], json!("12"));
    assert!(
        end.get("opacity").is_none(),
        "the kept end value must not gain an opacity edit"
    );
    assert_eq!(end["x"], json!(0.0));
    assert_eq!(end["y"], json!(2.0));
}

/// The app-05 hit: a three-step counter hides the first two steps and keeps
/// Step 2 (the last frame, i.e. the counter's resting value).
#[test]
fn app05_three_step_counter_hides_all_but_last() {
    let mut sink = insert_root(json!({
        "type": "frame", "id": "nroot", "name": "Root", "width": 375, "height": 812,
        "layout": "vertical", "children": [
            {
                "type": "frame", "id": "stack", "name": "Calorie Counter Stack",
                "width": "fill_container", "height": 52.0, "layout": "none", "children": [
                    counter_step("Calorie Count Step 0", "0"),
                    counter_step("Calorie Count Step 1", "128"),
                    counter_step("Calorie Count Step 2", "264")
                ]
            }
        ]
    }));

    let rid = root_id(&sink);
    let changed = repair_stacked_overlapping_texts(&mut sink, &rid);

    assert!(changed);
    let root = active_root_json(&sink);
    for name in ["Calorie Count Step 0", "Calorie Count Step 1"] {
        let step = find_by_name(&root, name).unwrap_or_else(|| panic!("{name} exists"));
        assert_eq!(step["opacity"], json!(0.0), "{name} must be hidden");
    }
    let last = find_by_name(&root, "Calorie Count Step 2").expect("last step exists");
    assert_eq!(last["content"], json!("264"));
    assert!(
        last.get("opacity").is_none(),
        "the last step is the resting value and must stay untouched"
    );
}

/// A `layout:none` window with a single text is a plain positioned label —
/// nothing overlaps, nothing to do.
#[test]
fn single_text_none_window_is_not_touched() {
    let mut sink = insert_root(json!({
        "type": "frame", "id": "nroot", "name": "Root", "width": 375, "height": 200,
        "layout": "vertical", "children": [
            {
                "type": "frame", "id": "label-window", "name": "Label Window",
                "width": 64.0, "height": 52.0, "layout": "none", "children": [
                    {
                        "type": "text", "id": "label", "name": "Label",
                        "x": 0.0, "y": 2.0, "width": 64.0, "content": "12"
                    }
                ]
            }
        ]
    }));

    let rid = root_id(&sink);
    let changed = repair_stacked_overlapping_texts(&mut sink, &rid);

    assert!(!changed);
    let root = active_root_json(&sink);
    let label = find_by_name(&root, "Label").expect("label exists");
    assert!(label.get("opacity").is_none());
}

/// Two texts at different x positions inside a `layout:none` frame are two
/// genuinely separate labels, not a roll.
#[test]
fn two_texts_at_different_x_are_not_touched() {
    let mut sink = insert_root(json!({
        "type": "frame", "id": "nroot", "name": "Root", "width": 375, "height": 200,
        "layout": "vertical", "children": [
            {
                "type": "frame", "id": "row", "name": "Metric Row",
                "width": 200.0, "height": 52.0, "layout": "none", "children": [
                    {
                        "type": "text", "id": "value", "name": "Value",
                        "x": 0.0, "y": 2.0, "width": 64.0, "content": "12"
                    },
                    {
                        "type": "text", "id": "unit", "name": "Unit",
                        "x": 72.0, "y": 2.0, "width": 40.0, "content": "min"
                    }
                ]
            }
        ]
    }));

    let rid = root_id(&sink);
    let changed = repair_stacked_overlapping_texts(&mut sink, &rid);

    assert!(!changed);
    let root = active_root_json(&sink);
    for name in ["Value", "Unit"] {
        let node = find_by_name(&root, name).unwrap_or_else(|| panic!("{name} exists"));
        assert!(node.get("opacity").is_none(), "{name} must stay untouched");
    }
}

/// Two texts in a `layout: vertical` container are normal flow copy — no
/// authored x/y at all, so the same-position clause cannot hold.
#[test]
fn vertical_container_texts_are_not_touched() {
    let mut sink = insert_root(json!({
        "type": "frame", "id": "nroot", "name": "Root", "width": 375, "height": 200,
        "layout": "vertical", "children": [
            {
                "type": "frame", "id": "col", "name": "Metric Column",
                "width": 200.0, "height": "fit_content", "layout": "vertical", "gap": 4.0,
                "children": [
                    { "type": "text", "id": "value", "name": "Value", "content": "12" },
                    { "type": "text", "id": "unit", "name": "Unit", "content": "min" }
                ]
            }
        ]
    }));

    let rid = root_id(&sink);
    let changed = repair_stacked_overlapping_texts(&mut sink, &rid);

    assert!(!changed);
    let root = active_root_json(&sink);
    for name in ["Value", "Unit"] {
        let node = find_by_name(&root, name).unwrap_or_else(|| panic!("{name} exists"));
        assert!(node.get("opacity").is_none(), "{name} must stay untouched");
    }
}

/// A window whose start node already carries `opacity: 0` is already fixed —
/// the pass is idempotent and reports no edits.
#[test]
fn already_hidden_start_node_is_idempotent() {
    let mut doc = app01_duration_card();
    doc["children"][0]["children"][0]["children"][0]["opacity"] = json!(0.0);
    let mut sink = insert_root(doc);

    let rid = root_id(&sink);
    let changed = repair_stacked_overlapping_texts(&mut sink, &rid);

    assert!(!changed, "an already-hidden roll reports no edits");
    let root = active_root_json(&sink);
    let end = find_by_name(&root, "本次时长卡-数字").expect("end exists");
    assert!(
        end.get("opacity").is_none(),
        "the kept end value must stay untouched on a no-op run"
    );
}

/// The diagnostic mirrors the repair predicate, carries the
/// `stacked-overlapping-text` key and names the container.
#[test]
fn diagnostic_reports_the_window_with_its_name() {
    let sink = insert_root(app01_duration_card());

    let diagnostics = crate::geometry_validation::geometry_diagnostics(sink.state());

    assert!(
        diagnostics.iter().any(|line| {
            line.contains("stacked-overlapping-text") && line.contains("本次时长卡-数字窗")
        }),
        "diagnostics must mention the stacked window: {diagnostics:?}"
    );
}
