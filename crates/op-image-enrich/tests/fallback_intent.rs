use std::collections::{HashMap, HashSet};

use op_editor_core::{walkers, EditorCommand, EditorState, NodeId};
use op_image_enrich::{
    apply_result, collect_targets, image_fallback_policy_with_widths, image_request_mode,
    ImageRequestMode, ImageSearchTarget, SEARCH_FAILED_PLACEHOLDER_SRC,
};
use serde_json::{json, Value};

const QUERY: &str = "dumbbell squat workout";
// Retained from the 09 workout-detail regression: broad retrieval keywords
// must not replace the authored goblet-squat action during desktop retries.
const PROMPT: &str = "Athlete performing a goblet squat holding a dumbbell at chest height, gym setting, natural light";

fn state(query: Option<&str>, prompt: Option<&str>) -> EditorState {
    let document = serde_json::from_value(json!({
        "version": "1.0.0", "children": [{
            "type": "image", "id": "slot", "name": "Goblet Squat",
            "width": 64, "height": 64, "src": "",
            "imageSearchQuery": query, "imagePrompt": prompt
        }]
    }))
    .unwrap();
    EditorState::from_document(document)
}

fn target(state: &EditorState) -> ImageSearchTarget {
    let targets = collect_targets(state, &HashSet::new());
    assert_eq!(targets.len(), 1, "{targets:?}");
    targets.into_iter().next().unwrap()
}

fn fail(state: &mut EditorState) {
    assert!(apply_result(
        state,
        &NodeId::new("slot"),
        SEARCH_FAILED_PLACEHOLDER_SRC
    ));
}

fn patch(state: &mut EditorState, patch: Value) {
    assert!(state.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("slot"),
        patch_json: patch.to_string(),
        page_id: None,
    }));
}

fn reopen(state: &EditorState) -> EditorState {
    let written = serde_json::to_string(&state.doc).unwrap();
    EditorState::from_document(jian_ops_schema::load_str(&written).unwrap().value)
}

fn branch_state(branch: &str, query: Option<&str>, prompt: Option<&str>) -> EditorState {
    let mut state = state(query, prompt);
    if branch != "thumb" {
        patch(&mut state, json!({"width": 375, "height": 240}));
    }
    if branch == "covered" {
        let slot = serde_json::to_value(state.active_children().first().unwrap()).unwrap();
        state = EditorState::from_document(serde_json::from_value(json!({
            "version": "1.0.0", "children": [{
                "type": "frame", "id": "hero-stack", "layout": "none", "width": 375, "height": 240,
                "children": [slot, {"type": "text", "id": "overlay-title", "content": "Workout", "y": 180}]
            }]
        })).unwrap());
    }
    state
}

fn slot_value(state: &EditorState) -> Value {
    serde_json::to_value(walkers::find_node(state.active_children(), &NodeId::new("slot")).unwrap())
        .unwrap()
}

#[test]
fn rich_image_intent_survives_failure_save_reopen_and_repeated_collection() {
    let mut state = state(Some(QUERY), Some(PROMPT));
    let original = target(&state);
    assert_eq!(original.mode, ImageRequestMode::Auto);
    fail(&mut state);
    assert_eq!(target(&state), original);
    let state = reopen(&state);
    let saved = state.doc.clone();
    for _ in 0..2 {
        assert_eq!(target(&state), original);
        assert_eq!(state.doc, saved);
    }
    let node = walkers::find_node(state.active_children(), &NodeId::new("slot")).unwrap();
    assert_eq!(image_request_mode(node), ImageRequestMode::Auto);
    assert!(collect_targets(&state, &HashSet::from(["slot".to_owned()])).is_empty());
}

#[test]
fn generate_only_fallback_keeps_its_mode_and_can_land_the_generated_result() {
    let mut state = state(None, Some("A quiet lake painted in warm earth tones"));
    let original = target(&state);
    assert_eq!(original.mode, ImageRequestMode::Generate);
    assert_eq!(original.query, "Goblet Squat");
    fail(&mut state);
    let mut state = reopen(&state);
    assert_eq!(target(&state), original);
    assert!(apply_result(
        &mut state,
        &NodeId::new("slot"),
        "https://example.test/generated.jpg"
    ));
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
    assert!(collect_targets(&reopen(&state), &HashSet::new()).is_empty());
}

#[test]
fn search_only_fallback_preserves_the_query_only_contract() {
    let mut state = state(Some(QUERY), None);
    let original = target(&state);
    assert_eq!(original.mode, ImageRequestMode::Search);
    fail(&mut state);
    assert_eq!(target(&reopen(&state)), original);
}

#[test]
fn edited_live_query_supersedes_saved_prompt_and_mode() {
    let mut state = state(Some(QUERY), Some(PROMPT));
    fail(&mut state);
    patch(
        &mut state,
        json!({"imageSearchQuery": "seated balance ball exercise"}),
    );
    let changed = target(&reopen(&state));
    assert_eq!(changed.query, "seated balance ball exercise");
    assert_eq!(changed.prompt, None);
    assert_eq!(changed.mode, ImageRequestMode::Search);
    patch(&mut state, json!({"imageSearchQuery": null}));
    assert!(collect_targets(&reopen(&state), &HashSet::new()).is_empty());
}

#[test]
fn cosmetic_query_whitespace_and_renaming_do_not_erase_the_saved_contract() {
    let mut state = state(Some(QUERY), Some(PROMPT));
    fail(&mut state);
    patch(
        &mut state,
        json!({"imageSearchQuery": format!("  {QUERY}  "), "name": "My exercise photo"}),
    );
    let recovered = target(&reopen(&state));
    assert_eq!(recovered.query, QUERY);
    assert_eq!(recovered.prompt.as_deref(), Some(PROMPT));
    assert_eq!(recovered.mode, ImageRequestMode::Auto);
}

fn metadata() -> Value {
    json!({"imageSearchQuery": QUERY, "imagePrompt": PROMPT,
        "imageRequestMode": "auto", "originalExplain": null})
}

#[test]
fn malformed_or_non_policy_metadata_never_restores_a_rich_contract() {
    let valid = metadata();
    let mut bad_payloads = vec![json!([]), json!(true), json!("text")];
    for key in [
        "imageSearchQuery",
        "imagePrompt",
        "imageRequestMode",
        "originalExplain",
    ] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(key);
        bad_payloads.push(missing);
        let mut wrong_type = valid.clone();
        wrong_type[key] = json!(42);
        bad_payloads.push(wrong_type);
    }
    let mut unknown_mode = valid.clone();
    unknown_mode["imageRequestMode"] = json!("automatic");
    bad_payloads.push(unknown_mode);
    let mut explains: Vec<String> = bad_payloads
        .into_iter()
        .map(|payload| format!("image fallback: thumb {payload}"))
        .collect();
    explains.extend([
        "image fallback: thumb {broken".to_owned(),
        format!("image fallback: unknown {valid}"),
        format!("User note: image fallback: thumb {valid}"),
        valid.to_string(),
        format!("image fallback: thumb {valid} extra"),
    ]);
    for explain in explains {
        let mut state = state(Some(QUERY), Some(PROMPT));
        fail(&mut state);
        patch(&mut state, json!({"explain": explain}));
        let before = state.doc.clone();
        let legacy = target(&state);
        assert_eq!(legacy.query, QUERY);
        assert_eq!(legacy.prompt, None, "{explain}");
        assert_eq!(legacy.mode, ImageRequestMode::Search, "{explain}");
        assert_eq!(state.doc, before);
    }
}

#[test]
fn legacy_fallback_without_metadata_keeps_existing_behavior() {
    for explain in [
        Value::Null,
        json!("image fallback: thumb {}"),
        json!("image fallback: thumb"),
    ] {
        let mut state = state(Some(QUERY), Some(PROMPT));
        fail(&mut state);
        patch(&mut state, json!({"explain": explain}));
        let legacy = target(&reopen(&state));
        assert_eq!(legacy.query, QUERY);
        assert_eq!(legacy.prompt, None);
        assert_eq!(legacy.mode, ImageRequestMode::Search);
    }
}

#[test]
fn a_landed_fallback_stays_resolved_even_though_its_envelope_is_retained() {
    let mut state = state(Some(QUERY), Some(PROMPT));
    fail(&mut state);
    assert!(apply_result(
        &mut state,
        &NodeId::new("slot"),
        "https://example.test/goblet-squat.jpg"
    ));
    let state = reopen(&state);
    let before = state.doc.clone();
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
    assert_eq!(state.doc, before);
}

#[test]
fn recovered_intent_does_not_restore_old_slot_geometry() {
    let mut state = state(Some(QUERY), Some(PROMPT));
    let original = target(&state);
    fail(&mut state);
    patch(&mut state, json!({"width": 180, "height": 90}));
    let changed = target(&reopen(&state));
    assert_eq!(changed.prompt, original.prompt);
    assert_eq!(changed.mode, original.mode);
    assert_eq!(changed.width, Some(180.0));
    assert_eq!(changed.height, Some(90.0));
    assert_ne!(changed.judge_view, original.judge_view);
}

#[test]
fn every_fallback_branch_preserves_auto_and_generate_intent_until_success() {
    for branch in ["thumb", "media", "covered"] {
        for query in [Some(QUERY), None] {
            let mut state = branch_state(branch, query, Some(PROMPT));
            let original = target(&state);
            fail(&mut state);
            assert!(slot_value(&state)["explain"]
                .as_str()
                .unwrap()
                .starts_with(&format!("image fallback: {branch} ")));
            let mut state = reopen(&state);
            assert_eq!(target(&state), original);
            assert!(apply_result(
                &mut state,
                &NodeId::new("slot"),
                "https://example.test/result.jpg"
            ));
            assert!(collect_targets(&reopen(&state), &HashSet::new()).is_empty());
        }
    }
}

#[test]
fn newly_recognized_fallbacks_never_swallow_authored_descendants() {
    for branch in ["thumb", "media", "covered"] {
        for prompt in [Some(PROMPT), None] {
            let mut state = branch_state(branch, None, prompt);
            fail(&mut state);
            let mut children = slot_value(&state)["children"].as_array().unwrap().clone();
            children
                .push(json!({"type": "text", "id": "authored-note", "content": "Keep this note"}));
            children.push(
                json!({"type": "image", "id": "nested-image", "name": "Nested lake photo",
                "src": "", "width": 48, "height": 48, "imageSearchQuery": "lake"}),
            );
            patch(&mut state, json!({"children": children}));
            let mut state = reopen(&state);
            let before = state.doc.clone();
            let targets = collect_targets(&state, &HashSet::new());
            assert_eq!(targets.len(), 1, "{branch} {prompt:?}: {targets:?}");
            assert_eq!(targets[0].node_id.as_str(), "nested-image");
            assert!(!apply_result(
                &mut state,
                &NodeId::new("slot"),
                "https://example.test/replacement.jpg"
            ));
            assert_eq!(state.doc, before);
        }
    }
}

#[test]
fn legacy_generated_media_caption_is_allowed_but_edited_caption_is_not() {
    let mut state = branch_state("media", None, Some(PROMPT));
    let original = target(&state);
    fail(&mut state);
    let mut children = slot_value(&state)["children"].clone();
    children[1]["width"] = json!(160);
    children[1]["height"] = json!(16);
    children[1].as_object_mut().unwrap().remove("textGrowth");
    patch(&mut state, json!({"children": children}));
    assert_eq!(target(&reopen(&state)), original);
    children[1]["content"] = json!("My chosen caption");
    patch(&mut state, json!({"children": children}));
    let before = state.doc.clone();
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
    assert!(!apply_result(
        &mut state,
        &NodeId::new("slot"),
        "https://example.test/replacement.jpg"
    ));
    assert_eq!(state.doc, before);
}

#[test]
fn a_landed_image_in_layered_fills_blocks_new_fallback_eligibility() {
    let mut state = branch_state("media", None, Some(PROMPT));
    fail(&mut state);
    patch(
        &mut state,
        json!({"fill": [
            {"type": "solid", "color": "#000000"},
            {"type": "image", "url": "https://example.test/user-photo.jpg"}
        ]}),
    );
    let before = state.doc.clone();
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
    assert!(!apply_result(
        &mut state,
        &NodeId::new("slot"),
        "https://example.test/replacement.jpg"
    ));
    assert_eq!(state.doc, before);
}

#[test]
fn acquisition_modes_require_their_saved_request_field() {
    let mut state = state(None, Some(PROMPT));
    fail(&mut state);
    let mut invalid = metadata();
    invalid["imageSearchQuery"] = Value::Null;
    invalid["imageRequestMode"] = json!("search");
    patch(
        &mut state,
        json!({"explain": format!("image fallback: thumb {invalid}")}),
    );
    assert!(collect_targets(&state, &HashSet::new()).is_empty());

    let mut state = branch_state("thumb", Some(QUERY), None);
    fail(&mut state);
    invalid["imageSearchQuery"] = json!(QUERY);
    invalid["imagePrompt"] = Value::Null;
    invalid["imageRequestMode"] = json!("generate");
    patch(
        &mut state,
        json!({"explain": format!("image fallback: thumb {invalid}")}),
    );
    assert_eq!(target(&state).mode, ImageRequestMode::Search);
}

#[test]
fn resizing_generate_only_tiles_preserves_intent_and_current_geometry() {
    for (branch, width) in [("thumb", 180.0), ("media", 64.0)] {
        let mut state = branch_state(branch, None, Some(PROMPT));
        let original = target(&state);
        fail(&mut state);
        patch(&mut state, json!({"width": width, "height": 90}));
        let mut state = reopen(&state);
        let changed = target(&state);
        assert_eq!(changed.query, original.query);
        assert_eq!(changed.prompt, original.prompt);
        assert_eq!(changed.mode, ImageRequestMode::Generate);
        assert_eq!(changed.width, Some(width));
        assert_eq!(changed.height, Some(90.0));
        assert!(apply_result(
            &mut state,
            &NodeId::new("slot"),
            "https://example.test/generated.jpg"
        ));
        assert!(collect_targets(&state, &HashSet::new()).is_empty());
    }
}

#[test]
fn a_resolved_fill_container_thumb_retains_its_saved_decoration_branch() {
    let mut state = branch_state("thumb", None, Some(PROMPT));
    patch(
        &mut state,
        json!({"width": "fill_container", "src": SEARCH_FAILED_PLACEHOLDER_SRC}),
    );
    let node = walkers::find_node(state.active_children(), &NodeId::new("slot")).unwrap();
    let widths = HashMap::from([("slot".to_string(), 64.0)]);
    let fallback = image_fallback_policy_with_widths(node, &widths, false)
        .into_iter()
        .next()
        .unwrap();
    patch(
        &mut state,
        serde_json::from_str(&fallback.patch_json).unwrap(),
    );
    let mut state = reopen(&state);
    let recovered = target(&state);
    assert_eq!(recovered.mode, ImageRequestMode::Generate);
    assert_eq!(recovered.prompt.as_deref(), Some(PROMPT));
    assert!(apply_result(
        &mut state,
        &NodeId::new("slot"),
        "https://example.test/generated.jpg"
    ));
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
}

#[test]
fn repeated_failed_retries_preserve_every_tile_and_its_saved_intent() {
    for branch in ["thumb", "media", "covered"] {
        for query in [Some(QUERY), None] {
            let mut state = branch_state(branch, query, Some(PROMPT));
            let original = target(&state);
            fail(&mut state);
            for _ in 0..2 {
                let before = state.doc.clone();
                assert!(!apply_result(
                    &mut state,
                    &NodeId::new("slot"),
                    SEARCH_FAILED_PLACEHOLDER_SRC
                ));
                assert_eq!(state.doc, before, "{branch} {query:?}");
                state = reopen(&state);
                assert_eq!(target(&state), original, "{branch} {query:?}");
            }
            assert!(apply_result(
                &mut state,
                &NodeId::new("slot"),
                "https://example.test/recovered.jpg"
            ));
            let resolved = state.doc.clone();
            assert!(!apply_result(
                &mut state,
                &NodeId::new("slot"),
                SEARCH_FAILED_PLACEHOLDER_SRC
            ));
            assert_eq!(state.doc, resolved);
            assert!(collect_targets(&reopen(&state), &HashSet::new()).is_empty());
        }
    }
}
