use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn evidence_root() -> Value {
    json!({
        "type": "frame",
        "id": "root",
        "name": "Page",
        "width": 375,
        "height": 844,
        "layout": "vertical",
        "gap": 16,
        "padding": 0,
        "children": [
            {
                "type": "frame", "id": "status", "name": "Status Bar",
                "role": "status-bar", "width": "fill_container", "height": 62
            },
            {
                "type": "frame", "id": "hero-section", "name": "Hero",
                "width": "fill_container", "height": "fit_content",
                "layout": "vertical", "gap": 12, "padding": [0, 24],
                "children": [
                    {
                        "type": "image", "id": "hero-image", "name": "Hero Image",
                        "x": 24, "width": 327, "height": 280,
                        "src": "https://example.com/hero.jpg"
                    },
                    {"type": "text", "id": "title", "name": "Title", "content": "Dawn"},
                    {
                        "type": "frame", "id": "meta", "name": "Meta Row",
                        "layout": "horizontal", "children": [
                            {"type": "text", "id": "meta-text", "content": "Today"}
                        ]
                    }
                ]
            }
        ]
    })
}

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 375.0,
            height: 844.0,
            layout: Some("vertical".into()),
            gap: Some(16.0),
            padding: Some(0.0),
            fill: None,
        },
        subtasks: vec![Subtask {
            id: "hero-task".into(),
            label: "Hero".into(),
            region: Region {
                width: 375.0,
                height: 280.0,
            },
            bleed_hero: true,
            id_prefix: "hero-task".into(),
            parent_frame_id: Some("root".into()),
            insert_after_sibling_id: None,
            elements: Some("ARCHETYPE: image-led".into()),
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
            retry_feedback: None,
        }],
        style_guide_name: None,
    }
}

fn sink_with(root: Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state.doc.children = vec![serde_json::from_value(root).expect("root fixture")];
    sink
}

fn root_value(sink: &VecDocSink) -> Value {
    serde_json::to_value(&sink.state.active_children()[0]).expect("root serializes")
}

fn hero_value(sink: &VecDocSink) -> Value {
    root_value(sink)["children"]
        .as_array()
        .expect("root children")
        .iter()
        .find(|child| child["name"] == "Hero (bleed)")
        .cloned()
        .expect("bleed hero section")
}

#[test]
fn evidence_shaped_hero_becomes_full_bleed_with_one_inset() {
    let mut sink = sink_with(evidence_root());

    assert_eq!(enforce(&mut sink, &plan(), "root"), 1);

    let hero = hero_value(&sink);
    assert_eq!(hero["padding"], json!([0.0, 0.0]));
    assert_eq!(hero["name"], "Hero (bleed)");
    assert_eq!(hero["children"][0]["id"], "hero-image");
    assert_eq!(hero["children"][0]["width"], "fill_container");
    assert_eq!(hero["children"][0]["x"], 0.0);
    assert_eq!(hero["children"].as_array().unwrap().len(), 2);

    let inset = &hero["children"][1];
    assert_eq!(inset["name"], "Hero inset");
    assert_eq!(inset["layout"], "vertical");
    assert_eq!(inset["gap"], 12.0);
    assert_eq!(inset["padding"], json!([0.0, 24.0]));
    assert_eq!(inset["width"], "fill_container");
    assert_eq!(inset["height"], "fit_content");
    assert_eq!(
        inset["children"]
            .as_array()
            .unwrap()
            .iter()
            .map(|child| child["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["title", "meta"]
    );
    assert_eq!(root_value(&sink)["children"][0]["id"], "status");
}

#[test]
fn hero_bleed_is_idempotent() {
    let mut sink = sink_with(evidence_root());
    let plan = plan();

    assert_eq!(enforce(&mut sink, &plan, "root"), 1);
    let after_first = root_value(&sink);
    assert_eq!(enforce(&mut sink, &plan, "root"), 0);
    assert_eq!(root_value(&sink), after_first);
}

#[test]
fn section_whose_first_non_status_child_is_text_is_untouched() {
    let mut root = evidence_root();
    root["children"][1]["children"] = json!([
        {"type": "text", "id": "first", "content": "Title"},
        {"type": "image", "id": "later", "width": 327, "height": 280,
         "src": "https://example.com/later.jpg"}
    ]);
    let mut sink = sink_with(root);
    let before = root_value(&sink);

    assert_eq!(enforce(&mut sink, &plan(), "root"), 0);
    assert_eq!(root_value(&sink), before);
}

#[test]
fn whole_cleanup_driver_keeps_the_bleed_section_flush() {
    let mut sink = sink_with(evidence_root());
    let mut summary = crate::repair_summary::RepairSummary::default();

    crate::cleanup::run_cleanup_passes_with_summary(&mut sink, &plan(), &["root"], &mut summary);

    let hero = hero_value(&sink);
    assert_eq!(hero["padding"][1], 0.0);
    assert_eq!(hero["padding"][3], Value::Null);
    assert_eq!(hero["children"][0]["width"], "fill_container");
    assert_eq!(hero["children"][1]["padding"], json!([0.0, 24.0]));
    assert!(summary
        .records()
        .iter()
        .any(|record| record.pass == "hero-bleed"));
}
