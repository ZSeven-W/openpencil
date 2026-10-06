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
    sink.state.doc.variables = Some(
        [
            ("--foreground", "#0F172A"),
            ("--color-error-foreground", "#0F172A"),
        ]
        .into_iter()
        .map(|(name, color)| {
            (
                name.to_string(),
                serde_json::from_value(json!({
                    "type": "color",
                    "value": [{"value": color, "theme": {"Mode": "Light"}}]
                }))
                .expect("hero color variable"),
            )
        })
        .collect(),
    );
    sink.state.doc.themes = Some(
        [("Mode".to_string(), vec!["Light".to_string()])]
            .into_iter()
            .collect(),
    );
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
fn none_stack_finds_and_flushes_later_image_and_scrim() {
    let mut root = evidence_root();
    root["children"][1] = json!({
        "type": "frame", "id": "hero-section", "name": "Workout Hero",
        "width": "fill_container", "height": "fit_content",
        "layout": "vertical", "padding": [0, 24, 0, 24],
        "children": [{
            "type": "frame", "id": "hero-stack", "name": "Hero Media Stack",
            "layout": "none", "width": 327, "height": 284,
            "children": [
                {
                    "type": "frame", "id": "hero-content", "name": "Hero Content",
                    "x": 24, "width": 327,
                    "children": [{"type": "text", "id": "hero-title", "content": "Workout"}]
                },
                {
                    "type": "frame", "id": "back-button", "name": "Back Button",
                    "x": 16, "width": 44,
                    "children": [{"type": "text", "id": "back-label", "content": "Back"}]
                },
                {
                    "type": "rectangle", "id": "hero-scrim", "name": "Hero Scrim",
                    "x": 0, "width": 327,
                    "fill": [{"type": "linear_gradient", "stops": []}]
                },
                {
                    "type": "image", "id": "hero-image", "name": "Workout Hero Image",
                    "x": 0, "width": 327, "height": 284, "src": "workout.png"
                }
            ]
        }]
    });
    let mut sink = sink_with(root);
    let mut workout_plan = plan();
    workout_plan.subtasks[0].label = "Workout Hero".into();

    assert_eq!(enforce(&mut sink, &workout_plan, "root"), 1);

    let hero = root_value(&sink)["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["name"] == "Workout Hero (bleed)")
        .cloned()
        .expect("bleed hero section");
    assert_eq!(hero["name"], "Workout Hero (bleed)");
    assert_eq!(hero["padding"], json!([0.0, 0.0, 0.0, 0.0]));
    let stack = &hero["children"][0];
    assert_eq!(stack["width"], "fill_container");
    assert_eq!(stack["children"][0]["x"], 24.0);
    assert_eq!(stack["children"][0]["width"], 327.0);
    assert_eq!(stack["children"][2]["width"], "fill_container");
    assert_eq!(stack["children"][2]["x"], 0.0);
    assert_eq!(stack["children"][3]["width"], "fill_container");
    assert_eq!(stack["children"][3]["x"], 0.0);
}

#[test]
fn none_stack_without_media_is_untouched() {
    let mut root = evidence_root();
    root["children"][1]["children"] = json!([{
        "type": "frame", "id": "hero-stack", "layout": "none", "width": 327,
        "children": [
            {"type": "frame", "id": "copy", "width": 327,
             "children": [{"type": "text", "id": "title", "content": "Workout"}]},
            {"type": "frame", "id": "button", "width": 44,
             "children": [{"type": "text", "id": "label", "content": "Back"}]}
        ]
    }]);
    let mut sink = sink_with(root);
    let before = root_value(&sink);

    assert_eq!(enforce(&mut sink, &plan(), "root"), 0);
    assert_eq!(root_value(&sink), before);
}

#[test]
fn app18_shaped_image_stack_gets_scrim_and_light_overlay_copy() {
    let mut root = evidence_root();
    root["children"][1] = json!({
        "type": "frame", "id": "hero-section", "name": "Hero Image Header",
        "width": "fill_container", "height": "fit_content", "layout": "vertical",
        "children": [{
            "type": "frame", "id": "hero-stack", "name": "hero-stack",
            "width": 375, "height": 340, "layout": "none", "children": [
                {"type": "frame", "id": "back", "name": "Back Button",
                 "fill": [{"type": "solid", "color": "#0B1A0F80"}],
                 "children": [{"type": "icon_font", "id": "back-icon", "iconFontName": "arrow-left", "fill":
                    [{"type": "solid", "color": "$--color-error-foreground"}]}]},
                {"type": "frame", "id": "copy", "name": "Hero Content", "children": [
                    {"type": "text", "id": "eyebrow", "content": "CATEGORY", "fontSize": 12,
                     "fill": [{"type": "solid", "color": "$--foreground"}]},
                    {"type": "text", "id": "title", "name": "workout-title", "content": "Power Flow", "fontSize": 48,
                     "fill": [{"type": "solid", "color": "$--color-error-foreground"}]},
                    {"type": "frame", "id": "meta", "children": [
                        {"type": "text", "id": "meta-text", "content": "32 min", "fontSize": 14,
                         "fill": [{"type": "solid", "color": "$--foreground"}]}
                    ]}
                ]},
                {"type": "image", "id": "hero-photo", "width": 375, "height": 340,
                 "src": "workout.png"}
            ]
        }]
    });
    let mut sink = sink_with(root);
    let mut hero_plan = plan();
    hero_plan.subtasks[0].label = "Hero Image Header".into();

    assert_eq!(enforce(&mut sink, &hero_plan, "root"), 1);
    let hero = root_value(&sink)["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["name"] == "Hero Image Header (bleed)")
        .cloned()
        .expect("bleed hero section");
    let stack = &hero["children"][0];
    let children = stack["children"].as_array().expect("stack children");
    let image_index = children
        .iter()
        .position(|child| child["type"] == "image")
        .unwrap();
    assert_eq!(children[image_index + 1]["name"], "Hero Image Header scrim");
    assert_eq!(
        children[image_index + 1]["fill"][0]["type"],
        "linear_gradient"
    );
    assert_eq!(children[image_index + 1]["fill"][0]["angle"], 90.0);
    assert_eq!(
        children[image_index + 1]["fill"][0]["stops"][0]["color"],
        "#00000000"
    );
    assert_eq!(
        children[image_index + 1]["fill"][0]["stops"][1]["color"],
        "#000000A6"
    );
    assert_eq!(
        children[image_index + 1]["fill"][0]["explain"],
        "hero scrim"
    );
    assert_eq!(children[1]["children"][0]["fill"][0]["color"], "#FFFFFFCC");
    assert_eq!(children[1]["children"][1]["fill"][0]["color"], "#FFFFFF");
    assert_eq!(
        children[1]["children"][2]["children"][0]["fill"][0]["color"],
        "#FFFFFFCC"
    );
    assert_eq!(
        children[0]["children"][0]["fill"][0]["color"],
        "$--color-error-foreground"
    );
}

#[test]
fn image_stack_with_gradient_after_media_does_not_get_a_second_scrim() {
    let mut root = evidence_root();
    root["children"][1] = json!({
        "type": "frame", "id": "hero-section", "name": "Hero", "children": [{
            "type": "frame", "id": "stack", "layout": "none", "width": 375, "height": 300,
            "children": [
                {"type": "image", "id": "photo", "width": 375, "height": 300, "src": "hero.png"},
                {"type": "rectangle", "id": "overlay", "width": 375, "height": 300,
                 "fill": [{"type": "linear_gradient", "angle": 90, "stops": []}]},
                {"type": "text", "id": "title", "content": "Power Flow", "fontSize": 48,
                 "fill": [{"type": "solid", "color": "#0F172A"}]}
            ]
        }]
    });
    let mut sink = sink_with(root);
    assert_eq!(enforce(&mut sink, &plan(), "root"), 1);
    let stack = &hero_value(&sink)["children"][0];
    assert_eq!(
        stack["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|child| { child["fill"][0]["type"] == "linear_gradient" })
            .count(),
        1
    );
}

#[test]
fn coloured_media_hero_gets_no_image_scrim() {
    let mut root = evidence_root();
    root["children"][1] = json!({
        "type": "frame", "id": "hero-section", "name": "Colour Hero", "children": [{
            "type": "frame", "id": "stack", "layout": "none", "width": 375, "height": 200,
            "children": [
                {"type": "frame", "id": "colour", "width": 375, "height": 200,
                 "fill": [{"type": "solid", "color": "#0F172A"}]},
                {"type": "text", "id": "title", "content": "Power Flow", "fontSize": 48,
                 "fill": [{"type": "solid", "color": "#0F172A"}]}
            ]
        }]
    });
    let mut sink = sink_with(root);
    let mut coloured_plan = plan();
    coloured_plan.subtasks[0].label = "Colour Hero".into();
    assert_eq!(enforce(&mut sink, &coloured_plan, "root"), 1);
    let hero = root_value(&sink)["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["name"] == "Colour Hero (bleed)")
        .cloned()
        .expect("bleed hero section");
    let stack = &hero["children"][0];
    assert_eq!(stack["children"].as_array().unwrap().len(), 2);
    assert_eq!(stack["children"][1]["fill"][0]["color"], "#0F172A");
}

#[test]
fn text_inside_solid_chip_in_image_stack_is_not_recoloured() {
    let mut root = evidence_root();
    root["children"][1] = json!({
        "type": "frame", "id": "hero-section", "name": "Hero", "children": [{
            "type": "frame", "id": "stack", "layout": "none", "width": 375, "height": 200,
            "children": [
                {"type": "image", "id": "photo", "width": 375, "height": 200, "src": "hero.png"},
                {"type": "frame", "id": "chip", "fill": [{"type": "solid", "color": "#FFFFFF"}],
                 "children": [{"type": "text", "id": "chip-label", "content": "VIP", "fontSize": 14,
                    "fill": [{"type": "solid", "color": "#0F172A"}]}]},
                {"type": "text", "id": "title", "content": "Power Flow", "fontSize": 48,
                 "fill": [{"type": "solid", "color": "#0F172A"}]}
            ]
        }]
    });
    let mut sink = sink_with(root);
    assert_eq!(enforce(&mut sink, &plan(), "root"), 1);
    let stack = &hero_value(&sink)["children"][0];
    let chip = stack["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["id"] == "chip")
        .expect("chip");
    let title = stack["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["id"] == "title")
        .expect("title");
    assert_eq!(chip["children"][0]["fill"][0]["color"], "#0F172A");
    assert_eq!(title["fill"][0]["color"], "#FFFFFF");
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
