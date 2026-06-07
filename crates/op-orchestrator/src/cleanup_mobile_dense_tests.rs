use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use serde_json::json;
use serde_json::Value;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn icon(id: &str, size: i32) -> Value {
    json!({
        "type": "icon_font",
        "id": id,
        "iconFontName": "home",
        "width": size,
        "height": size
    })
}

fn label(id: &str, content: &str) -> Value {
    json!({"type": "text", "id": id, "content": content})
}

fn tab(id: &str, icon_id: &str, label_id: &str, content: &str) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "role": "button",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "vertical",
        "padding": [12, 24],
        "children": [icon(icon_id, 22), label(label_id, content)]
    })
}

#[test]
fn cleanup_compacts_dense_mobile_tab_rows() {
    let mut sink = VecDocSink::new();
    let row = json!({
        "type": "frame",
        "id": "tab-row",
        "name": "Tab Bar",
        "role": "navbar",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "padding": [12, 28],
        "children": [
            tab("home", "home-icon", "home-label", "HOME"),
            tab("stats", "stats-icon", "stats-label", "STATS"),
            {
                "type": "frame",
                "id": "add",
                "role": "button",
                "width": 52,
                "height": 52,
                "padding": [12, 24],
                "children": [icon("add-icon", 24)]
            },
            tab("habits", "habits-icon", "habits-label", "HABITS"),
            tab("profile", "profile-icon", "profile-label", "PROFILE")
        ]
    });
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Habit Tracker",
        "x": 80,
        "y": 40,
        "width": 390,
        "height": 844,
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "bottom-tabs",
                "name": "Bottom Tab Bar",
                "role": "section",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "children": [row]
            }
        ]
    }))
    .expect("mobile root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeLayoutProp {
                property,
                value: op_editor_core::LayoutPropValue::NumberArray(values),
                ..
            } if property == "padding"
                && values == &vec![8.0, 8.0, 8.0, 8.0]
        )),
        "dense mobile tab rows should reduce oversized tab padding before layout"
    );
    assert!(
        sink.applied
            .iter()
            .filter(|c| matches!(
                c,
                EditorCommand::SetNodeLayoutProp {
                    property,
                    value: op_editor_core::LayoutPropValue::Keyword(value),
                    ..
                } if property == "width" && value == "fill_container"
            ))
            .count()
            >= 4,
        "dense mobile tab items should become equal-width children so the last label stays inside"
    );
}

#[test]
fn cleanup_compacts_weekly_progress_rows() {
    let mut sink = VecDocSink::new();
    let day = |id: &str, label_id: &str, label: &str| {
        json!({
            "type": "frame",
            "id": id,
            "name": format!("{label} Indicator"),
            "role": "stat-card",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "gap": 6,
            "padding": [24, 24],
            "children": [
                {"type": "frame", "id": format!("{id}-dot"), "width": 24, "height": 24, "children": [icon(&format!("{id}-icon"), 14)]},
                {"type": "text", "id": label_id, "role": "caption", "content": label}
            ]
        })
    };
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Habit Tracker",
        "width": 390,
        "height": 844,
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "streak-section",
                "role": "section",
                "width": "fill_container",
                "height": "fit_content",
                "padding": [0, 24],
                "children": [
                    {
                        "type": "frame",
                        "id": "week-row",
                        "name": "Seven Day Progress Indicators",
                        "role": "row",
                        "width": "fill_container",
                        "height": "fit_content",
                        "layout": "horizontal",
                        "gap": 8,
                        "children": [
                            day("monday", "m-label", "M"),
                            day("tuesday", "t-label", "T"),
                            day("wednesday", "w-label", "W"),
                            day("thursday", "th-label", "T"),
                            day("friday", "f-label", "F"),
                            day("saturday", "s-label", "S"),
                            day("sunday", "su-label", "S")
                        ]
                    }
                ]
            }
        ]
    }))
    .expect("weekly mobile root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeLayoutProp {
                property,
                value: op_editor_core::LayoutPropValue::Number(value),
                ..
            } if property == "gap" && (*value - 4.0).abs() < f64::EPSILON
        )),
        "weekly progress rows should tighten gaps before layout"
    );
    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeLayoutProp {
                property,
                value: op_editor_core::LayoutPropValue::NumberArray(values),
                ..
            } if property == "padding"
                && values == &vec![4.0, 0.0, 4.0, 0.0]
        )),
        "weekly progress day cells should remove oversized horizontal padding"
    );
}
