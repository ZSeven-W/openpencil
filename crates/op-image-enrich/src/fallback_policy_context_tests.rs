use super::*;

fn slot(width: f64) -> PenNode {
    serde_json::from_value(json!({
        "type": "image", "id": "hero", "name": "Workout photo", "width": width,
        "height": 240, "src": SEARCH_FAILED_PLACEHOLDER_SRC,
        "imageSearchQuery": "dumbbell shoulder press",
        "imagePrompt": "A person pressing two dumbbells overhead",
        "explain": "image request: search"
    }))
    .unwrap()
}

fn stack(image: PenNode, layout: &str, with_title: bool) -> PenNode {
    let mut children = vec![serde_json::to_value(image).unwrap()];
    if with_title {
        children.push(json!({
            "type": "frame", "id": "title-overlay", "x": 20, "y": 160,
            "children": [{"type": "text", "id": "title", "content": "Upper Body"}]
        }));
    }
    serde_json::from_value(json!({
        "type": "frame", "id": "stack", "layout": layout,
        "width": 375, "height": 240, "children": children
    }))
    .unwrap()
}

fn state_with(node: PenNode) -> EditorState {
    let mut state = EditorState::new();
    assert!(state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    }));
    state
}

fn node(state: &EditorState, id: &str) -> PenNode {
    walkers::find_node(state.active_children(), &NodeId::new(id))
        .unwrap()
        .clone()
}

fn media_fallback() -> PenNode {
    let mut state = state_with(slot(375.0));
    assert!(apply_image_fallback_policy_to_node(
        &mut state,
        &NodeId::new("hero")
    ));
    node(&state, "hero")
}

#[test]
fn targeted_fallback_matches_full_tree_overlay_and_width_rules() {
    for (layout, title, width, expected) in [
        ("none", true, 375.0, ImageFallbackBranch::Covered),
        ("none", false, 375.0, ImageFallbackBranch::Media),
        ("vertical", true, 375.0, ImageFallbackBranch::Media),
        ("none", true, 96.0, ImageFallbackBranch::Thumb),
        ("none", true, 97.0, ImageFallbackBranch::Covered),
    ] {
        let root = stack(slot(width), layout, title);
        let patches = image_fallback_policy(&root, false);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].branch, expected);
        let mut full_tree = state_with(root.clone());
        assert!(full_tree.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new("hero"),
            patch_json: patches[0].patch_json.clone(),
            page_id: None,
        }));
        let mut targeted = state_with(root);
        assert!(apply_image_fallback_policy_to_node(
            &mut targeted,
            &NodeId::new("hero")
        ));
        assert_eq!(node(&targeted, "stack"), node(&full_tree, "stack"));
        let saved = targeted.doc.clone();
        assert!(!apply_image_fallback_policy_to_node(
            &mut targeted,
            &NodeId::new("hero")
        ));
        assert_eq!(targeted.doc, saved);
        assert!(image_fallback_policy(&node(&targeted, "stack"), false).is_empty());
    }
}

#[test]
fn targeted_container_does_not_receive_a_descendants_patch() {
    let mut state = state_with(stack(slot(375.0), "none", true));
    let saved = state.doc.clone();
    assert!(!apply_image_fallback_policy_to_node(
        &mut state,
        &NodeId::new("stack")
    ));
    assert_eq!(state.doc, saved);
}

#[test]
fn image_search_failure_uses_parent_context_immediately() {
    let mut image = slot(375.0);
    let PenNode::Image(ref mut image_node) = image else {
        unreachable!()
    };
    image_node.src = String::new().into();
    let mut state = state_with(stack(image, "none", true));
    let title = node(&state, "title-overlay");
    assert!(crate::apply_result(
        &mut state,
        &NodeId::new("hero"),
        SEARCH_FAILED_PLACEHOLDER_SRC
    ));
    let fallback = node(&state, "hero");
    assert!(fallback.children().unwrap().is_empty());
    assert_eq!(
        fallback_metadata(&fallback),
        Some(json!({
            "imageSearchQuery": "dumbbell shoulder press",
            "imagePrompt": "A person pressing two dumbbells overhead",
        "imageRequestMode": "auto",
            "originalExplain": "image request: search"
        }))
    );
    assert_eq!(node(&state, "title-overlay"), title);
    let restored = restore_image_fallback_node(&fallback).unwrap();
    let PenNode::Image(restored) = restored else {
        panic!("image retry")
    };
    assert_eq!(restored.base.id, "hero");
    assert_eq!(
        restored.base.explain.as_deref(),
        Some("image request: search")
    );
    assert_eq!(
        restored.image_prompt.as_deref(),
        Some("A person pressing two dumbbells overhead")
    );
}

#[test]
fn existing_media_tile_is_repaired_without_rewriting_retry_metadata_or_geometry() {
    let fallback = media_fallback();
    let original_metadata = fallback_metadata(&fallback);
    let original_restored = restore_image_fallback_node(&fallback);
    let original = serde_json::to_value(&fallback).unwrap();
    let mut state = state_with(stack(fallback, "none", true));
    let patches = image_fallback_policy(&node(&state, "stack"), false);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].branch, ImageFallbackBranch::Covered);
    assert!(apply_image_fallback_policy_to_node(
        &mut state,
        &NodeId::new("hero")
    ));
    let repaired = node(&state, "hero");
    assert_eq!(fallback_metadata(&repaired), original_metadata);
    assert!(repaired.children().unwrap().is_empty());
    assert_eq!(restore_image_fallback_node(&repaired), original_restored);
    let repaired_json = serde_json::to_value(&repaired).unwrap();
    for key in [
        "name",
        "width",
        "height",
        "layout",
        "fill",
        "imageSearchQuery",
        "imagePrompt",
    ] {
        assert_eq!(repaired_json[key], original[key], "{key}");
    }
    assert!(!apply_image_fallback_policy_to_node(
        &mut state,
        &NodeId::new("hero")
    ));
}

#[test]
fn edited_fallback_caption_prevents_automatic_overlay_repair() {
    let mut fallback = serde_json::to_value(media_fallback()).unwrap();
    fallback["children"][1]["content"] = json!("My chosen caption");
    fallback["children"][1]["fontSize"] = json!(15);
    let mut state = state_with(stack(
        serde_json::from_value(fallback).unwrap(),
        "none",
        true,
    ));
    let before = state.doc.clone();
    assert!(!apply_image_fallback_policy_to_node(
        &mut state,
        &NodeId::new("hero")
    ));
    assert_eq!(state.doc, before);
}

#[test]
fn fallback_caption_does_not_turn_a_plain_stack_into_an_overlay() {
    let root = stack(media_fallback(), "none", false);
    assert!(image_fallback_policy(&root, false).is_empty());
    let mut state = state_with(root);
    assert!(!apply_image_fallback_policy_to_node(
        &mut state,
        &NodeId::new("hero")
    ));
    assert_eq!(node(&state, "hero").children().unwrap().len(), 2);
}

#[test]
fn authored_children_prevent_automatic_overlay_repair() {
    let mut fallback = serde_json::to_value(media_fallback()).unwrap();
    fallback["children"].as_array_mut().unwrap().push(json!({
        "type": "text", "id": "authored-note", "content": "Keep this note",
        "fontSize": 16, "x": 12, "y": 180
    }));
    let mut state = state_with(stack(
        serde_json::from_value(fallback).unwrap(),
        "none",
        true,
    ));
    let before = state.doc.clone();
    assert!(!apply_image_fallback_policy_to_node(
        &mut state,
        &NodeId::new("hero")
    ));
    assert_eq!(state.doc, before);
}

#[test]
fn legacy_caption_is_repaired_in_one_pass_only_when_a_real_overlay_exists() {
    for (with_title, width) in [
        (false, 120.0_f64),
        (false, 375.0),
        (true, 120.0),
        (true, 375.0),
    ] {
        let mut fallback = serde_json::to_value(media_fallback()).unwrap();
        fallback["width"] = json!(width);
        fallback["children"][1]["width"] = json!(width.clamp(1.0, 160.0));
        fallback["children"][1]["height"] = json!(16);
        fallback["children"][1]
            .as_object_mut()
            .unwrap()
            .remove("textGrowth");
        let mut state = state_with(stack(
            serde_json::from_value(fallback).unwrap(),
            "none",
            with_title,
        ));
        assert!(apply_image_fallback_policy_to_node(
            &mut state,
            &NodeId::new("hero")
        ));
        assert_eq!(
            node(&state, "hero").children().unwrap().len(),
            if with_title { 0 } else { 2 }
        );
        let after = state.doc.clone();
        assert!(!apply_image_fallback_policy_to_node(
            &mut state,
            &NodeId::new("hero")
        ));
        assert!(image_fallback_policy(&node(&state, "stack"), false).is_empty());
        assert_eq!(state.doc, after);
    }
}

#[test]
fn changed_or_unverifiable_legacy_caption_width_is_never_normalized_or_removed() {
    for (slot_width, caption_width) in [
        (json!(375), 240.0),
        (json!(375), 320.0),
        (json!(375), 0.0),
        (json!(375), -1.0),
        (json!(375), 80.0),
        (json!(120), 160.0),
        (json!(0), 160.0),
        (json!(-1), 160.0),
        (json!("fill_container"), 160.0),
    ] {
        let mut fallback = serde_json::to_value(media_fallback()).unwrap();
        fallback["width"] = slot_width;
        fallback["children"][1]["width"] = json!(caption_width);
        fallback["children"][1]["height"] = json!(16);
        fallback["children"][1]
            .as_object_mut()
            .unwrap()
            .remove("textGrowth");
        let mut state = state_with(stack(
            serde_json::from_value(fallback).unwrap(),
            "none",
            true,
        ));
        let before = state.doc.clone();
        for _ in 0..2 {
            assert!(!apply_image_fallback_policy_to_node(
                &mut state,
                &NodeId::new("hero")
            ));
            assert!(image_fallback_policy(&node(&state, "stack"), false).is_empty());
            assert_eq!(state.doc, before);
        }
    }
}
