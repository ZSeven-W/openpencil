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
fn keyword_width_media_caption_uses_the_resolved_slot_and_wraps() {
    let doc=jian_ops_schema::load_str(&json!({"version":"1.0.0","children":[
        {"type":"frame","id":"root","width":100,"height":120,"layout":"vertical","children":[
            {"type":"image","id":"slot","width":"fill_container","height":120,
             "src":SEARCH_FAILED_PLACEHOLDER_SRC,"imageSearchQuery":"stretching recovery exercise after a long day"}
        ]}
    ]}).to_string()).unwrap().value;
    let mut state = EditorState::from_document(doc);
    assert!(apply_image_fallback_policy_to_node(
        &mut state,
        &NodeId::new("slot")
    ));
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&state);
    let page = scene.active_page().unwrap();
    let slot = page.find("slot").unwrap().bounds;
    let caption = page.find("slot-image-fallback-caption").unwrap().bounds;
    assert!(
        caption.size.x <= slot.size.x + 1.0,
        "{caption:?} inside {slot:?}"
    );
    assert!(
        caption.origin.y + caption.size.y <= slot.origin.y + slot.size.y + 1.0,
        "{caption:?} inside {slot:?}"
    );
    let saved = state.doc.clone();
    assert!(!apply_image_fallback_policy_to_node(
        &mut state,
        &NodeId::new("slot")
    ));
    assert_eq!(state.doc, saved);
}

#[test]
fn legacy_policy_caption_upgrades_its_layout_without_rewriting_copy_or_typography() {
    let node:PenNode=serde_json::from_value(json!({"type":"frame","id":"slot","name":"Media (image fallback)",
        "explain":"image fallback: media {}","width":"fill_container","height":120,"layout":"vertical",
        "children":[{"type":"text","id":"slot-image-fallback-caption","name":"Image fallback caption",
            "content":"用户修改后的说明","width":160,"height":16,"fontSize":14,"fontWeight":600,"fill":[{"type":"solid","color":"#123456"}]}]
    })).unwrap();
    let patches = image_fallback_policy(&node, false);
    assert_eq!(patches.len(), 1);
    let caption = &patch_value(&patches[0])["children"][0];
    assert_eq!(caption["content"], "用户修改后的说明");
    assert_eq!(caption["fontSize"], 14.0);
    assert_eq!(caption["fontWeight"], 600.0);
    assert_eq!(caption["id"], "slot-image-fallback-caption");
    assert_eq!(caption["width"], "fill_container");
    let mut customized = serde_json::to_value(&node).unwrap();
    customized["children"][0]["height"] = json!(32);
    assert!(image_fallback_policy(&serde_json::from_value(customized).unwrap(), false).is_empty());
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

#[test]
fn a_failed_slot_under_overlay_text_becomes_a_silent_block() {
    // The hero shape: a `layout: none` stack with the title laid over the
    // image. The tile must not put an icon or a caption under that title.
    let stack: PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "stack", "layout": "none", "width": 375, "height": 350,
        "children": [
            {"type": "frame", "id": "overlay", "x": 0, "y": 105, "width": 375,
             "children": [{"type": "text", "id": "title", "content": "Full Body Burn"}]},
            {"type": "image", "id": "hero", "name": "Hero image", "x": 0, "y": 0,
             "src": SEARCH_FAILED_PLACEHOLDER_SRC, "imageSearchQuery": "dark gym workout",
             "width": 375, "height": 350}
        ]
    }))
    .expect("valid stack fixture");

    let patches = image_fallback_policy(&stack, false);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].node_id, "hero");
    assert_eq!(patches[0].branch, ImageFallbackBranch::Covered);
    let patch = patch_value(&patches[0]);
    assert_eq!(patch["children"], json!([]));
    assert_eq!(patch["fill"][0]["color"], "$--muted");
    assert_eq!(patch["imageSearchQuery"], "dark gym workout");

    // The same slot in a stack WITHOUT text keeps the icon + caption tile.
    let plain: PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "stack", "layout": "none", "width": 375, "height": 350,
        "children": [
            {"type": "image", "id": "hero", "name": "Hero image", "x": 0, "y": 0,
             "src": SEARCH_FAILED_PLACEHOLDER_SRC, "imageSearchQuery": "dark gym workout",
             "width": 375, "height": 350}
        ]
    }))
    .expect("valid stack fixture");
    assert_eq!(
        image_fallback_policy(&plain, false)[0].branch,
        ImageFallbackBranch::Media
    );
}
