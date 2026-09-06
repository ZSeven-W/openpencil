use super::*;

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use serde_json::{json, Value};

use crate::apply_result;

fn image(source: &str, query: &str, width: f64) -> PenNode {
    serde_json::from_value(json!({
        "type": "image",
        "id": "slot",
        "name": "Exercise image",
        "src": source,
        "imageSearchQuery": query,
        "width": width,
        "height": 56
    }))
    .expect("valid image fixture")
}

fn patch_value(patch: &ImageFallbackPatch) -> Value {
    serde_json::from_str(&patch.patch_json).expect("valid patch")
}

#[test]
fn authored_width_selects_thumb_and_media_branches() {
    let thumb = image(SEARCH_FAILED_PLACEHOLDER_SRC, "jump squat exercise", 56.0);
    let media = image(SEARCH_FAILED_PLACEHOLDER_SRC, "city skyline", 320.0);

    let thumb_patch = &image_fallback_policy(&thumb, false)[0];
    assert_eq!(thumb_patch.branch, ImageFallbackBranch::Thumb);
    assert_eq!(
        patch_value(thumb_patch)["children"][0]["iconFontName"],
        "dumbbell"
    );

    let media_patch = &image_fallback_policy(&media, false)[0];
    assert_eq!(media_patch.branch, ImageFallbackBranch::Media);
    assert_eq!(
        patch_value(media_patch)["children"][1]["content"],
        "City skyline"
    );
}

#[test]
fn fallback_patch_keeps_intent_and_has_a_stable_marker() {
    let node: PenNode = serde_json::from_value(json!({
        "type": "image",
        "id": "slot",
        "name": "Generated art",
        "src": SEARCH_FAILED_PLACEHOLDER_SRC,
        "imageSearchQuery": "forest trail",
        "imagePrompt": "a painted moonlit forest",
        "width": 320,
        "height": 180
    }))
    .unwrap();
    let patch = patch_value(&image_fallback_policy(&node, false)[0]);
    assert_eq!(patch["imageSearchQuery"], "forest trail");
    assert_eq!(patch["imagePrompt"], "a painted moonlit forest");
    assert!(patch["name"]
        .as_str()
        .unwrap()
        .ends_with(IMAGE_FALLBACK_NAME_SUFFIX));
    assert!(patch["explain"]
        .as_str()
        .unwrap()
        .starts_with("image fallback:"));
}

#[test]
fn apply_result_immediately_rewrites_failed_image_and_is_idempotent() {
    let mut state = EditorState::new();
    state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![image("", "coffee cup", 56.0)],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let id = NodeId::new("slot");

    assert!(apply_result(&mut state, &id, SEARCH_FAILED_PLACEHOLDER_SRC));
    let fallback = op_editor_core::walkers::find_node(state.active_children(), &id)
        .expect("fallback survives");
    assert!(matches!(fallback, PenNode::Frame(_)));
    assert!(is_image_fallback(fallback));
    assert!(!apply_image_fallback_policy_to_node(&mut state, &id));
}

#[test]
fn retry_inverse_restores_the_same_image_id_and_intent() {
    let mut state = EditorState::new();
    state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![image(SEARCH_FAILED_PLACEHOLDER_SRC, "forest trail", 320.0)],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let id = NodeId::new("slot");
    assert!(apply_image_fallback_policy_to_node(&mut state, &id));
    let fallback = op_editor_core::walkers::find_node(state.active_children(), &id)
        .expect("fallback survives")
        .clone();

    let restored = restore_image_fallback_node(&fallback).expect("inverse image");
    let PenNode::Image(restored) = restored else {
        panic!("retry inverse must produce an image");
    };
    assert_eq!(restored.base.id, "slot");
    assert_eq!(restored.src, "");
    assert_eq!(restored.image_search_query.as_deref(), Some("forest trail"));
}
