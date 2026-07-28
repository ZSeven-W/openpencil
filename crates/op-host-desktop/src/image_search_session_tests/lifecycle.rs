//! Session bookkeeping tests — enqueue/poll, the scan gate, and
//! document-replacement resets. Split out of the flat
//! `image_search_session_tests.rs` to keep every file under the 800-line
//! cap; pure code motion.

use super::super::*;
use super::*;

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
fn poll_discards_standalone_image_result_after_collaboration_binds() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));
    let revision = state.document_revision();

    // Model the race explicitly: the external lookup was launched while the
    // document was standalone, then the editor joined as a collaborator
    // before the worker result reached the UI-thread sink.
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Some("https://result.example.com/burger.jpg".to_string()))
        .unwrap();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["img1".to_string()]),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("img1"),
            intent: None,
            rx,
        }],
        ..Default::default()
    };
    assert!(state.editor_ui.collab.set_authenticated_session(
        op_editor_core::CollabConnectionPhase::Active,
        op_editor_core::AuthenticatedCollabSession {
            session_name: "Shared design".into(),
            role: op_editor_core::CollabUiRole::Editor,
            share_endpoint: None,
        },
        Vec::new(),
    ));

    assert!(
        session.poll_into(&mut state),
        "discarding the result publishes a collaboration notice"
    );
    let PenNode::Image(image) = &state.active_children()[0] else {
        panic!("image")
    };
    assert!(image.src.is_empty(), "external result must not land");
    assert_eq!(
        state.document_revision(),
        revision,
        "a rejected result must not dirty document content"
    );
    assert!(session.jobs.is_empty());
    assert!(session.in_flight.is_empty());
    assert!(!session.completed.contains("img1"));
    assert_eq!(
        state.editor_ui.collab.notice.map(|notice| notice.kind),
        Some(op_editor_core::CollabNoticeKind::UnsupportedEdit(
            op_editor_core::CollabUnsupportedFeature::ExternalAssets,
        ))
    );
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
fn poll_shares_one_stale_intent_scan_across_a_completed_batch() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));
    state
        .active_children_mut()
        .push(image_node("img2", "", Some("mountain lake")));
    let original_targets = collect_targets(&state, &HashSet::new());
    let expected_by_id: std::collections::HashMap<_, _> = original_targets
        .into_iter()
        .map(|target| {
            let expected = intent_fingerprint(&target, None);
            (target.node_id.as_str().to_string(), expected)
        })
        .collect();

    let PenNode::Image(image) = &mut state.active_children_mut()[1] else {
        panic!("image")
    };
    image.image_search_query = Some("city skyline".into());
    state.mark_document_changed();

    let mut jobs = Vec::new();
    for id in ["img1", "img2"] {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Some(format!("https://result.example.com/{id}.jpg")))
            .unwrap();
        jobs.push(ImageSearchJob {
            node_id: NodeId::new(id),
            intent: Some(expected_by_id[id].clone()),
            rx,
        });
    }
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["img1".to_string(), "img2".to_string()]),
        jobs,
        ..Default::default()
    };

    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&state);
    assert!(session.poll_into_with_scene(&mut state, &scene));
    assert_eq!(
        session.stale_intent_scan_count, 1,
        "all ready jobs in one poll must share one current-intent snapshot"
    );
    let PenNode::Image(current) = &state.active_children()[0] else {
        panic!("image")
    };
    assert_eq!(
        current.src, "https://result.example.com/img1.jpg",
        "a result whose authored intent is still current must land"
    );
    let PenNode::Image(stale) = &state.active_children()[1] else {
        panic!("image")
    };
    assert!(stale.src.is_empty(), "a stale result must not land");
    assert!(session.jobs.is_empty());
    assert!(session.in_flight.is_empty());
}

#[test]
fn poll_does_not_scan_intents_while_all_jobs_are_pending() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("img1", "", Some("burger fries")));
    let target = collect_targets(&state, &HashSet::new())
        .into_iter()
        .next()
        .expect("target");
    let (_tx, rx) = std::sync::mpsc::channel();
    let mut session = ImageSearchSession {
        in_flight: HashSet::from(["img1".to_string()]),
        jobs: vec![ImageSearchJob {
            node_id: NodeId::new("img1"),
            intent: Some(intent_fingerprint(&target, None)),
            rx,
        }],
        ..Default::default()
    };

    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&state);
    assert!(!session.poll_into_with_scene(&mut state, &scene));
    assert_eq!(session.stale_intent_scan_count, 0);
    assert!(session.is_pending());
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
