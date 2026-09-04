use std::collections::HashSet;

use op_editor_core::EditorState;
use op_image_enrich::{apply_result, collect_targets};

fn state_from_children(children: serde_json::Value) -> EditorState {
    let document = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": children,
    }))
    .expect("valid test document");
    EditorState::from_document(document)
}

#[test]
fn visible_icon_tiles_are_not_inferred_as_image_slots_from_media_names() {
    let state = state_from_children(serde_json::json!([
        {
            "type": "frame",
            "id": "lighting",
            "name": "Lighting art",
            "width": 64,
            "height": 64,
            "fill": [{ "type": "solid", "color": "#F5EAD8" }],
            "children": [{
                "type": "icon_font",
                "id": "lighting-icon",
                "iconFontName": "lamp",
                "width": 24,
                "height": 24
            }]
        },
        {
            "type": "frame",
            "id": "decor",
            "name": "Decor media",
            "width": 64,
            "height": 64,
            "fill": [{ "type": "solid", "color": "#E8EFE9" }],
            "children": [{
                "type": "icon_font",
                "id": "decor-icon",
                "iconFontName": "sparkles",
                "width": 24,
                "height": 24
            }]
        },
        {
            "type": "rectangle",
            "id": "furniture",
            "name": "Furniture image",
            "width": 96,
            "height": 96,
            "fill": [{ "type": "solid", "color": "#EEEAF5" }],
            "children": [{
                "type": "icon_font",
                "id": "furniture-icon",
                "iconFontName": "armchair",
                "width": 24,
                "height": 24
            }]
        }
    ]));

    let targets = collect_targets(&state, &HashSet::new());
    assert!(
        targets.is_empty(),
        "authored category glyphs must not be replaced by searched images: {targets:?}"
    );
}

#[test]
fn explicit_frame_image_intent_overrides_the_visible_icon_guard() {
    let mut state = state_from_children(serde_json::json!([
        {
            "type": "frame",
            "id": "query-slot",
            "name": "Lighting art",
            "imageSearchQuery": "warm pendant lamps",
            "width": 96,
            "height": 96,
            "fill": [{ "type": "solid", "color": "#F5EAD8" }],
            "children": [{
                "type": "icon_font",
                "id": "query-icon",
                "iconFontName": "image",
                "width": 24,
                "height": 24
            }]
        },
        {
            "type": "frame",
            "id": "role-slot",
            "name": "Decor media",
            "role": "image-placeholder",
            "width": 96,
            "height": 96,
            "fill": [{ "type": "solid", "color": "#E8EFE9" }],
            "children": [{
                "type": "icon_font",
                "id": "role-icon",
                "iconFontName": "image",
                "width": 24,
                "height": 24
            }]
        }
    ]));

    let targets = collect_targets(&state, &HashSet::new());
    assert_eq!(targets.len(), 2, "explicit intent still wins: {targets:?}");
    assert_eq!(
        targets
            .iter()
            .find(|target| target.node_id.as_str() == "query-slot")
            .expect("query-bound slot")
            .query,
        "warm pendant lamps"
    );
    assert!(targets
        .iter()
        .any(|target| target.node_id.as_str() == "role-slot"));

    assert!(apply_result(
        &mut state,
        &op_editor_core::NodeId::new("query-slot"),
        "https://example.com/pendant.jpg"
    ));
    assert!(
        collect_targets(&state, &HashSet::new())
            .iter()
            .all(|target| target.node_id.as_str() != "query-slot"),
        "a landed explicit-query frame must not remain unresolved"
    );
}

#[test]
fn hidden_icon_does_not_mask_an_otherwise_empty_named_slot() {
    let state = state_from_children(serde_json::json!([{
        "type": "frame",
        "id": "album-art",
        "name": "Album art",
        "width": 64,
        "height": 64,
        "fill": [{ "type": "solid", "color": "#E5E7EB" }],
        "children": [{
            "type": "icon_font",
            "id": "hidden-icon",
            "iconFontName": "image",
            "visible": false,
            "width": 24,
            "height": 24
        }]
    }]));

    let targets = collect_targets(&state, &HashSet::new());
    assert_eq!(targets.len(), 1, "hidden chrome is not visible content");
    assert_eq!(targets[0].node_id.as_str(), "album-art");
}
