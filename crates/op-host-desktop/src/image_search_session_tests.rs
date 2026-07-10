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

    assert!(apply_result(
        &mut state,
        &NodeId::new("img1"),
        "https://example.com/photo.jpg"
    ));
    let PenNode::Image(image) = &state.active_children()[0] else {
        panic!("expected image");
    };
    assert_eq!(image.src, "https://example.com/photo.jpg");
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
    let url = fetch_first_image_url("burger fries", None, None)
        .await
        .expect("common query should return a renderable image data URL");
    assert!(url.starts_with("data:image/"), "got {url}");
    assert!(url.contains(";base64,"), "got {url}");
}
