//! Slot-detection + `apply_result` tests for `image_search_session`.
//! Split out of the flat `image_search_session_tests.rs` to keep every file
//! under the 800-line cap; pure code motion.

use super::super::*;
use super::*;

#[test]
fn collect_targets_prefers_query_on_empty_image_nodes() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "img1");
    assert_eq!(targets[0].query, "burger fries");
}

#[test]
fn collect_targets_infers_image_aspect_ratio() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].aspect_ratio, Some(ImageAspectRatio::Wide));
}

#[test]
fn apply_result_sets_empty_image_src() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));
    let revision_before = state.document_revision();

    assert!(apply_result(
        &mut state,
        &NodeId::new("img1"),
        "https://example.com/photo.jpg"
    ));
    let PenNode::Image(image) = &state.active_children()[0] else {
        panic!("expected image");
    };
    assert_eq!(image.src, "https://example.com/photo.jpg");
    // A content-mutating apply_result bumps the revision so the layer-panel
    // row cache + save-dirty tracking (keyed on `document_revision()`) refresh.
    assert_ne!(
        state.document_revision(),
        revision_before,
        "apply_result that writes content must advance document_revision"
    );
}

#[test]
fn collect_targets_includes_unfilled_placeholder_frames() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame_node(
        "photo",
        "Image",
        Some("image-placeholder"),
        Some(vec![solid_fill()]),
        vec![text_label(
            "label",
            Some("image-placeholder-label"),
            "pizza hero",
        )],
    ));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "pizza hero");
}

#[test]
fn collect_targets_prefers_frame_image_search_query() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    let frame: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "photo",
        "name": "Image",
        "role": "image-placeholder",
        "width": 240,
        "height": 160,
        "fill": [{ "type": "solid", "color": "#E5E7EB" }],
        "imageSearchQuery": "burger fries"
    }))
    .unwrap();
    state.active_children_mut().push(frame);

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "burger fries");
}

#[test]
fn collect_targets_uses_parent_semantic_name_for_generic_heuristic_frame() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    let photo = frame_node("photo", "Image", None, Some(vec![solid_fill()]), Vec::new());
    state
        .active_children_mut()
        .push(frame_node("card", "Bella Italia", None, None, vec![photo]));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "Bella Italia");
}

#[test]
fn collect_targets_includes_solid_rectangle_image_areas() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(rectangle_node(
        "photo",
        "Latte Image",
        Some(vec![solid_fill()]),
    ));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "Latte Image");
}

#[test]
fn collect_targets_includes_fill_width_rectangle_image_areas() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(rectangle_node_with_sizing(
        "photo",
        "Latte Image",
        Some(vec![solid_fill()]),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)),
        Some(SizingBehavior::Number(180.0)),
    ));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "Latte Image");
}

#[test]
fn apply_result_repaints_placeholder_frame_with_image_fill_and_clears_children() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame_node(
        "photo",
        "Image",
        Some("image-placeholder"),
        Some(vec![solid_fill()]),
        vec![text_label(
            "label",
            Some("image-placeholder-label"),
            "pizza hero",
        )],
    ));

    assert!(apply_result(
        &mut state,
        &NodeId::new("photo"),
        "https://example.com/photo.jpg"
    ));
    let PenNode::Frame(frame) = &state.active_children()[0] else {
        panic!("expected frame");
    };
    let Some([PenFill::Image(image_fill)]) = frame.container.fill.as_deref() else {
        panic!("expected single image fill");
    };
    assert_eq!(image_fill.url, "https://example.com/photo.jpg");
    assert_eq!(image_fill.mode, Some(ImageFillMode::Crop));
    assert_eq!(frame.children.as_deref(), Some(&[][..]));
}

#[test]
fn apply_result_repaints_placeholder_rectangle_with_image_fill() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(rectangle_node(
        "photo",
        "Latte Image",
        Some(vec![solid_fill()]),
    ));

    assert!(apply_result(
        &mut state,
        &NodeId::new("photo"),
        "https://example.com/photo.jpg"
    ));
    let PenNode::Rectangle(rect) = &state.active_children()[0] else {
        panic!("expected rectangle");
    };
    let Some([PenFill::Image(image_fill)]) = rect.container.fill.as_deref() else {
        panic!("expected single image fill");
    };
    assert_eq!(image_fill.url, "https://example.com/photo.jpg");
    assert_eq!(image_fill.mode, Some(ImageFillMode::Crop));
}

/// GLM-shaped slot: a rectangle that already carries ONE image fill whose
/// url is still empty. It must be collected (no solid-fill heuristic gate)
/// and, on `apply_result`, keep its authored fill body — only the url lands.
#[test]
fn collect_targets_includes_rect_with_empty_image_fill() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(rectangle_node(
        "photo",
        "Album Cover",
        Some(vec![image_fill("")]),
    ));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "Album Cover");
}

#[test]
fn collect_targets_skips_rect_with_landed_image_fill() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(rectangle_node(
        "photo",
        "Album Cover",
        Some(vec![image_fill("https://example.com/photo.jpg")]),
    ));

    let targets = collect_targets(&state, &HashSet::new());

    assert!(
        targets.is_empty(),
        "a landed image fill is already filled: {targets:?}"
    );
}

/// Regression for the placeholder-frame detector: an image fill with an
/// EMPTY url is "still unfilled" — before the fix it read as done.
#[test]
fn collect_targets_includes_placeholder_frame_with_empty_image_fill() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame_node(
        "photo",
        "Image",
        Some("image-placeholder"),
        Some(vec![image_fill("")]),
        vec![text_label(
            "label",
            Some("image-placeholder-label"),
            "pizza hero",
        )],
    ));

    let targets = collect_targets(&state, &HashSet::new());

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
    assert_eq!(targets[0].query, "pizza hero");
}

#[test]
fn apply_result_overwrites_only_url_of_empty_image_fill_rectangle() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    // A body with distinctive authored fields: everything except the url
    // must survive the write-back.
    state.active_children_mut().push(rectangle_node(
        "photo",
        "Album Cover",
        Some(vec![PenFill::Image(ImageFillBody {
            url: "".into(),
            mode: Some(ImageFillMode::Fill),
            original_size: None,
            transform: None,
            tile_scale: Some(2.0),
            explain: None,
            opacity: Some(0.75),
            blend_mode: None,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
        })]),
    ));
    let revision_before = state.document_revision();

    assert!(apply_result(
        &mut state,
        &NodeId::new("photo"),
        "https://example.com/photo.jpg"
    ));
    let PenNode::Rectangle(rect) = &state.active_children()[0] else {
        panic!("write-back must keep the rectangle node kind");
    };
    let Some([PenFill::Image(image_fill)]) = rect.container.fill.as_deref() else {
        panic!("expected the single image fill");
    };
    assert_eq!(image_fill.url, "https://example.com/photo.jpg");
    assert_eq!(image_fill.mode, Some(ImageFillMode::Fill));
    assert_eq!(image_fill.tile_scale, Some(2.0));
    assert_eq!(image_fill.opacity, Some(0.75));
    assert_ne!(
        state.document_revision(),
        revision_before,
        "a content-mutating apply_result must advance document_revision"
    );
}

#[test]
fn failed_search_writes_the_adaptive_placeholder_sentinel() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("nonexistent subject")));

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(None).unwrap();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["img1".to_string()]),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("img1"),
            intent: None,
            rx,
        }],
        ..Default::default()
    };

    assert!(session.poll_into(&mut state));
    let PenNode::Image(image) = &state.active_children()[0] else {
        panic!("expected image");
    };
    assert_eq!(image.src, SEARCH_FAILED_PLACEHOLDER_SRC);
    assert!(
        session.completed.contains("img1"),
        "failed slot must not re-enqueue this session"
    );
}

/// test0711-2-ds shape: "Mini Player" holds a bare unnamed 44×44 solid
/// rectangle as the artwork slot — the name-keyword gate must work off the
/// ANCESTOR chain so the anonymous slot still enriches.
#[test]
fn unnamed_square_slot_inside_media_named_parent_is_a_target() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame_node(
        "player",
        "Mini Player",
        None,
        Some(vec![solid_fill()]),
        vec![
            rectangle_node_with_sizing(
                "art",
                "",
                Some(vec![solid_fill()]),
                Some(SizingBehavior::Number(44.0)),
                Some(SizingBehavior::Number(44.0)),
            ),
            text_label("title", None, "Blinding Lights"),
        ],
    ));

    let targets = collect_targets(&state, &HashSet::new());
    assert!(
        targets.iter().any(|t| t.node_id.as_str() == "art"),
        "anonymous 44px art slot must enrich: {targets:?}"
    );

    // The same bare rectangle OUTSIDE any media context stays untouched.
    let mut plain = EditorState::default();
    plain.active_children_mut().clear();
    plain.active_children_mut().push(frame_node(
        "box",
        "Stats Row",
        None,
        Some(vec![solid_fill()]),
        vec![rectangle_node_with_sizing(
            "chip",
            "",
            Some(vec![solid_fill()]),
            Some(SizingBehavior::Number(44.0)),
            Some(SizingBehavior::Number(44.0)),
        )],
    ));
    let targets = collect_targets(&plain, &HashSet::new());
    assert!(
        targets.iter().all(|t| t.node_id.as_str() != "chip"),
        "no media context, no enrichment: {targets:?}"
    );
}

/// DeepSeek V4 shape (test0711-2-ds): whole album grid of NAMELESS empty
/// solid squares with no G() bindings — the sibling text ("Blinding
/// Lights") is the subject source. A slot with no text siblings stays out.
#[test]
fn anonymous_cover_slot_uses_sibling_text_as_query() {
    let mut card = frame_node(
        "card",
        "",
        None,
        None,
        vec![
            {
                let mut slot = frame_node("slot", "", None, Some(vec![solid_fill()]), vec![]);
                if let PenNode::Frame(f) = &mut slot {
                    f.base.name = None;
                    f.container.width = Some(SizingBehavior::Number(120.0));
                    f.container.height = Some(SizingBehavior::Number(120.0));
                    f.container.clip_content = Some(true);
                }
                slot
            },
            text_label("t1", None, "Blinding Lights"),
            text_label("t2", None, "The Weeknd"),
        ],
    );
    if let PenNode::Frame(f) = &mut card {
        f.base.name = None;
    }
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(card);

    let targets = collect_targets(&state, &HashSet::new());
    let slot = targets
        .iter()
        .find(|t| t.node_id.as_str() == "slot")
        .expect("anonymous slot becomes a target");
    assert!(
        slot.query.to_lowercase().contains("blinding lights"),
        "query derives from sibling text: {}",
        slot.query
    );
}

#[test]
fn anonymous_slot_does_not_borrow_text_from_a_cousin_card() {
    let mut slot = frame_node("slot", "", None, Some(vec![solid_fill()]), vec![]);
    if let PenNode::Frame(frame) = &mut slot {
        frame.base.name = None;
        frame.container.width = Some(SizingBehavior::Number(120.0));
        frame.container.height = Some(SizingBehavior::Number(120.0));
        frame.container.clip_content = Some(true);
    }
    let mut empty_card = frame_node("empty-card", "", None, None, vec![slot]);
    if let PenNode::Frame(frame) = &mut empty_card {
        frame.base.name = None;
    }
    let other_card = frame_node(
        "other-card",
        "Santorini Card",
        None,
        None,
        vec![frame_node(
            "other-info",
            "Info",
            None,
            None,
            vec![text_label("other-title", None, "Santorini, Greece")],
        )],
    );
    let rail = frame_node(
        "rail",
        "Destination Rail",
        None,
        None,
        vec![empty_card, other_card],
    );
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(rail);

    let targets = collect_targets(&state, &HashSet::new());
    assert!(
        targets
            .iter()
            .all(|target| target.node_id.as_str() != "slot"),
        "an unlabelled card must stay empty, not borrow its cousin's title: {targets:?}"
    );
}

#[test]
fn rounded_kpi_tile_is_not_an_anonymous_image_slot() {
    let mut tile = frame_node("tile", "", None, Some(vec![solid_fill()]), vec![]);
    if let PenNode::Frame(frame) = &mut tile {
        frame.base.name = None;
        frame.container.width = Some(SizingBehavior::Number(64.0));
        frame.container.height = Some(SizingBehavior::Number(64.0));
        frame.container.corner_radius = Some(jian_ops_schema::node::CornerRadius::Uniform(16.0));
    }
    let card = frame_node(
        "kpi",
        "Revenue KPI Card",
        None,
        None,
        vec![tile, text_label("label", None, "Monthly revenue")],
    );
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(card);

    let targets = collect_targets(&state, &HashSet::new());
    assert!(
        targets
            .iter()
            .all(|target| target.node_id.as_str() != "tile"),
        "rounded KPI geometry plus a label is not media intent: {targets:?}"
    );
}

/// The measured churn: the model rebuilds a section mid-run, the same subject
/// searches again, and the session-wide dedup skips the very photo it picked
/// the first time — so a real Bali temple photo became a plain blue sky. One
/// subject, one photo: a repeat query resolves from the memo, and the dedup
/// only ever guards DIFFERENT subjects from sharing a picture.
#[test]
fn g_style_fill_image_uses_resolved_parent_slot_aspect() {
    let state_for_slot = |width: f64, height: f64| {
        let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
            "version":"1.0", "children":[{
                "type":"frame", "id":"root", "width":600, "height":400, "layout":"vertical",
                "children":[{
                    "type":"frame", "id":"slot", "name":"Bali Hero", "width":width,
                    "height":height, "layout":"vertical", "clipContent":true, "children":[{
                        "type":"image", "id":"photo", "name":"Bali Indonesia", "src":"",
                        "imagePrompt":"Bali Indonesia", "width":"fill_container",
                        "height":"fill_container", "objectFit":"crop"
                    }]
                }]
            }]
        }))
        .expect("G-shaped document");
        EditorState::from_document(doc)
    };

    let wide = collect_targets(&state_for_slot(320.0, 180.0), &HashSet::new())
        .into_iter()
        .find(|target| target.node_id.as_str() == "photo")
        .expect("wide image target");
    let square = collect_targets(&state_for_slot(180.0, 180.0), &HashSet::new())
        .into_iter()
        .find(|target| target.node_id.as_str() == "photo")
        .expect("square image target");

    assert_eq!(wide.aspect_ratio, Some(ImageAspectRatio::Wide));
    assert_eq!(square.aspect_ratio, Some(ImageAspectRatio::Square));
    assert_eq!((wide.width, wide.height), (Some(320.0), Some(180.0)));
    assert_ne!(
        intent_fingerprint(&wide, None),
        intent_fingerprint(&square, None),
        "a parent-slot aspect change invalidates the in-flight search"
    );
}

/// MiniMax-M3 builds every card around a RECTANGLE named "img" (or a "ph"
/// rectangle inside a frame named "img") — neither word was in the keyword
/// table, so a whole page of destination cards shipped as grey boxes with no
/// images at all (measured test0711-1-m3, 2026-07-12).
#[test]
fn m3_style_img_and_ph_rectangles_are_image_slots() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "card", "name": "Santorini",
                  "width": "fill_container", "height": "fit_content", "layout": "vertical",
                  "children": [
                    { "type": "frame", "id": "wrap", "name": "img",
                      "width": 180, "height": 140, "layout": "none", "children": [
                        { "type": "rectangle", "id": "ph", "name": "ph",
                          "width": 180, "height": 140,
                          "fill": [{ "type": "solid", "color": "#E5E5E5" }] }
                      ]},
                    { "type": "text", "id": "t", "content": "Santorini" }
                  ]},
                { "type": "frame", "id": "deal", "name": "Bali, Indonesia",
                  "width": "fill_container", "height": "fit_content", "layout": "vertical",
                  "children": [
                    { "type": "rectangle", "id": "img", "name": "img",
                      "width": 165, "height": 130,
                      "fill": [{ "type": "solid", "color": "#E5E5E5" }] }
                  ]}
            ]
        }] }"##,
    )
    .expect("parse");
    let state = op_editor_core::EditorState::from_document(doc);
    let targets = super::super::collect_targets(&state, &std::collections::HashSet::new());

    let by_id: std::collections::HashMap<&str, &str> = targets
        .iter()
        .map(|t| (t.node_id.as_str(), t.query.as_str()))
        .collect();
    assert!(
        by_id.contains_key("ph"),
        "a \"ph\" rectangle in an \"img\" wrapper is an image slot: {by_id:?}"
    );
    assert!(
        by_id.contains_key("img"),
        "a rectangle literally named \"img\" is an image slot: {by_id:?}"
    );
    assert_eq!(
        by_id.get("ph"),
        Some(&"Santorini"),
        "the slot carries no subject — the CARD names the picture"
    );
    assert_eq!(by_id.get("img"), Some(&"Bali, Indonesia"));
}

/// Geometry alone cannot distinguish a photo slot from a chart, swatch, or
/// decorative surface. The in-loop diagnostic can ask the model about it, but
/// background enrichment requires explicit media semantics.
#[test]
fn an_unnamed_fill_container_rectangle_is_not_auto_filled_from_geometry() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "card", "name": "Card",
                "width": 200, "height": 260, "layout": "vertical",
                "children": [
                    { "type": "frame", "id": "band", "width": "fill_container", "height": 140,
                      "layout": "vertical", "children": [
                        { "type": "rectangle", "id": "slot",
                          "width": "fill_container", "height": "fill_container",
                          "fill": [{ "type": "solid", "color": "#F1F1F1" }] }
                      ]},
                    { "type": "text", "id": "title", "content": "Santorini" },
                    { "type": "text", "id": "sub", "content": "Greece" }
                ]
            }]
        }] }"##,
    )
    .expect("parse");
    let state = op_editor_core::EditorState::from_document(doc);
    let targets = super::super::collect_targets(&state, &std::collections::HashSet::new());
    assert!(
        targets
            .iter()
            .all(|target| target.node_id.as_str() != "slot"),
        "an unnamed solid box needs an explicit role/name/query: {targets:?}"
    );
}

#[test]
fn a_thin_divider_rectangle_is_not_an_image_slot() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "row", "width": "fill_container", "height": 60,
                "layout": "vertical", "children": [
                    { "type": "text", "id": "t", "content": "Section" },
                    { "type": "rectangle", "id": "divider", "name": "divider",
                      "width": "fill_container", "height": 1,
                      "fill": [{ "type": "solid", "color": "#E5E5E5" }] }
                ]
            }]
        }] }"##,
    )
    .expect("parse");
    let state = op_editor_core::EditorState::from_document(doc);
    let targets = super::super::collect_targets(&state, &std::collections::HashSet::new());
    assert!(
        !targets.iter().any(|t| t.node_id.as_str() == "divider"),
        "a 1px rule is not a photo: {targets:?}"
    );
}

#[test]
fn image_fields_preserve_search_generate_and_legacy_auto_modes() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version":"1.0", "children":[{
            "type":"frame", "id":"root", "width":390, "height":844, "layout":"vertical",
            "children":[
                {"type":"image", "id":"search", "name":"Search Photo", "src":"",
                 "imageSearchQuery":"Kyoto temple", "width":160, "height":90},
                {"type":"image", "id":"generate", "name":"Generated Art", "src":"",
                 "imagePrompt":"surreal Kyoto at dusk", "width":160, "height":90},
                {"type":"image", "id":"legacy-auto", "name":"Compatible Art", "src":"",
                 "imageSearchQuery":"Kyoto dusk", "imagePrompt":"surreal Kyoto at dusk",
                 "width":160, "height":90}
            ]
        }] }"##,
    )
    .expect("parse");
    let state = op_editor_core::EditorState::from_document(doc);
    let targets = collect_targets(&state, &std::collections::HashSet::new());

    assert_eq!(
        targets
            .iter()
            .find(|target| target.node_id.as_str() == "search")
            .expect("search target")
            .mode,
        ImageRequestMode::Search
    );
    assert_eq!(
        targets
            .iter()
            .find(|target| target.node_id.as_str() == "generate")
            .expect("generate target")
            .mode,
        ImageRequestMode::Generate
    );
    assert_eq!(
        targets
            .iter()
            .find(|target| target.node_id.as_str() == "legacy-auto")
            .expect("legacy auto target")
            .mode,
        ImageRequestMode::Auto
    );
}

/// Forensic: `OP_SLOT_PROBE=<path.op>` prints every slot the enrichment pass
/// would enqueue for a saved document, with the query it would search.
#[test]
#[ignore]
fn slot_probe() {
    let Ok(path) = std::env::var("OP_SLOT_PROBE") else {
        return;
    };
    let src = std::fs::read_to_string(&path).expect("read");
    let loaded = op_pen_loader::load_canonical(&src).expect("load");
    let state = op_editor_core::EditorState::from_document(loaded.value);
    let targets = super::super::collect_targets(&state, &std::collections::HashSet::new());
    eprintln!("SLOTS ({}):", targets.len());
    for t in &targets {
        eprintln!("  {} -> {:?}", t.node_id.as_str(), t.query);
    }
}
