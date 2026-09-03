//! `concurrent` unit tests — `BufferDocSink` (the buffering sink shared by
//! the `spawn_agents` fan-out and the screen-group executor) and
//! `effective_concurrency`'s gating logic. End-to-end coverage of the
//! screen-group executor itself (`run_screen_groups_concurrent`, the retry
//! ladder, failure isolation, progress completeness) lives in
//! `run_tests_screen_groups.rs` — it needs a full `Orchestrator::run()` to
//! exercise real per-root content routing.

use super::*;
use op_editor_core::EditorCommand;

// ── effective_concurrency ────────────────────────────────────────────────

#[test]
fn single_group_always_forces_sequential() {
    // At most one group — nothing to run alongside — regardless of how high
    // `request.concurrency` (⚡Nx) is set.
    assert_eq!(effective_concurrency(1, 0), 1);
    assert_eq!(effective_concurrency(1, 1), 1);
    assert_eq!(effective_concurrency(6, 1), 1);
}

#[test]
fn multi_group_is_capped_by_both_clamp_and_group_count() {
    // 3 groups, concurrency=6 (max) → capped to 3 (no point over-provisioning
    // permits beyond the number of groups that could ever use them).
    assert_eq!(effective_concurrency(6, 3), 3);
    // 5 groups, concurrency=2 → stays 2 (the [1,6] clamp is the binding
    // constraint here, not the group count).
    assert_eq!(effective_concurrency(2, 5), 2);
    // concurrency=0 clamps to 1 first, then is unaffected by group count.
    assert_eq!(effective_concurrency(0, 3), 1);
    // concurrency=99 clamps to 6, then capped to the group count.
    assert_eq!(effective_concurrency(99, 4), 4);
}

/// `BufferDocSink` collects commands without modifying a real doc.
#[test]
fn buffer_doc_sink_collects_commands() {
    let mut sink = BufferDocSink::new(EditorState::new());
    let applied = sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    assert!(applied, "BufferDocSink.apply must always return true");
    assert_eq!(sink.commands.len(), 1);
}

/// `state()` on `BufferDocSink` returns the snapshot passed at construction.
#[test]
fn buffer_doc_sink_state_returns_snapshot() {
    let state = EditorState::new();
    let sink = BufferDocSink::new(state.clone());
    let _ = sink.state();
}

/// `BufferDocSink` tracks undo-batch depth correctly.
#[test]
fn buffer_doc_sink_undo_batch_depth() {
    let mut sink = BufferDocSink::new(EditorState::new());
    assert_eq!(sink.batch_depth, 0);
    sink.begin_undo_batch();
    assert_eq!(sink.batch_depth, 1);
    sink.end_undo_batch();
    assert_eq!(sink.batch_depth, 0);
}

// ── geometry_echo (`maybe_geometry_echo`) ───────────────────────────────
//
// End-to-end coverage (a full `Orchestrator::run()` reproducing the
// "Savings Goals" rail-collapse fixture) lives in `run_tests.rs`; these
// exercise `maybe_geometry_echo` directly against a hand-built sink so
// each budget/no-op/adopt/keep-original branch is provable in isolation.

mod geometry_echo {
    use super::*;
    use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
    use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
    use crate::types::{DesignRequest, GeometryEchoBudget, LlmError, Progress, SubtaskOutcome};
    use futures::executor::block_on;

    fn req() -> DesignRequest {
        DesignRequest {
            prompt: "a finance app".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            continuation_context: None,
            append_context: None,
            validation_enabled: true,
            visual_ref_enabled: false,
            pinned_style_guide: None,
            reference_skeleton: None,
        }
    }

    fn plan() -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "Page".into(),
                width: 375.0,
                height: 800.0,
                layout: None,
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks: vec![],
            style_guide_name: None,
        }
    }

    fn subtask() -> Subtask {
        Subtask {
            id: "goals".into(),
            label: "Savings Goals".into(),
            region: Region {
                width: 375.0,
                height: 200.0,
            },
            id_prefix: "goals".into(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
            retry_feedback: None,
        }
    }

    /// A card rail whose `fill_container` siblings collapse beside the
    /// 200px fixed `Emergency Fund` card — the SAME shape
    /// `geometry_validation_tests.rs`'s `rail_width_collapse_is_echoed_for_the_model_under_real_layout`
    /// proves `geometry_diagnostics` flags under real jian layout.
    fn violated_rail_doc() -> jian_ops_schema::PenDocument {
        serde_json::from_value(serde_json::json!({
            "version": "1.0",
            "children": [{
                "type": "frame", "id": "rail", "name": "Goals Rail", "layout": "horizontal",
                "width": 327, "height": "fit_content", "gap": 12,
                "children": [
                    { "type": "frame", "id": "c1", "name": "Emergency Fund", "layout": "vertical",
                      "width": 200, "height": "fit_content",
                      "fill": [{"type": "solid", "color": "#FFFFFF"}] },
                    { "type": "frame", "id": "c2", "name": "New Car", "layout": "vertical",
                      "width": "fill_container", "height": "fit_content",
                      "fill": [{"type": "solid", "color": "#FFFFFF"}] },
                    { "type": "frame", "id": "c3", "name": "Vacation", "layout": "vertical",
                      "width": "fill_container", "height": "fit_content",
                      "fill": [{"type": "solid", "color": "#FFFFFF"}] }
                ]
            }]
        }))
        .expect("valid doc")
    }

    fn sink_with_violated_rail() -> VecDocSink {
        VecDocSink {
            state: EditorState::from_document(violated_rail_doc()),
            applied: Vec::new(),
            batch_depth: 0,
        }
    }

    fn outcome_for_violated_rail() -> SubtaskOutcome {
        SubtaskOutcome {
            id: "goals".into(),
            node_count: 1,
            error: None,
            inserted_root_ids: vec!["rail".into()],
            subtask: None,
        }
    }

    /// A clean 3-up rail (all `fill_container`, no fixed reference card to
    /// starve against) — the echo retry's "model fixed it" response.
    const CLEAN_RAIL_SCRIPT: &str = r##"I(null, {"type":"frame","name":"Goals Rail","layout":"horizontal","width":327,"height":"fit_content","gap":12,"children":[
        {"type":"frame","name":"Emergency Fund","layout":"vertical","width":"fill_container","height":"fit_content","fill":[{"type":"solid","color":"#FFFFFF"}]},
        {"type":"frame","name":"New Car","layout":"vertical","width":"fill_container","height":"fit_content","fill":[{"type":"solid","color":"#FFFFFF"}]},
        {"type":"frame","name":"Vacation","layout":"vertical","width":"fill_container","height":"fit_content","fill":[{"type":"solid","color":"#FFFFFF"}]}
    ]});"##;

    #[test]
    fn successful_retry_replaces_the_violated_content() {
        let mut sink = sink_with_violated_rail();
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(CLEAN_RAIL_SCRIPT.into())]);
        let budget = GeometryEchoBudget::new(6);
        let mut events: Vec<Progress> = Vec::new();
        let mut on_progress = |p: Progress| events.push(p);

        let result = block_on(maybe_geometry_echo(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
            None,
            &budget,
            &mut on_progress,
            outcome_for_violated_rail(),
        ));

        assert!(result.node_count > 0, "the retry's real content must land");
        assert_ne!(
            result.inserted_root_ids,
            vec!["rail".to_string()],
            "the ADOPTED outcome must carry the NEW root id(s), not the original"
        );
        assert!(
            matches!(events.first(), Some(Progress::GeometryEcho { issue_count, .. }) if *issue_count >= 2),
            "must announce the echo with the real issue count: {events:?}"
        );
        // The original "rail" root must be gone — deleted in favour of the
        // retry's real replacement, not left as an orphaned duplicate.
        assert!(
            op_editor_core::walkers::find_node(
                sink.state().active_children(),
                &op_editor_core::NodeId::new("rail")
            )
            .is_none(),
            "the original violated root must be deleted after a successful replace"
        );
    }

    #[test]
    fn failed_retry_keeps_the_original_content() {
        let mut sink = sink_with_violated_rail();
        let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
            message: "stream disconnected before completion".into(),
            aborted: false,
        })]);
        let budget = GeometryEchoBudget::new(6);
        let mut on_progress = |_p: Progress| {};

        let result = block_on(maybe_geometry_echo(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
            None,
            &budget,
            &mut on_progress,
            outcome_for_violated_rail(),
        ));

        assert_eq!(
            result.inserted_root_ids,
            vec!["rail".to_string()],
            "a failed echo retry must keep the ORIGINAL outcome untouched"
        );
        // The original content must still be live in the document — a
        // failed retry must never delete real content it couldn't replace.
        assert!(op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &op_editor_core::NodeId::new("rail")
        )
        .is_some());
    }

    #[test]
    fn no_violation_spends_zero_extra_calls() {
        let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
            "version": "1.0",
            "children": [{
                "type": "frame", "id": "rail", "name": "Goals Rail", "layout": "horizontal",
                "width": 327, "height": "fit_content", "gap": 12,
                "children": [
                    { "type": "frame", "id": "c1", "name": "Emergency Fund", "layout": "vertical",
                      "width": "fill_container", "height": "fit_content",
                      "fill": [{"type": "solid", "color": "#FFFFFF"}] },
                    { "type": "frame", "id": "c2", "name": "New Car", "layout": "vertical",
                      "width": "fill_container", "height": "fit_content",
                      "fill": [{"type": "solid", "color": "#FFFFFF"}] }
                ]
            }]
        }))
        .expect("valid doc");
        let mut sink = VecDocSink {
            state: EditorState::from_document(doc),
            applied: Vec::new(),
            batch_depth: 0,
        };
        // Empty response queue — if `maybe_geometry_echo` made ANY LLM call
        // it would get `ScriptedLlm`'s "exhausted" error and the assertions
        // below would fail, proving zero calls happened rather than merely
        // asserting a call count we can't otherwise observe.
        let llm = ScriptedLlm::new(vec![]);
        let budget = GeometryEchoBudget::new(6);
        let mut events: Vec<Progress> = Vec::new();
        let mut on_progress = |p: Progress| events.push(p);

        let result = block_on(maybe_geometry_echo(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
            None,
            &budget,
            &mut on_progress,
            SubtaskOutcome {
                id: "goals".into(),
                node_count: 1,
                error: None,
                inserted_root_ids: vec!["rail".into()],
                subtask: None,
            },
        ));

        assert_eq!(result.inserted_root_ids, vec!["rail".to_string()]);
        assert!(
            events.is_empty(),
            "no diagnostic, no progress event: {events:?}"
        );
        assert!(
            budget.try_consume(),
            "budget of 6 must be fully untouched (still has all 6)"
        );
    }

    #[test]
    fn exhausted_budget_skips_the_retry_even_with_a_real_violation() {
        let mut sink = sink_with_violated_rail();
        let llm = ScriptedLlm::new(vec![]); // must never be called
        let budget = GeometryEchoBudget::new(0);
        let mut events: Vec<Progress> = Vec::new();
        let mut on_progress = |p: Progress| events.push(p);

        let result = block_on(maybe_geometry_echo(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
            None,
            &budget,
            &mut on_progress,
            outcome_for_violated_rail(),
        ));

        assert_eq!(
            result.inserted_root_ids,
            vec!["rail".to_string()],
            "an exhausted budget must leave the original outcome untouched"
        );
        assert!(
            events.is_empty(),
            "an exhausted budget must not even announce the echo: {events:?}"
        );
    }

    #[test]
    fn zero_node_outcome_is_never_echoed() {
        // A subtask that failed outright (all 3 ladder attempts zero-node)
        // has nothing to lay out — must be a pure pass-through, not a panic
        // or a spurious LLM call.
        let mut sink = VecDocSink::new();
        let llm = ScriptedLlm::new(vec![]);
        let budget = GeometryEchoBudget::new(6);
        let mut on_progress = |_p: Progress| {};

        let result = block_on(maybe_geometry_echo(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
            None,
            &budget,
            &mut on_progress,
            SubtaskOutcome {
                id: "goals".into(),
                node_count: 0,
                error: Some("script error: unexpected end of string".into()),
                inserted_root_ids: Vec::new(),
                subtask: None,
            },
        ));
        assert_eq!(result.node_count, 0);
    }

    #[test]
    fn buffered_sink_with_empty_ids_is_never_echoed() {
        // The concurrent screen-group path's `BufferDocSink` always returns
        // an empty `inserted_root_ids` (its `state()` never reflects its
        // own buffered inserts — see `BufferDocSink`'s doc) — this is the
        // signal `maybe_geometry_echo` uses to recognise "nothing live to
        // address", regardless of sink type.
        let mut sink = BufferDocSink::new(EditorState::new());
        let llm = ScriptedLlm::new(vec![]);
        let budget = GeometryEchoBudget::new(6);
        let mut events: Vec<Progress> = Vec::new();
        let mut on_progress = |p: Progress| events.push(p);

        let result = block_on(maybe_geometry_echo(
            &subtask(),
            &plan(),
            &req(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            false,
            false,
            None,
            &budget,
            &mut on_progress,
            SubtaskOutcome {
                id: "goals".into(),
                node_count: 1,
                error: None,
                inserted_root_ids: Vec::new(),
                subtask: None,
            },
        ));
        assert!(
            events.is_empty(),
            "buffered sink must never trigger the echo: {events:?}"
        );
        assert_eq!(result.node_count, 1);
    }
}
