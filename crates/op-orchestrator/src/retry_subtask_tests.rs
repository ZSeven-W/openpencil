//! Tests for the manual per-subtask retry entry point — see
//! `retry_subtask.rs` module doc.

use super::*;
use crate::plan::Region;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::AbortFlag;
use jian_ops_schema::PenDocument;
use op_editor_core::{EditorCommand, PenNodeExt};

fn design_request() -> DesignRequest {
    DesignRequest {
        prompt: "retry test".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

/// A live document whose sole top-level frame is the retry target's parent
/// — a different width/fill than the failed subtask's own `region`, so a
/// test can tell whether `plan_for_retry` derived its context from the
/// LIVE document (correct) or from the stale original subtask (wrong).
fn sink_with_root() -> VecDocSink {
    let doc: PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "root", "name": "Home", "width": 1200, "height": 900,
              "layout": "vertical",
              "fill": [{ "type": "solid", "color": "#112233" }] }
        ] }"##,
    )
    .expect("doc");
    let mut sink = VecDocSink::new();
    sink.state = op_editor_core::EditorState::from_document(doc);
    sink
}

fn sink_with_ordered_root() -> VecDocSink {
    let doc: PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "root", "name": "Page", "width": 1200, "height": 900,
              "layout": "vertical", "children": [
                { "type": "frame", "id": "nav", "name": "Nav", "width": 1200, "height": 80 },
                { "type": "frame", "id": "features", "name": "Features", "width": 1200, "height": 300 },
                { "type": "frame", "id": "footer", "name": "Footer", "width": 1200, "height": 80 }
              ] }
        ] }"##,
    )
    .expect("doc");
    let mut sink = VecDocSink::new();
    sink.state = op_editor_core::EditorState::from_document(doc);
    sink
}

fn ordered_root_child_ids(sink: &VecDocSink) -> Vec<String> {
    sink.state
        .active_children()
        .first()
        .and_then(PenNodeExt::children)
        .expect("ordered root has children")
        .iter()
        .map(PenNodeExt::id_str)
        .map(str::to_owned)
        .collect()
}

fn failed_subtask(parent_frame_id: Option<&str>) -> Subtask {
    Subtask {
        id: "browse-all-grid".into(),
        label: "Browse All Grid".into(),
        // Deliberately different from the live root (1200x900) so the
        // plan-derivation test can distinguish "read from the document"
        // from "read from the stale subtask region".
        region: Region {
            width: 390.0,
            height: 300.0,
        },
        id_prefix: "browse-all-grid".into(),
        parent_frame_id: parent_frame_id.map(str::to_string),
        insert_after_sibling_id: None,
        elements: Some("A grid of browsable cards".into()),
        screen: Some("Home".into()),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

// Script-gen program text — same fixture shape `run_tests_d1.rs` uses.
fn node_json(prefix: &str) -> String {
    format!(
        r#"I(null, {{"type":"frame","name":"Sec","x":0,"y":0,"width":300,"height":120,"children":[{{"type":"text","content":"{prefix}","fontSize":18}}]}});"#
    )
}

#[test]
fn retry_succeeds_against_the_live_document_and_clears_the_subtask_field() {
    let mut sink = sink_with_root();
    let subtask = failed_subtask(Some("root"));
    let request = design_request();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(node_json("retried"))]);
    let abort = AbortFlag::new();

    let outcome = futures::executor::block_on(retry_subtask(
        &subtask, &request, &llm, &mut sink, &abort, None, None,
    ));

    assert!(outcome.error.is_none(), "{outcome:?}");
    assert!(outcome.node_count > 0, "{outcome:?}");
    assert!(
        outcome.subtask.is_none(),
        "success must not carry the spec forward: {outcome:?}"
    );
    assert_eq!(outcome.id, "browse-all-grid");
}

#[test]
fn retry_moves_inserted_section_after_its_recorded_sibling() {
    let mut sink = sink_with_ordered_root();
    let mut subtask = failed_subtask(Some("root"));
    subtask.insert_after_sibling_id = Some("nav".into());
    let request = design_request();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(node_json("retried"))]);
    let abort = AbortFlag::new();

    let outcome = futures::executor::block_on(retry_subtask(
        &subtask, &request, &llm, &mut sink, &abort, None, None,
    ));

    assert!(outcome.error.is_none(), "{outcome:?}");
    assert_eq!(
        ordered_root_child_ids(&sink),
        vec![
            "nav".to_string(),
            outcome.inserted_root_ids[0].clone(),
            "features".to_string(),
            "footer".to_string()
        ]
    );
}

#[test]
fn retry_without_an_anchor_moves_inserted_section_to_the_front() {
    let mut sink = sink_with_ordered_root();
    let subtask = failed_subtask(Some("root"));
    let request = design_request();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(node_json("retried"))]);
    let abort = AbortFlag::new();

    let outcome = futures::executor::block_on(retry_subtask(
        &subtask, &request, &llm, &mut sink, &abort, None, None,
    ));

    assert!(outcome.error.is_none(), "{outcome:?}");
    assert_eq!(
        ordered_root_child_ids(&sink)[0],
        outcome.inserted_root_ids[0]
    );
}

#[test]
fn retry_with_a_stale_anchor_leaves_inserted_section_appended() {
    let mut sink = sink_with_ordered_root();
    let mut subtask = failed_subtask(Some("root"));
    subtask.insert_after_sibling_id = Some("deleted".into());
    let request = design_request();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(node_json("retried"))]);
    let abort = AbortFlag::new();

    let outcome = futures::executor::block_on(retry_subtask(
        &subtask, &request, &llm, &mut sink, &abort, None, None,
    ));

    assert!(outcome.error.is_none(), "{outcome:?}");
    assert_eq!(
        ordered_root_child_ids(&sink),
        vec![
            "nav".to_string(),
            "features".to_string(),
            "footer".to_string(),
            outcome.inserted_root_ids[0].clone()
        ]
    );
    assert!(
        !sink
            .applied
            .iter()
            .any(|command| matches!(command, EditorCommand::MoveNode { .. })),
        "a stale anchor must not emit move commands"
    );
}

#[test]
fn stale_parent_fails_fast_without_calling_the_llm() {
    struct PanicLlm;
    impl LlmClient for PanicLlm {
        fn call(
            &self,
            _req: crate::types::CallRequest,
        ) -> futures::stream::BoxStream<
            'static,
            Result<crate::types::LlmChunk, crate::types::LlmError>,
        > {
            panic!("retry_subtask must fail fast on a stale parent BEFORE calling the LLM");
        }
    }

    let mut sink = sink_with_root();
    // "gone-root" does not exist in `sink_with_root()`'s document — the
    // original run's cleanup could plausibly have replaced it.
    let subtask = failed_subtask(Some("gone-root"));
    let request = design_request();
    let llm = PanicLlm;
    let abort = AbortFlag::new();

    let outcome = futures::executor::block_on(retry_subtask(
        &subtask, &request, &llm, &mut sink, &abort, None, None,
    ));

    assert_eq!(outcome.node_count, 0);
    assert!(
        outcome.subtask.is_none(),
        "the fast-fail path returns a fresh outcome, not the persisted one: {outcome:?}"
    );
    let error = outcome.error.expect("stale parent must be reported");
    assert!(error.contains("gone-root"), "{error}");
}

#[test]
fn plan_for_retry_derives_root_frame_from_the_live_document_not_the_stale_subtask() {
    let sink = sink_with_root();
    // No parent needed for this check — only exercises `plan_for_retry`.
    let subtask = failed_subtask(None);

    let plan = plan_for_retry(&sink, &subtask);

    // Root context comes from the LIVE document's root (1200x900, #112233),
    // NOT the subtask's own (stale) region (390x300).
    assert_eq!(plan.root_frame.width, 1200.0);
    assert_eq!(plan.root_frame.height, 900.0);
    assert_eq!(
        plan.root_frame.first_solid_hex().as_deref(),
        Some("#112233")
    );

    // The subtask itself rides through completely unchanged — region,
    // elements, and screen must survive byte-for-byte.
    assert_eq!(plan.subtasks, vec![subtask.clone()]);
    assert_eq!(plan.subtasks[0].region.width, 390.0);
    assert_eq!(
        plan.subtasks[0].elements.as_deref(),
        Some(subtask.elements.as_deref().unwrap())
    );
    assert_eq!(plan.subtasks[0].screen.as_deref(), Some("Home"));
}

/// A screen rebuilt between the original run and the retry: the `screen`
/// marker survives, every id under it does not. Reproduces `0827-gk-1`, where
/// the saved page went from `n117` to `n380` and two sections then failed
/// pointing at a frame that no longer existed.
fn sink_with_rebuilt_screen() -> VecDocSink {
    let doc: PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "n380", "name": "收藏", "screen": "/screen-1",
              "width": 390, "height": 844, "layout": "vertical",
              "fill": [{ "type": "solid", "color": "#112233" }] }
        ] }"##,
    )
    .expect("doc");
    let mut sink = VecDocSink::new();
    sink.state = op_editor_core::EditorState::from_document(doc);
    sink
}

fn subtask_on_screen(parent_frame_id: &str, screen: &str) -> Subtask {
    let mut subtask = failed_subtask(Some(parent_frame_id));
    subtask.screen = Some(screen.to_string());
    subtask
}

#[test]
fn a_renumbered_screen_reparents_instead_of_making_the_user_re_describe_it() {
    let mut sink = sink_with_rebuilt_screen();
    let subtask = subtask_on_screen("n117", "/screen-1");
    let request = design_request();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(node_json("browse-all-grid"))]);
    let abort = AbortFlag::new();

    let outcome = futures::executor::block_on(retry_subtask(
        &subtask, &request, &llm, &mut sink, &abort, None, None,
    ));

    assert!(
        outcome.error.is_none(),
        "the screen marker still resolves this section's home: {outcome:?}"
    );
    assert!(outcome.node_count > 0, "{outcome:?}");
}

#[test]
fn the_reparent_resolves_through_the_screen_marker_not_the_frame_name() {
    // Names are not routing identity — two screens can share a label, and a
    // rename must not silently move a section to a different screen.
    let sink = sink_with_rebuilt_screen();
    assert_eq!(
        reparent_by_screen(&sink, &subtask_on_screen("n117", "/screen-1")),
        Some("n380".to_string())
    );
    assert_eq!(
        reparent_by_screen(&sink, &subtask_on_screen("n117", "/screen-9")),
        None,
        "an unknown screen path must not fall back to whatever frame is there"
    );
    let mut no_screen = failed_subtask(Some("n117"));
    no_screen.screen = None;
    assert_eq!(
        reparent_by_screen(&sink, &no_screen),
        None,
        "without a screen marker there is nothing to resolve through"
    );
}

#[test]
fn two_frames_claiming_one_screen_path_decline_rather_than_guess() {
    let doc: PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "a", "name": "收藏", "screen": "/screen-1",
              "width": 390, "height": 844, "layout": "vertical" },
            { "type": "frame", "id": "b", "name": "收藏(旧)", "screen": "/screen-1",
              "width": 390, "height": 844, "layout": "vertical" }
        ] }"##,
    )
    .expect("doc");
    let mut sink = VecDocSink::new();
    sink.state = op_editor_core::EditorState::from_document(doc);
    assert_eq!(
        reparent_by_screen(&sink, &subtask_on_screen("n117", "/screen-1")),
        None,
        "picking either would be a coin flip that lands the section on the wrong screen"
    );
}
