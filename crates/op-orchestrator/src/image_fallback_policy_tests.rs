use super::*;

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, PenNodeExt};
use serde_json::{json, Value};
use std::collections::HashMap;

fn node(source: &str, query: &str, width: f64, height: f64) -> PenNode {
    serde_json::from_value(json!({
        "type": "image",
        "id": "slot",
        "name": "Exercise image",
        "src": source,
        "imageSearchQuery": query,
        "width": width,
        "height": height,
        "x": 12,
        "y": 20,
        "cornerRadius": 8
    }))
    .expect("valid image fixture")
}

fn rect(width: f64, height: f64) -> HashMap<String, ResolvedRect> {
    HashMap::from([(
        "slot".to_string(),
        ResolvedRect {
            x: 12.0,
            y: 20.0,
            width,
            height,
        },
    )])
}

fn patch_value(patch: &ImageFallbackPatch) -> Value {
    serde_json::from_str(&patch.patch_json).expect("valid patch")
}

#[test]
fn failed_thumb_uses_muted_dumbbell_tile() {
    let image = node(
        SEARCH_FAILED_PLACEHOLDER_SRC,
        "jump squat exercise",
        56.0,
        56.0,
    );
    let patches = image_fallback_policy(&image, &rect(56.0, 56.0), false);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].branch, ImageFallbackBranch::Thumb);
    let patch = patch_value(&patches[0]);
    assert_eq!(patch["type"], "frame");
    assert_eq!(patch["fill"][0]["color"], "$--muted");
    assert_eq!(patch["children"][0]["iconFontName"], "dumbbell");
    assert_eq!(patch["children"][0]["width"], 20);
    assert_eq!(patch["explain"], "image fallback: thumb");
}

#[test]
fn failed_hero_uses_icon_and_sentence_case_caption() {
    let image = node(SEARCH_FAILED_PLACEHOLDER_SRC, "city skyline", 327.0, 280.0);
    let patches = image_fallback_policy(&image, &rect(327.0, 280.0), false);
    assert_eq!(patches[0].branch, ImageFallbackBranch::Media);
    let patch = patch_value(&patches[0]);
    assert_eq!(patch["layout"], "vertical");
    assert_eq!(patch["children"].as_array().unwrap().len(), 2);
    assert_eq!(patch["children"][0]["iconFontName"], "image");
    assert_eq!(patch["children"][0]["width"], 30);
    assert_eq!(patch["children"][1]["content"], "City skyline");
    assert_eq!(patch["children"][1]["fontSize"], 12);
    assert_eq!(
        patch["children"][1]["fill"][0]["color"],
        "$--muted-foreground"
    );
}

#[test]
fn resolved_op_image_is_untouched_and_rewrite_is_idempotent() {
    let image = node("op-image:asset-42", "jump squat exercise", 56.0, 56.0);
    assert!(image_fallback_policy(&image, &rect(56.0, 56.0), true).is_empty());

    let failed = node(
        SEARCH_FAILED_PLACEHOLDER_SRC,
        "jump squat exercise",
        56.0,
        56.0,
    );
    let patches = image_fallback_policy(&failed, &rect(56.0, 56.0), false);
    let rewritten: PenNode = {
        let mut value = serde_json::to_value(&failed).unwrap();
        let object = value.as_object_mut().unwrap();
        for (key, value) in patch_value(&patches[0]).as_object().unwrap() {
            object.insert(key.clone(), value.clone());
        }
        serde_json::from_value(value).unwrap()
    };
    assert!(image_fallback_policy(&rewritten, &rect(56.0, 56.0), true).is_empty());
    assert!(matches!(rewritten, PenNode::Frame(_)));
}

#[test]
fn after_enrich_converts_an_empty_image_slot() {
    let image = node("", "coffee cup", 56.0, 56.0);
    assert!(image_fallback_policy(&image, &rect(56.0, 56.0), false).is_empty());
    let patches = image_fallback_policy(&image, &rect(56.0, 56.0), true);
    assert_eq!(patches.len(), 1);
    assert_eq!(
        patch_value(&patches[0])["children"][0]["iconFontName"],
        "coffee"
    );
}

#[test]
fn enrich_side_state_hook_applies_after_enrichment() {
    let image = node("", "jump squat exercise", 56.0, 56.0);
    let mut state = EditorState::new();
    state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![image],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });

    assert_eq!(apply_image_fallback_policy_to_state(&mut state, true), 1);
    let rewritten = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("slot"),
    )
    .expect("fallback node exists");
    assert!(matches!(rewritten, PenNode::Frame(_)));
    assert_eq!(
        rewritten.base().name.as_deref(),
        Some("Exercise image (image fallback)")
    );
}

#[test]
fn every_fallback_icon_name_exists_in_lucide_catalog() {
    let catalog_path = format!(
        "{}/../op-editor-ui/assets/iconify-catalog-core.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let catalog = std::fs::read_to_string(catalog_path).expect("read icon catalog");
    for icon in [
        "dumbbell",
        "utensils",
        "coffee",
        "shopping-bag",
        "map",
        "user",
        "building",
        "car",
        "music",
        "play",
        "image",
        "map-pin",
    ] {
        assert!(catalog.contains(&format!("\"{icon}\"")), "missing {icon}");
    }
}
