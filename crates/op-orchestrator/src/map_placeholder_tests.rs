use super::*;

use crate::image_fallback_policy::ResolvedRect;
use crate::role_defaults::Theme;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, PenNodeExt};
use op_image_enrich::collect_targets;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

fn map_node() -> PenNode {
    serde_json::from_value(json!({
        "type": "frame",
        "id": "city-map",
        "name": "地图占位区",
        "imageSearchQuery": "city map",
        "width": 327,
        "height": 280,
        "x": 16,
        "y": 120,
        "cornerRadius": 12,
        "children": [{
            "type": "frame",
            "id": "locate",
            "name": "Locate button",
            "x": 267,
            "y": 220,
            "width": 44,
            "height": 44,
            "layout": "none"
        }]
    }))
    .expect("valid map fixture")
}

fn rects() -> HashMap<String, ResolvedRect> {
    HashMap::from([(
        "city-map".to_string(),
        ResolvedRect {
            x: 16.0,
            y: 120.0,
            width: 327.0,
            height: 280.0,
        },
    )])
}

fn patch_value(patch: &MapPlaceholderPatch) -> Value {
    serde_json::from_str(&patch.patch_json).expect("valid map patch")
}

fn evidence_section() -> PenNode {
    serde_json::from_value(json!({
        "type": "frame",
        "id": "map-section",
        "name": "Map & Current Location",
        "width": 375,
        "height": 320,
        "layout": "vertical",
        "children": [{
            "type": "image",
            "id": "map-image",
            "src": "placeholder://image-search-failed",
            "imageSearchQuery": "city map",
            "width": 327,
            "height": 300
        }, {
            "type": "frame",
            "id": "current-location-card",
            "name": "Current location card",
            "layout": "vertical",
            "width": 327,
            "height": 88,
            "children": [{
                "type": "text",
                "id": "current-location-label",
                "content": "当前位置",
                "width": 100,
                "height": 20
            }, {
                "type": "text_input",
                "id": "current-location-input",
                "width": 300,
                "height": 44,
                "placeholder": "当前位置"
            }]
        }]
    }))
    .expect("valid section map fixture")
}

#[test]
fn city_map_becomes_a_vector_placeholder_and_clears_search_intent() {
    let source = map_node();
    let patches = map_placeholder(&source, &rects(), Theme::Light);
    assert_eq!(patches.len(), 1);
    let patch = patch_value(&patches[0]);
    assert_eq!(patch["type"], "frame");
    assert_eq!(patch["layout"], "none");
    assert_eq!(patch["clipContent"], true);
    assert_eq!(patch["fill"][0]["color"], "$--muted");
    assert!(patch["imageSearchQuery"].is_null());

    let children = patch["children"].as_array().unwrap();
    let map_blocks: Vec<&Value> = children
        .iter()
        .filter(|child| {
            child["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("map-block-"))
        })
        .collect();
    assert_eq!(map_blocks.len(), 12);
    let expected_block_width = (327.0 - 5.0 * 12.0) / 4.0;
    assert!(map_blocks.iter().all(|block| {
        (block["width"].as_f64().unwrap() - expected_block_width).abs() < f64::EPSILON
    }));
    assert_eq!(
        children
            .iter()
            .filter(|child| child["name"].as_str() == Some("map-road-h"))
            .count(),
        1
    );
    let road_h = children
        .iter()
        .find(|child| child["name"].as_str() == Some("map-road-h"))
        .unwrap();
    let road_v = children
        .iter()
        .find(|child| child["name"].as_str() == Some("map-road-v"))
        .unwrap();
    assert_eq!(road_h["height"], json!(10.0));
    assert_eq!(road_v["width"], json!(10.0));
    assert!(
        road_h["y"].as_f64().unwrap()
            > map_blocks[0]["y"].as_f64().unwrap() + map_blocks[0]["height"].as_f64().unwrap()
    );
    assert!(road_h["y"].as_f64().unwrap() < map_blocks[4]["y"].as_f64().unwrap());
    assert!(
        road_v["x"].as_f64().unwrap()
            > map_blocks[0]["x"].as_f64().unwrap() + map_blocks[0]["width"].as_f64().unwrap()
    );
    assert!(road_v["x"].as_f64().unwrap() < map_blocks[1]["x"].as_f64().unwrap());
    let route_index = children
        .iter()
        .position(|child| child["name"].as_str() == Some("map-route"))
        .unwrap();
    let road_h_index = children
        .iter()
        .position(|child| child["name"].as_str() == Some("map-road-h"))
        .unwrap();
    let road_v_index = children
        .iter()
        .position(|child| child["name"].as_str() == Some("map-road-v"))
        .unwrap();
    assert!(route_index > road_h_index && route_index > road_v_index);
    assert_eq!(children[route_index]["stroke"]["thickness"], json!(5));
    assert_eq!(children[route_index]["stroke"]["join"], json!("round"));
    assert_eq!(children[route_index]["stroke"]["cap"], json!("round"));
    assert_eq!(
        children[route_index]["stroke"]["fill"],
        json!([{ "type": "solid", "color": "$--primary" }])
    );
    let pin_index = children
        .iter()
        .position(|child| child["name"].as_str() == Some("map-pin"))
        .unwrap();
    let origin_halo_index = children
        .iter()
        .position(|child| child["name"].as_str() == Some("map-origin-halo"))
        .unwrap();
    let origin_index = children
        .iter()
        .position(|child| child["name"].as_str() == Some("map-origin"))
        .unwrap();
    assert!(pin_index > route_index);
    assert!(origin_halo_index > route_index);
    assert!(origin_index > route_index);
    assert!(patch.get("height").is_none());
    assert!(children.iter().any(|child| child["id"] == "locate"));
    for child in children {
        for field in ["x", "y", "width", "height"] {
            assert!(child[field].is_number(), "{field} missing on {child}");
        }
    }
}

#[test]
fn map_layout_is_stable_and_dark_pages_use_secondary() {
    let source = map_node();
    let light_a = map_placeholder(&source, &rects(), Theme::Light);
    let light_b = map_placeholder(&source, &rects(), Theme::Light);
    assert_eq!(light_a, light_b);

    let dark = patch_value(&map_placeholder(&source, &rects(), Theme::Dark)[0]);
    assert_eq!(dark["fill"][0]["color"], "$--secondary");
}

#[test]
fn section_map_name_does_not_replace_content_container() {
    let source = evidence_section();
    let original_card = source.children().unwrap()[1].clone();
    let rects = HashMap::from([
        (
            "map-section".to_string(),
            ResolvedRect {
                x: 0.0,
                y: 0.0,
                width: 375.0,
                height: 320.0,
            },
        ),
        (
            "map-image".to_string(),
            ResolvedRect {
                x: 24.0,
                y: 80.0,
                width: 327.0,
                height: 300.0,
            },
        ),
    ]);
    let patches = map_placeholder(&source, &rects, Theme::Light);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].node_id, "map-image");

    let mut state = EditorState::new();
    state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![source],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    state.apply(EditorCommand::PatchNodeData {
        node_id: op_editor_core::NodeId::new("map-image"),
        patch_json: patches[0].patch_json.clone(),
        page_id: None,
    });

    let section = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("map-section"),
    )
    .expect("section remains");
    assert_eq!(
        serde_json::to_value(section).unwrap()["layout"],
        json!("vertical")
    );
    let children = section.children().expect("section children remain");
    assert_eq!(children.len(), 2);
    assert_eq!(
        children[0].base().name.as_deref(),
        Some("Map (map placeholder)")
    );
    assert_eq!(
        serde_json::to_value(&children[1]).unwrap(),
        serde_json::to_value(original_card).unwrap()
    );
    assert!(matches!(
        children[1].children().unwrap()[0],
        PenNode::Text(_)
    ));
    assert!(matches!(
        children[1].children().unwrap()[1],
        PenNode::TextInput(_)
    ));
}

#[test]
fn fit_content_height_is_sized_without_overwriting_width_sizing() {
    let source: PenNode = serde_json::from_value(json!({
        "type": "image",
        "id": "fit-map",
        "name": "Map image",
        "src": "placeholder://image-search-failed",
        "imagePrompt": "city map",
        "width": "fill_container",
        "height": "fit_content"
    }))
    .expect("valid fit-content map fixture");
    let patch = patch_value(&map_placeholder(&source, &HashMap::new(), Theme::Light)[0]);
    assert_eq!(patch["height"], json!(300.0));
    assert!(patch.get("width").is_none());
    let block = patch["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["name"] == "map-block-1")
        .unwrap();
    assert_eq!(block["width"], json!((327.0 - 5.0 * 12.0) / 4.0));
}

#[test]
fn applying_map_patch_preserves_pinned_control_and_removes_targets() {
    let source = map_node();
    let patch = patch_value(&map_placeholder(&source, &rects(), Theme::Light)[0]);
    let mut state = EditorState::new();
    state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![source],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    state.apply(EditorCommand::PatchNodeData {
        node_id: op_editor_core::NodeId::new("city-map"),
        patch_json: patch.to_string(),
        page_id: None,
    });
    let rewritten = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("city-map"),
    )
    .expect("rewritten map exists");
    assert!(matches!(rewritten, PenNode::Frame(_)));
    assert_eq!(rewritten.base().x, Some(16.0));
    assert_eq!(rewritten.base().y, Some(120.0));
    assert_eq!(rewritten.width_px(), Some(327.0));
    assert_eq!(rewritten.height_px(), Some(280.0));
    assert_eq!(
        serde_json::to_value(rewritten).unwrap()["cornerRadius"].as_f64(),
        Some(12.0)
    );
    assert!(rewritten
        .children()
        .unwrap()
        .iter()
        .any(|child| child.id_str() == "locate"));
    assert!(map_placeholder(rewritten, &rects(), Theme::Light).is_empty());
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
}

#[test]
fn decorations_inside_a_hand_drawn_map_and_small_map_shapes_are_left_alone() {
    // GLM-5.3-Flash drew its own map and named the inner blocks map-park /
    // map-water; each was turned into a nested mini map. A map-named
    // container's children are decorations, and a lone map-named shape
    // narrower than 200 px is not a slot either. A wide leaf still is.
    let root: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Ride Home",
        "width": 375,
        "height": 812,
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "map-canvas",
                "name": "map-canvas",
                "width": 327,
                "height": 320,
                "layout": "none",
                "children": [
                    {"type": "rectangle", "id": "park", "name": "map-park", "x": 20, "y": 30, "width": 110, "height": 90},
                    {"type": "text", "id": "road-label", "name": "road-label", "content": "科苑南路", "x": 60, "y": 10}
                ]
            },
            {"type": "rectangle", "id": "lone-water", "name": "map-water", "width": 110, "height": 90},
            {"type": "rectangle", "id": "wide-tile", "name": "map-tile", "width": 327, "height": 300}
        ]
    }))
    .expect("fixture");
    let rects = HashMap::from([
        (
            "map-canvas".to_string(),
            ResolvedRect {
                x: 24.0,
                y: 62.0,
                width: 327.0,
                height: 320.0,
            },
        ),
        (
            "park".to_string(),
            ResolvedRect {
                x: 44.0,
                y: 92.0,
                width: 110.0,
                height: 90.0,
            },
        ),
        (
            "lone-water".to_string(),
            ResolvedRect {
                x: 24.0,
                y: 400.0,
                width: 110.0,
                height: 90.0,
            },
        ),
        (
            "wide-tile".to_string(),
            ResolvedRect {
                x: 24.0,
                y: 500.0,
                width: 327.0,
                height: 300.0,
            },
        ),
    ]);
    let patches = map_placeholder(&root, &rects, Theme::Light);
    let ids: Vec<&str> = patches.iter().map(|p| p.node_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["wide-tile"],
        "only the wide leaf is a map slot; got {ids:?}"
    );
}
