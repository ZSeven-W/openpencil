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
    assert!((4..=6).contains(
        &children
            .iter()
            .filter(|child| child["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("map-block-")))
            .count()
    ));
    assert_eq!(
        children
            .iter()
            .filter(|child| child["name"].as_str() == Some("map-road-h"))
            .count(),
        1
    );
    assert_eq!(
        children
            .iter()
            .filter(|child| child["name"].as_str() == Some("map-road-v"))
            .count(),
        1
    );
    assert_eq!(
        children
            .iter()
            .filter(|child| child["name"].as_str() == Some("map-route"))
            .count(),
        1
    );
    assert_eq!(
        children
            .iter()
            .filter(|child| child["name"].as_str() == Some("map-pin"))
            .count(),
        1
    );
    assert_eq!(
        children
            .iter()
            .filter(|child| child["name"].as_str() == Some("map-origin"))
            .count(),
        1
    );
    assert!(children.iter().any(|child| child["id"] == "locate"));
    for child in children {
        for field in ["x", "y", "width", "height"] {
            assert!(child[field].is_number(), "{field} missing on {child}");
        }
    }
}

#[test]
fn map_layout_is_seeded_by_node_id_and_dark_pages_use_secondary() {
    let source = map_node();
    let light_a = map_placeholder(&source, &rects(), Theme::Light);
    let light_b = map_placeholder(&source, &rects(), Theme::Light);
    assert_eq!(light_a, light_b);

    let dark = patch_value(&map_placeholder(&source, &rects(), Theme::Dark)[0]);
    assert_eq!(dark["fill"][0]["color"], "$--secondary");
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
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
}
