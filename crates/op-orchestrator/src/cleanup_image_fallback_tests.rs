use super::*;

use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::EditorCommand;
use serde_json::json;

#[test]
fn map_template_adapter_resolves_dark_background_variable() {
    let mut sink = sink_with_image();
    sink.state.doc.variables = Some(
        serde_json::from_value(json!({
            "--background": {"type":"color", "value":"#101820"}
        }))
        .unwrap(),
    );
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("root"),
        page_id: None,
        patch_json: json!({"fill":[{"type":"solid","color":"$--background"}]}).to_string(),
    });
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("failed"),
        page_id: None,
        patch_json: json!({"imageSearchQuery":"map-template:delivery", "width":327, "height":260})
            .to_string(),
    });
    repair_map_placeholders(
        &mut sink,
        "root",
        &mut RepairSummary::default(),
        &mut RepairCounter::new(),
    );
    let node = walkers::find_node(sink.state.active_children(), &NodeId::new("failed")).unwrap();
    let node = serde_json::to_value(node).unwrap();
    let ground = node["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "map-ground")
        .unwrap();
    assert_eq!(ground["fill"][0]["color"], "#18232B");
}

#[test]
fn map_template_palette_rescues_dark_route_and_keeps_brand_variables_and_overlays() {
    let mut sink = sink_with_image();
    let variables = json!({
        "--background":{"type":"color","value":"#0D1B28"},
        "--primary":{"type":"color","value":"#0F172A"},
        "--primary-foreground":{"type":"color","value":"#FFFFFF"},
        "--accent":{"type":"color","value":"#465768"}
    });
    sink.state.doc.variables = Some(serde_json::from_value(variables.clone()).unwrap());
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("root"),
        page_id: None,
        patch_json: json!({"fill":[{"type":"solid","color":"$--background"}]}).to_string(),
    });
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("failed"),
        page_id: None,
        patch_json: json!({"imageSearchQuery":"map-template:delivery","width":327,"height":260})
            .to_string(),
    });
    repair_map_placeholders(
        &mut sink,
        "root",
        &mut RepairSummary::default(),
        &mut RepairCounter::new(),
    );
    let map = serde_json::to_value(
        walkers::find_node(sink.state.active_children(), &NodeId::new("failed")).unwrap(),
    )
    .unwrap();
    let children = map["children"].as_array().unwrap();
    let route = children
        .iter()
        .find(|node| node["name"] == "map-route")
        .unwrap();
    let color = route["stroke"]["fill"][0]["color"].as_str().unwrap();
    for bg in ["#18232B", "#29343C", "#28483E", "#40535D"] {
        assert!(
            op_design_lint::color::color_contrast(color, bg) >= 3.0,
            "{color} / {bg}"
        );
    }
    let pin = children
        .iter()
        .find(|node| node["name"] == "map-pin")
        .unwrap();
    assert_eq!(pin["fill"][0]["color"], color);
    assert_eq!(pin["children"][0]["fill"][0]["color"], "#111827");
    assert_eq!(
        serde_json::to_value(&sink.state.doc.variables).unwrap(),
        variables
    );
    let before = serde_json::to_value(sink.state.active_children()).unwrap();
    repair_map_placeholders(
        &mut sink,
        "root",
        &mut RepairSummary::default(),
        &mut RepairCounter::new(),
    );
    assert_eq!(
        serde_json::to_value(sink.state.active_children()).unwrap(),
        before
    );
}

#[test]
fn map_template_palette_prefers_readable_tokens_and_leaves_authored_controls_untouched() {
    use crate::{map_placeholder::build_map_placeholder, role_defaults::Theme};
    for (theme, primary, accent, expected) in [
        (Theme::Dark, "#FDBA74", "#465768", "$--primary"),
        (Theme::Dark, "#0F172A", "#FDBA74", "$--accent"),
        (Theme::Light, "#0F172A", "#FDBA74", "$--primary"),
        (Theme::Light, "#FAFCF8", "#FAFCF8", "#9A3412"),
    ] {
        let source: PenNode = serde_json::from_value(
            json!({"type":"image","id":"map","src":"","imageSearchQuery":"map-template:delivery"}),
        )
        .unwrap();
        let mut patch = build_map_placeholder(&source, 327.0, 260.0, theme);
        let overlay = json!({"type":"frame","name":"authored-brand-control","fill":[{"type":"solid","color":"$--primary"}]});
        patch["children"]
            .as_array_mut()
            .unwrap()
            .push(overlay.clone());
        map_template_palette::adapt(&mut patch, |raw| {
            Some(
                match raw {
                    "$--primary" => primary,
                    "$--accent" => accent,
                    "$--primary-foreground" => "#FFFFFF",
                    other => other,
                }
                .to_string(),
            )
        });
        let children = patch["children"].as_array().unwrap();
        assert_eq!(
            children.iter().find(|n| n["name"] == "map-route").unwrap()["stroke"]["fill"][0]
                ["color"],
            expected
        );
        assert_eq!(children.last().unwrap(), &overlay);
    }
}

#[test]
fn orange_brand_route_repairs_only_the_owned_halo_instead_of_turning_green() {
    use crate::{map_placeholder::build_map_placeholder, role_defaults::Theme};
    for kind in ["running", "delivery"] {
        let source: PenNode = serde_json::from_value(json!({"type":"image","id":"map",
            "src":"","imageSearchQuery":format!("map-template:{kind}")}))
        .unwrap();
        let mut patch = build_map_placeholder(&source, 375.0, 734.0, Theme::Dark);
        let authored = json!({"type":"frame","id":"authored","name":"map-route-halo",
            "fill":[{"type":"solid","color":"#40535D"}]});
        patch["children"]
            .as_array_mut()
            .unwrap()
            .push(authored.clone());
        map_template_palette::adapt(&mut patch, |raw| {
            Some(
                match raw {
                    "$--primary" => "#F97316",
                    "$--accent" => "#14B8A6",
                    "$--primary-foreground" => "#FFFFFF",
                    other => other,
                }
                .into(),
            )
        });
        let children = patch["children"].as_array().unwrap();
        let route = children.iter().find(|n| n["name"] == "map-route").unwrap();
        assert_eq!(route["stroke"]["fill"][0]["color"], "$--primary");
        for node in children.iter().filter(|n| n["explain"].is_string()) {
            let color = match node["name"].as_str().unwrap() {
                "map-route-halo" => node["stroke"]["fill"][0]["color"].as_str(),
                "map-origin-halo" | "map-destination-halo" | "map-courier" => {
                    node["fill"][0]["color"].as_str()
                }
                _ => None,
            };
            if let Some(color) = color {
                assert!(op_design_lint::color::color_contrast("#F97316", color) >= 3.0);
            }
        }
        assert_eq!(children.last().unwrap(), &authored);
    }
}

fn sink_with_image() -> VecDocSink {
    let mut sink = VecDocSink::new();
    let root: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "width": 390,
        "height": 844,
        "children": [{
            "type": "image",
            "id": "failed",
            "src": "placeholder://image-search-failed",
            "imageSearchQuery": "jump squat exercise",
            "width": 56,
            "height": 56
        }]
    }))
    .expect("valid fallback fixture");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![root],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

#[test]
fn fallback_adapter_records_node_and_policy_checkpoint() {
    let mut sink = sink_with_image();
    let mut counter = RepairCounter::new();
    let mut counting = counter.wrap(&mut sink);
    let mut summary = RepairSummary::default();
    repair_image_fallback_policy(&mut counting, &mut summary, &mut counter);
    assert_eq!(summary.repairs_for(CheckCategory::Structure), 1);
    assert!(summary
        .records()
        .iter()
        .any(|record| record.pass == "image-fallback-policy"
            && record.node_id == "failed"
            && record.detail.contains("thumb")));
}
