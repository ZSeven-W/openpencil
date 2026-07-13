use super::*;
use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::{
    ContainerProps, FrameNode, ImageNode, PenNode, RectangleNode, TextContent, TextNode,
};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{ImageFillMode, PenFill, SolidFillBody};

fn image_node(id: &str, src: &str, query: Option<&str>) -> PenNode {
    PenNode::Image(ImageNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some("Menu photo".into()),
            ..Default::default()
        },
        src: src.into(),
        object_fit: None,
        width: Some(SizingBehavior::Number(240.0)),
        height: Some(SizingBehavior::Number(160.0)),
        corner_radius: None,
        effects: None,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
        image_prompt: None,
        image_search_query: query.map(str::to_string),
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn text_label(id: &str, role: Option<&str>, content: &str) -> PenNode {
    PenNode::Text(TextNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some("Label".into()),
            role: role.map(str::to_string),
            ..Default::default()
        },
        width: Some(SizingBehavior::Number(160.0)),
        height: Some(SizingBehavior::Number(24.0)),
        content: TextContent::Plain(content.to_string()),
        font_family: None,
        font_size: None,
        font_weight: None,
        font_style: None,
        letter_spacing: None,
        line_height: None,
        text_align: None,
        text_align_vertical: None,
        text_growth: None,
        underline: None,
        strikethrough: None,
        fill: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn frame_node(
    id: &str,
    name: &str,
    role: Option<&str>,
    fill: Option<Vec<PenFill>>,
    children: Vec<PenNode>,
) -> PenNode {
    PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.into()),
            role: role.map(str::to_string),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(240.0)),
            height: Some(SizingBehavior::Number(160.0)),
            fill,
            ..Default::default()
        },
        children: Some(children),
        image_search_query: None,
        reusable: None,
        screen: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn rectangle_node(id: &str, name: &str, fill: Option<Vec<PenFill>>) -> PenNode {
    rectangle_node_with_sizing(
        id,
        name,
        fill,
        Some(SizingBehavior::Number(240.0)),
        Some(SizingBehavior::Number(160.0)),
    )
}

fn rectangle_node_with_sizing(
    id: &str,
    name: &str,
    fill: Option<Vec<PenFill>>,
    width: Option<SizingBehavior>,
    height: Option<SizingBehavior>,
) -> PenNode {
    PenNode::Rectangle(RectangleNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.into()),
            ..Default::default()
        },
        container: ContainerProps {
            width,
            height,
            fill,
            ..Default::default()
        },
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn solid_fill() -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: "#E5E7EB".into(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

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
fn openverse_search_url_includes_aspect_ratio() {
    let url = openverse_search_url("burger fries", Some(ImageAspectRatio::Square))
        .expect("valid openverse url");

    assert_eq!(
        url.query_pairs()
            .find(|(key, _)| key == "aspect_ratio")
            .map(|(_, value)| value.into_owned()),
        Some("square".to_string())
    );
}

#[test]
fn openverse_search_url_simplifies_verbose_ai_prompt_like_ts() {
    let url = openverse_search_url("a beautiful photo of the sunset on the beach", None)
        .expect("valid openverse url");

    assert_eq!(
        url.query_pairs()
            .find(|(key, _)| key == "q")
            .map(|(_, value)| value.into_owned()),
        Some("beautiful photo sunset beach".to_string())
    );
}

#[test]
fn openverse_search_url_limits_simplified_query_to_four_keywords_like_ts() {
    let url = openverse_search_url(
        "modern office workspace natural lighting wooden desk plants",
        None,
    )
    .expect("valid openverse url");

    assert_eq!(
        url.query_pairs()
            .find(|(key, _)| key == "q")
            .map(|(_, value)| value.into_owned()),
        Some("modern office workspace natural".to_string())
    );
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

#[test]
fn poll_into_applies_finished_job_to_placeholder_frame() {
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

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Some("https://example.com/photo.jpg".to_string()))
        .unwrap();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["photo".to_string()]),
        completed: HashSet::new(),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("photo"),
            intent: None,
            rx,
        }],
        ..Default::default()
    };

    assert!(session.poll_into(&mut state));
    assert!(!session.is_pending());
    assert!(session.in_flight.is_empty());
    assert!(!session.completed.contains("photo"));

    let PenNode::Frame(frame) = &state.active_children()[0] else {
        panic!("expected frame");
    };
    let Some([PenFill::Image(image_fill)]) = frame.container.fill.as_deref() else {
        panic!("expected single image fill");
    };
    assert_eq!(image_fill.url, "https://example.com/photo.jpg");
    assert_eq!(frame.children.as_deref(), Some(&[][..]));
}

#[test]
fn successful_apply_does_not_suppress_later_unfilled_retry() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state.active_children_mut().push(rectangle_node(
        "photo",
        "Latte Image",
        Some(vec![solid_fill()]),
    ));

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Some("https://example.com/photo.jpg".to_string()))
        .unwrap();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["photo".to_string()]),
        completed: HashSet::new(),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("photo"),
            intent: None,
            rx,
        }],
        ..Default::default()
    };

    assert!(session.poll_into(&mut state));

    let PenNode::Rectangle(rect) = &mut state.active_children_mut()[0] else {
        panic!("expected rectangle");
    };
    rect.container.fill = Some(vec![solid_fill()]);

    let mut known = session.completed.clone();
    known.extend(session.in_flight.iter().cloned());
    let targets = collect_targets(&state, &known);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].node_id.as_str(), "photo");
}

#[test]
fn enqueue_missing_skips_second_walk_when_document_unchanged() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    let mut session = ImageSearchSession::new();

    assert!(session.enqueue_missing(&state));
    // Document revision hasn't changed since the first scan (no
    // `mark_document_changed` call happened), so the second call must
    // short-circuit on the revision gate instead of re-walking the tree.
    session.enqueue_missing(&state);

    assert_eq!(session.scan_count, 1);
}

#[test]
fn poll_into_completion_invalidates_scan_gate() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    // A completed (here: failed) job mutates `in_flight`/`completed`, which
    // must force one rescan on the next `enqueue_missing` even though the
    // document revision did not change.
    let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
    tx.send(None).unwrap();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["img1".to_string()]),
        completed: HashSet::new(),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("img1"),
            intent: None,
            rx,
        }],
        ..Default::default()
    };

    session.enqueue_missing(&state);
    assert_eq!(session.scan_count, 1);
    session.enqueue_missing(&state);
    assert_eq!(
        session.scan_count, 1,
        "gate must hold while nothing changed"
    );

    session.poll_into(&mut state);

    session.enqueue_missing(&state);
    assert_eq!(session.scan_count, 2, "completion must force one rescan");
    session.enqueue_missing(&state);
    assert_eq!(
        session.scan_count, 2,
        "gate re-arms after the forced rescan"
    );
}

#[test]
fn poll_discards_a_result_when_the_nodes_image_intent_changed() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));
    let original = collect_targets(&state, &HashSet::new())
        .into_iter()
        .next()
        .expect("original target");
    let expected = intent_fingerprint(&original, None);

    let PenNode::Image(image) = &mut state.active_children_mut()[0] else {
        panic!("image")
    };
    image.image_search_query = Some("latte cup".into());
    state.mark_document_changed();

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Some("https://stale.example.com/burger.jpg".to_string()))
        .unwrap();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["img1".to_string()]),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("img1"),
            intent: Some(expected),
            rx,
        }],
        ..Default::default()
    };

    assert!(!session.poll_into(&mut state));
    let PenNode::Image(image) = &state.active_children()[0] else {
        panic!("image")
    };
    assert!(image.src.is_empty(), "stale URL must not land");
    assert!(!session.completed.contains("img1"));
}

#[test]
fn poll_discards_a_result_when_only_provider_truncated_words_changed() {
    let before = "santorini greece white buildings blue dome";
    let after = "santorini greece white buildings sunset beach";
    assert_eq!(
        simplify_search_query(before),
        simplify_search_query(after),
        "the provider intentionally sees the same four-keyword request"
    );

    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some(before)));
    let original = collect_targets(&state, &HashSet::new())
        .into_iter()
        .next()
        .expect("original target");
    let expected = intent_fingerprint(&original, None);

    let PenNode::Image(image) = &mut state.active_children_mut()[0] else {
        panic!("image")
    };
    image.image_search_query = Some(after.into());
    state.mark_document_changed();

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Some("https://stale.example.com/blue-dome.jpg".to_string()))
        .unwrap();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["img1".to_string()]),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("img1"),
            intent: Some(expected),
            rx,
        }],
        ..Default::default()
    };

    assert!(!session.poll_into(&mut state));
    let PenNode::Image(image) = &state.active_children()[0] else {
        panic!("image")
    };
    assert!(
        image.src.is_empty(),
        "the provider-level collision must not weaken authored intent identity"
    );
}

#[test]
fn reset_invalidates_scan_gate() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    let mut session = ImageSearchSession::new();
    session.enqueue_missing(&state);
    session.enqueue_missing(&state);
    assert_eq!(session.scan_count, 1);

    session.reset();

    session.enqueue_missing(&state);
    assert_eq!(session.scan_count, 2);
}

#[test]
fn enqueue_missing_rewalks_after_document_revision_changes() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    let mut session = ImageSearchSession::new();
    assert!(session.enqueue_missing(&state));

    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img2", "", Some("latte cup")));
    state.mark_document_changed();

    assert!(session.enqueue_missing(&state));
    assert_eq!(session.scan_count, 2);
}

#[test]
fn enqueue_missing_rewalks_after_active_page_changes_even_with_same_revision() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    let mut session = ImageSearchSession::new();
    assert!(session.enqueue_missing(&state));
    assert_eq!(session.scan_count, 1);

    // `add_page` switches `ui.active_page_index` to the freshly created page
    // WITHOUT calling `mark_document_changed` (see
    // `op-editor-core/src/page_mutators.rs`), so `document_revision()` stays
    // put. `enqueue_missing` only walks the active page, so the gate must
    // still treat this as "unscanned" even though the revision looks
    // unchanged, or the new page's placeholders would never be found.
    let revision_before_switch = state.document_revision();
    state.add_page().expect("add_page should succeed");
    assert_eq!(
        state.document_revision(),
        revision_before_switch,
        "page switch must not bump document_revision (that's the bug under test)"
    );
    state
        .active_children_mut()
        .push(image_node("img2", "", Some("latte cup")));

    assert!(
        session.enqueue_missing(&state),
        "switching to a new, unscanned page must force a rescan"
    );
    assert_eq!(session.scan_count, 2);

    // Same page again, nothing changed — gate holds.
    session.enqueue_missing(&state);
    assert_eq!(session.scan_count, 2);
}

#[test]
fn enqueue_missing_rescans_after_invalidate_scan_gate_despite_revision_page_aliasing() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));

    let mut session = ImageSearchSession::new();
    assert!(session.enqueue_missing(&state));
    assert_eq!(session.scan_count, 1);

    // Simulate an MCP `ReplaceDocument`: `EditorState::replace_document`
    // resets `revision`/`revision_counter` to 0 and installs a fresh
    // `UiDraftState` (active_page_index back to 0), so a brand-new,
    // never-scanned document can carry the EXACT same `(revision, page)`
    // key as the document that was scanned before the replace.
    let mut replaced = EditorState::default();
    replaced.active_children_mut().clear();
    replaced
        .active_children_mut()
        .push(image_node("img2", "", Some("latte cup")));
    assert_eq!(
        (replaced.document_revision(), replaced.ui.active_page_index),
        (state.document_revision(), state.ui.active_page_index),
        "the replacement document must alias the pre-replace gate key"
    );

    // Without an explicit invalidation, the gate wrongly stays shut — this is
    // the aliasing hazard `reset()` exists to close. This test exercises the
    // private `invalidate_scan_gate()` primitive directly to document that
    // hazard in isolation; production replacement call sites (desktop MCP
    // pump / Figma import) call `reset()` instead, which clears
    // `in_flight`/`completed`/`jobs` too and invalidates the gate as its
    // last step (see `document_replacement_reset_*` tests below for the
    // stale-set hazards a gate-only invalidation would miss).
    assert!(!session.enqueue_missing(&replaced));
    assert_eq!(
        session.scan_count, 1,
        "gate aliases on the same (revision, page) key across a document replace"
    );

    session.invalidate_scan_gate();

    assert!(session.enqueue_missing(&replaced));
    assert_eq!(session.scan_count, 2, "invalidation forces the rescan");
}

#[test]
fn openverse_credentials_require_both_fields() {
    let mut state = EditorState::default();
    assert!(OpenverseCredentials::from_state(&state).is_none());

    state.editor_ui.agent_settings.openverse_client_id = " client ".into();
    assert!(OpenverseCredentials::from_state(&state).is_none());

    state.editor_ui.agent_settings.openverse_client_secret = " secret ".into();
    let credentials = OpenverseCredentials::from_state(&state).expect("complete credentials");
    assert_eq!(credentials.client_id, "client");
    assert_eq!(credentials.client_secret, "secret");
}

#[test]
fn image_bytes_to_data_url_encodes_canvas_renderable_src() {
    let src = image_bytes_to_data_url("image/png; charset=binary", b"ABC")
        .expect("png bytes should encode");

    assert_eq!(src, "data:image/png;base64,QUJD");
}

#[test]
fn image_bytes_to_data_url_normalizes_jpg_mime_alias() {
    let src = image_bytes_to_data_url("image/jpg", b"ABC").expect("jpg alias should encode");

    assert_eq!(src, "data:image/jpeg;base64,QUJD");
}

#[test]
fn image_bytes_to_data_url_rejects_svg_payloads() {
    assert!(image_bytes_to_data_url("image/svg+xml", b"<svg></svg>").is_none());
}

#[test]
fn sniff_image_mime_detects_common_raster_formats() {
    assert_eq!(
        sniff_image_mime(b"\x89PNG\r\n\x1A\nrest"),
        Some("image/png")
    );
    assert_eq!(sniff_image_mime(b"\xFF\xD8\xFFrest"), Some("image/jpeg"));
    assert_eq!(sniff_image_mime(b"GIF89arest"), Some("image/gif"));
    assert_eq!(sniff_image_mime(b"RIFFxxxxWEBPrest"), Some("image/webp"));
    assert_eq!(sniff_image_mime(b"<svg></svg>"), None);
}

#[test]
fn document_replacement_reset_drops_stale_pending_job_so_it_cannot_apply_to_new_document() {
    // Simulate: the pre-replacement document had a pending image-search job for
    // node id "photo".
    let (tx, rx) = std::sync::mpsc::channel();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["photo".to_string()]),
        completed: HashSet::new(),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("photo"),
            intent: None,
            rx,
        }],
        ..Default::default()
    };

    // Document replacement happens here (Figma import / MCP ReplaceDocument) while
    // the job above is still in flight. Only invalidating the scan gate — NOT
    // resetting the session — must NOT be enough to protect the new document.
    session.reset();

    // Replacement document reuses the SAME node id ("photo") as an unrelated,
    // still-unfilled placeholder — plausible because both replacement paths install
    // a fresh, small id space that can collide with ids from the prior document.
    let mut new_state = EditorState::default();
    new_state.active_children_mut().clear();
    new_state.active_children_mut().push(rectangle_node(
        "photo",
        "Latte Image",
        Some(vec![solid_fill()]),
    ));

    // The stale job resolves AFTER the replacement — its result must not land on
    // the new document's unrelated "photo" node.
    let _ = tx.send(Some(
        "https://stale.example.com/old-document-photo.jpg".to_string(),
    ));

    let changed = session.poll_into(&mut new_state);

    assert!(
        !changed,
        "a job queued before document replacement must not mutate the replacement document"
    );
    assert!(!session.is_pending());
    let PenNode::Rectangle(rect) = &new_state.active_children()[0] else {
        panic!("expected rectangle");
    };
    assert_eq!(
        rect.container.fill.as_deref(),
        Some([solid_fill()].as_slice()),
        "replacement document's placeholder must remain untouched by the stale job"
    );
}

#[test]
fn document_replacement_reset_clears_completed_ids_so_replacement_targets_are_not_suppressed() {
    // Simulate: the pre-replacement document had already resolved (or given up on)
    // node id "photo", so it sits in `completed`.
    let mut session = ImageSearchSession {
        completed: HashSet::from(["photo".to_string()]),
        ..Default::default()
    };

    // Document replacement happens here. Only invalidating the scan gate — NOT
    // resetting the session — must NOT be enough: `completed` would still suppress
    // any node reusing id "photo" in the new document forever.
    session.reset();

    // Replacement document reuses the SAME node id ("photo") for an unrelated,
    // still-unfilled placeholder that DOES need enrichment.
    let mut new_state = EditorState::default();
    new_state.active_children_mut().clear();
    new_state.active_children_mut().push(rectangle_node(
        "photo",
        "Latte Image",
        Some(vec![solid_fill()]),
    ));

    let spawned = session.enqueue_missing(&new_state);

    assert!(
        spawned,
        "replacement document's placeholder must be scheduled, not suppressed by a \
         stale completed id inherited from the pre-replacement document"
    );
    assert!(
        session.is_pending(),
        "a job must be queued for the replacement document's placeholder"
    );
}

#[tokio::test]
#[ignore = "network smoke test for Openverse/Wikimedia"]
async fn fetch_first_image_url_smoke() {
    let used = std::sync::Mutex::new(std::collections::HashSet::new());
    let url = fetch_first_image_url("burger fries", None, None, &used)
        .await
        .expect("common query should return a renderable image data URL");
    assert!(url.starts_with("data:image/"), "got {url}");
    assert!(url.contains(";base64,"), "got {url}");
}

#[test]
fn openverse_selection_skips_junk_and_prefers_query_overlap() {
    use serde_json::json;
    let results = vec![
        json!({"title": "File Not Found", "url": "https://x/1.jpg"}),
        json!({"title": "Sunset over green hills", "url": "https://x/2.jpg"}),
        json!({"title": "Midnight city neon lights", "url": "https://x/3.jpg"}),
    ];
    let empty = std::collections::HashSet::new();
    let picked = super::select_openverse_result(&results, "midnight city neon", &empty)
        .expect("a result survives");
    assert_eq!(
        picked["url"], "https://x/3.jpg",
        "query-overlapping title wins"
    );

    let all_junk = vec![
        json!({"title": "404 error page", "url": "https://x/1.jpg"}),
        json!({"title": "image not found placeholder", "url": "https://x/2.jpg"}),
    ];
    assert!(
        super::select_openverse_result(&all_junk, "midnight city", &empty).is_none(),
        "all-junk result sets leave the slot empty"
    );

    let no_overlap = vec![json!({"title": "Sunset over hills", "url": "https://x/9.jpg"})];
    let fallback = super::select_openverse_result(&no_overlap, "midnight city", &empty)
        .expect("non-junk fallback");
    assert_eq!(fallback["url"], "https://x/9.jpg");

    // Session dedup: a URL already used by another card is skipped, so
    // near-identical queries stop filling every card with the same photo.
    let mut used = std::collections::HashSet::new();
    used.insert("https://x/3.jpg".to_string());
    let second = super::select_openverse_result(&results, "midnight city neon", &used)
        .expect("a different result");
    assert_ne!(second["url"], "https://x/3.jpg", "used URL is skipped");
}

#[test]
fn simplify_strips_design_artifact_words_but_never_to_empty() {
    // "synthwave album cover neon" → the corpus has no album covers, but it
    // has plenty of synthwave/neon photography.
    assert_eq!(
        simplify_search_query("synthwave album cover neon"),
        "synthwave neon"
    );
    assert_eq!(
        simplify_search_query("playlist cover daily mix"),
        "daily mix"
    );
    // All-artifact queries keep their words rather than going empty.
    assert_eq!(simplify_search_query("album cover"), "album cover");
    // Concrete-subject queries are untouched.
    assert_eq!(
        simplify_search_query("kyoto temple cherry blossom"),
        "kyoto temple cherry blossom"
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
fn a_repeat_query_gets_the_same_photo_back_not_a_dedup_downgrade() {
    use super::{
        search_intent_key, spawn_job, ImageSearchTarget, SearchIntentKey, SearchMemoEntry,
    };
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

    let used_urls: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let resolved: Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>> =
        Arc::new(Mutex::new(HashMap::new()));
    // The first search already answered "Bali Indonesia" and marked its photo used.
    let good = "https://example.org/bali-temple.jpg".to_string();
    resolved.lock().unwrap().insert(
        search_intent_key("Bali, Indonesia", None),
        SearchMemoEntry::Ready(good.clone()),
    );
    used_urls.lock().unwrap().insert(good.clone());

    // The rebuilt card asks again — differently spelled, same subject.
    let target = ImageSearchTarget {
        node_id: op_editor_core::NodeId::new("n99".to_string()),
        query: "  bali indonesia  ".to_string(),
        prompt: None,
        mode: ImageRequestMode::Search,
        aspect_ratio: None,
        width: None,
        height: None,
    };
    let job = spawn_job(
        target,
        None,
        Arc::clone(&used_urls),
        Arc::clone(&resolved),
        1,
    );
    let answer = job
        .rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("the memo answers without touching the network");
    assert_eq!(
        answer,
        Some(good),
        "the rebuilt card gets ITS photo back, not the next-best junk result"
    );
}

#[test]
fn a_pending_search_intent_is_singleflight() {
    use super::{
        search_intent_key, spawn_job, ImageSearchTarget, SearchIntentKey, SearchMemoEntry,
    };
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

    let key = search_intent_key("Bali Indonesia", None);
    let memo: Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>> =
        Arc::new(Mutex::new(HashMap::from([(
            key.clone(),
            SearchMemoEntry::Pending {
                request_id: 7,
                waiters: Vec::new(),
            },
        )])));
    let target = ImageSearchTarget {
        node_id: NodeId::new("n100"),
        query: "Bali, Indonesia".into(),
        prompt: None,
        mode: ImageRequestMode::Search,
        aspect_ratio: None,
        width: None,
        height: None,
    };
    let job = spawn_job(
        target,
        None,
        Arc::new(Mutex::new(HashSet::new())),
        Arc::clone(&memo),
        8,
    );

    let waiters = match memo.lock().unwrap().remove(&key) {
        Some(SearchMemoEntry::Pending { waiters, .. }) => waiters,
        _ => panic!("the second caller joins the pending intent"),
    };
    assert_eq!(waiters.len(), 1, "no second fetch thread was created");
    waiters[0]
        .send(Some("https://example.org/bali.jpg".into()))
        .unwrap();
    assert_eq!(
        job.rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        Some("https://example.org/bali.jpg".into())
    );
}

#[test]
fn stale_pre_reset_request_cannot_publish_into_same_key_in_new_session() {
    use super::{publish_search_result, search_intent_key, SearchIntentKey, SearchMemoEntry};
    use std::collections::HashMap;
    use std::sync::{mpsc, Arc, Mutex};

    let key = search_intent_key("Bali Indonesia", None);
    let memo: Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let (old_tx, _old_rx) = mpsc::channel();
    memo.lock().unwrap().insert(
        key.clone(),
        SearchMemoEntry::Pending {
            request_id: 10,
            waiters: vec![old_tx],
        },
    );

    // reset(), followed by a new document asking for the same intent.
    memo.lock().unwrap().clear();
    let (new_tx, new_rx) = mpsc::channel();
    memo.lock().unwrap().insert(
        key.clone(),
        SearchMemoEntry::Pending {
            request_id: 11,
            waiters: vec![new_tx],
        },
    );

    assert!(!publish_search_result(
        &memo,
        key.clone(),
        10,
        Some("https://old.example/bali.jpg".into())
    ));
    assert!(matches!(new_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert!(publish_search_result(
        &memo,
        key,
        11,
        Some("https://new.example/bali.jpg".into())
    ));
    assert_eq!(
        new_rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        Some("https://new.example/bali.jpg".into())
    );
}

#[test]
fn reset_detaches_dedup_and_memo_generations_from_old_threads() {
    let mut session = ImageSearchSession::default();
    let old_used = std::sync::Arc::clone(&session.used_urls);
    let old_memo = std::sync::Arc::clone(&session.search_memo);
    old_used.lock().unwrap().insert("openverse:old".into());

    session.reset();

    assert!(!std::sync::Arc::ptr_eq(&old_used, &session.used_urls));
    assert!(!std::sync::Arc::ptr_eq(&old_memo, &session.search_memo));
    old_used
        .lock()
        .unwrap()
        .insert("openverse:late-old-thread".into());
    assert!(
        session.used_urls.lock().unwrap().is_empty(),
        "late old-document claims stay in the detached generation"
    );
}

#[test]
fn search_memo_separates_aspect_ratio_intents() {
    use super::{search_intent_key, ImageAspectRatio};

    assert_eq!(
        search_intent_key("Bali, Indonesia", Some(ImageAspectRatio::Square)),
        search_intent_key("bali indonesia", Some(ImageAspectRatio::Square)),
        "fetch-equivalent spelling shares one intent"
    );
    assert_ne!(
        search_intent_key("Bali Indonesia", Some(ImageAspectRatio::Square)),
        search_intent_key("Bali Indonesia", Some(ImageAspectRatio::Wide)),
        "a cover and hero must not share a cached crop"
    );
}

#[test]
fn memo_identity_keeps_lossy_provider_query_collisions_separate() {
    let album = "album cover neon lights night";
    let playlist = "playlist artwork neon lights night";
    assert_eq!(
        simplify_search_query(album),
        simplify_search_query(playlist),
        "both authored intents intentionally adapt to the same photo-corpus query"
    );
    assert_ne!(
        search_intent_key(album, Some(ImageAspectRatio::Square)),
        search_intent_key(playlist, Some(ImageAspectRatio::Square)),
        "provider adaptation must not merge authored image identity"
    );

    let dome = "santorini greece white buildings blue dome";
    let beach = "santorini greece white buildings sunset beach";
    assert_eq!(simplify_search_query(dome), simplify_search_query(beach));
    assert_ne!(
        search_intent_key(dome, Some(ImageAspectRatio::Wide)),
        search_intent_key(beach, Some(ImageAspectRatio::Wide)),
        "words beyond the provider's four-keyword cap remain part of identity"
    );
}

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
    let targets = super::collect_targets(&state, &std::collections::HashSet::new());

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
    let targets = super::collect_targets(&state, &std::collections::HashSet::new());
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
    let targets = super::collect_targets(&state, &std::collections::HashSet::new());
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

#[test]
fn image_result_claim_is_atomic_across_queries() {
    let used = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let winners = (0..8)
        .map(|_| {
            let used = std::sync::Arc::clone(&used);
            std::thread::spawn(move || claim_unused_image_src(&used, "data:image/png;base64,SAME"))
        })
        .map(|thread| thread.join().expect("claim thread"))
        .filter(|claimed| *claimed)
        .count();

    assert_eq!(winners, 1, "only one concurrent query may claim an image");
    assert!(
        used.lock()
            .unwrap()
            .iter()
            .all(|key| key.starts_with("content:") && !key.contains("base64")),
        "dedup stores compact digests, not multi-megabyte data URIs"
    );
}

#[test]
fn provider_identity_reservation_releases_only_unavailable_downloads() {
    let used = std::sync::Mutex::new(std::collections::HashSet::from([
        "openverse:unavailable".to_string(),
        "openverse:claimed".to_string(),
        "openverse:duplicate".to_string(),
    ]));

    assert_eq!(
        settle_provider_identity(
            &used,
            "openverse:unavailable",
            ImageCandidateClaim::Unavailable,
        ),
        None
    );
    assert_eq!(
        settle_provider_identity(
            &used,
            "openverse:claimed",
            ImageCandidateClaim::Claimed("data:image/png;base64,OK".into()),
        ),
        Some("data:image/png;base64,OK".into())
    );
    assert_eq!(
        settle_provider_identity(&used, "openverse:duplicate", ImageCandidateClaim::Duplicate,),
        None
    );

    let used = used.lock().unwrap();
    assert!(!used.contains("openverse:unavailable"));
    assert!(used.contains("openverse:claimed"));
    assert!(
        used.contains("openverse:duplicate"),
        "a successfully downloaded duplicate remains excluded"
    );
}

#[test]
fn wikimedia_missing_or_empty_imageinfo_releases_the_page_reservation() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let client = reqwest::Client::new();
    for page in [
        serde_json::json!({"pageid": 41, "title": "Missing imageinfo"}),
        serde_json::json!({"pageid": 42, "title": "Empty imageinfo", "imageinfo": []}),
    ] {
        let identity = wikimedia_page_identity(&page).expect("page identity");
        let used = std::sync::Mutex::new(std::collections::HashSet::from([identity.clone()]));
        let candidates = wikimedia_image_candidates(&page);
        assert!(candidates.is_empty());

        let outcome = runtime.block_on(first_unused_renderable_image_src(
            &client, candidates, &used,
        ));
        assert_eq!(outcome, ImageCandidateClaim::Unavailable);
        assert_eq!(settle_provider_identity(&used, &identity, outcome), None);
        assert!(
            !used.lock().unwrap().contains(&identity),
            "an unusable page must remain retryable"
        );
    }
}

#[test]
fn openverse_claim_uses_artwork_identity_not_thumbnail_variant() {
    let used = std::sync::Mutex::new(std::collections::HashSet::new());
    let first = vec![serde_json::json!({
        "id":"art-42", "title":"Kyoto temple", "thumbnail":"https://x/thumb-400.jpg",
        "url":"https://x/full.jpg"
    })];
    let resized = vec![serde_json::json!({
        "id":"art-42", "title":"Kyoto temple", "thumbnail":"https://x/thumb-800.jpg",
        "url":"https://x/full.jpg"
    })];

    assert!(claim_openverse_result(&first, "Kyoto temple", &used).is_some());
    assert!(
        claim_openverse_result(&resized, "Kyoto temple", &used).is_none(),
        "one artwork id owns all thumbnail/full-size URL variants"
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
    let targets = super::collect_targets(&state, &std::collections::HashSet::new());
    eprintln!("SLOTS ({}):", targets.len());
    for t in &targets {
        eprintln!("  {} -> {:?}", t.node_id.as_str(), t.query);
    }
}
