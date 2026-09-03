//! Design-session worker tests — concurrent spawn bridge, subtask retry and
//! viewport-fit math. Split out of `design_session.rs` at the 800-line cap;
//! declared as a child module there; the re-glob below keeps the nested
//! per-topic modules' `use super::*` reaching `design_session`'s items.

use super::*;

#[cfg(test)]
mod spawn_worker_tests {
    use super::*;
    use futures::stream::BoxStream;
    use op_editor_host_core::design::{DesignCmdAck, DesignCmdOp};
    use op_orchestrator::{CallRequest, LlmChunk, LlmError};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc as std_mpsc, Arc};
    use std::thread;
    use std::time::{Duration, Instant};

    /// A recording `LlmClient` — counts how many times `call` ran and
    /// returns one scripted node-script response per call (round-robin by
    /// call order). Proves the worker invokes the REAL per-subtask runner
    /// N times, not a canned ack.
    struct RecordingLlm {
        calls: Arc<AtomicUsize>,
        responses: std::sync::Mutex<std::collections::VecDeque<String>>,
    }

    impl op_orchestrator::LlmClient for RecordingLlm {
        fn call(&self, _req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let text = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_default();
            Box::pin(futures::stream::iter(vec![Ok(LlmChunk::Text(text))]))
        }
    }

    // Script-gen is the default subagent generation protocol, so the fixture
    // is a JS program calling the bound `I(parent, obj)` recorder (a single
    // insert whose object nests its children inline) rather than raw
    // `_parent` JSONL. The batch_design executor reassigns fresh ids to every
    // inserted node regardless of what's authored here, so `name` (not `id`)
    // is the field callers must key off of if they need to identify which
    // response landed where.
    fn node_json(id: &str) -> String {
        format!(
            r#"I(null, {{"type":"frame","name":"Sec-{id}","x":0,"y":0,"width":400,"height":120,"children":[{{"type":"text","content":"Hi","fontSize":18}}]}});"#
        )
    }

    fn make_req() -> DesignRequest {
        DesignRequest {
            prompt: "p".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 3,
            continuation_context: None,
            append_context: None,
            validation_enabled: false,
            visual_ref_enabled: false,
            pinned_style_guide: None,
            reference_skeleton: None,
        }
    }

    /// End-to-end: `run_spawned_agents_worker` (off a worker thread) drives
    /// the real concurrent subagent core through a `RemoteDocSink`; the UI
    /// side acks each `DesignCmdReq` against a live `EditorState`. Proves
    /// N real LLM calls happened AND N InsertSubtree commands were forwarded
    /// over the bridge channel — not a placeholder ack.
    #[test]
    fn spawn_worker_drives_n_real_llm_calls_and_forwards_n_insert_subtrees() {
        let calls = Arc::new(AtomicUsize::new(0));
        let llm = RecordingLlm {
            calls: Arc::clone(&calls),
            responses: std::sync::Mutex::new(
                vec![node_json("a"), node_json("b"), node_json("c")].into(),
            ),
        };
        let specs = vec![
            SpawnAgentSpec {
                id: "a".into(),
                label: "A".into(),
                prompt: "design A".into(),
                parent_frame_id: None,
            },
            SpawnAgentSpec {
                id: "b".into(),
                label: "B".into(),
                prompt: "design B".into(),
                parent_frame_id: None,
            },
            SpawnAgentSpec {
                id: "c".into(),
                label: "C".into(),
                prompt: "design C".into(),
                parent_frame_id: None,
            },
        ];

        let (cmd_tx, cmd_rx) = std_mpsc::channel::<DesignCmdReq>();
        let results_slot: Arc<std::sync::Mutex<Option<Vec<SpawnAgentResult>>>> =
            Arc::new(std::sync::Mutex::new(None));
        let results_for_worker = Arc::clone(&results_slot);

        // Worker thread drives the concurrent core; the live state lives on
        // the test (UI) thread, mirrored through the RemoteDocSink channel.
        let worker = thread::spawn(move || {
            let out =
                run_spawned_agents_worker(llm, specs, make_req(), EditorState::new(), cmd_tx, None);
            *results_for_worker.lock().unwrap() = Some(out);
        });

        // UI side: a live EditorState that applies each forwarded command and
        // acks with a fresh snapshot (mirrors `pump_commands`). The ack
        // carries a `narrowed_snapshot`, not a full clone: the worker's
        // `RemoteDocSink` mirror is only ever read through `DocSink::state()`,
        // and no orchestrator path touches `chat` / `codegen` /
        // `theme_presets` (see `op_editor_core::request_snapshot`).
        let mut state = EditorState::new();
        let mut insert_subtree_count = 0usize;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(req) => {
                    let applied = match req.op {
                        DesignCmdOp::Apply(cmd) => {
                            if matches!(cmd, EditorCommand::InsertSubtree { .. }) {
                                insert_subtree_count += 1;
                            }
                            state.apply(cmd)
                        }
                        DesignCmdOp::BeginUndoBatch | DesignCmdOp::EndUndoBatch => true,
                    };
                    let _ = req.ack.send(DesignCmdAck {
                        applied,
                        new_state: op_editor_core::request_snapshot::narrowed_snapshot(&mut state),
                    });
                }
                Err(std_mpsc::RecvTimeoutError::Disconnected) => break,
                Err(std_mpsc::RecvTimeoutError::Timeout) => {
                    if Instant::now() > deadline {
                        panic!("spawn worker did not finish within the deadline");
                    }
                }
            }
        }
        worker.join().expect("spawn worker exits cleanly");

        // N real LLM calls happened (the genuine per-subtask runner ran).
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "three real subagent LLM calls"
        );
        // N InsertSubtree commands were forwarded over the bridge channel.
        assert_eq!(
            insert_subtree_count, 3,
            "three subtrees forwarded into the live document"
        );
        // The live document now carries three inserted roots.
        assert_eq!(state.active_children().len(), 3);
        // The structured results name what each agent created.
        let results = results_slot.lock().unwrap().take().expect("results set");
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.error.is_none(), "agent {} failed: {:?}", r.id, r.error);
            assert_eq!(r.node_count, 1, "each agent produced one real section root");
        }
        // NOTE: `inserted_root_ids` stays empty on the `RemoteDocSink` path —
        // the default `insert_subtree_returning_root_ids` trait impl applies
        // the command over the channel and cannot surface the post-remap ids
        // the UI thread mints. The orchestrator-core test
        // (`spawn_concurrent_tests::run_spawned_agents_invokes_real_runner…`)
        // proves real id capture against an immediate-apply `VecDocSink`.
    }
}

#[cfg(test)]
mod subtask_retry_tests {
    use super::*;
    use futures::stream::BoxStream;
    use op_editor_host_core::design::{DesignCmdAck, DesignCmdOp};
    use op_orchestrator::plan::{Region, Subtask};
    use op_orchestrator::{CallRequest, LlmChunk, LlmError};
    use std::time::{Duration, Instant};

    fn make_req() -> DesignRequest {
        DesignRequest {
            prompt: "p".into(),
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

    fn failed_subtask() -> Subtask {
        Subtask {
            id: "hero".into(),
            label: "Hero".into(),
            region: Region {
                width: 1200.0,
                height: 400.0,
            },
            id_prefix: "hero".into(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
            retry_feedback: None,
        }
    }

    /// Always returns the same scripted text — a real per-subtask runner
    /// still parses/validates/inserts it, proving `start_subtask_retry`
    /// drives the genuine `retry_subtask` path, not a canned ack.
    struct OneShotLlm(String);
    impl LlmClient for OneShotLlm {
        fn call(&self, _req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
            Box::pin(futures::stream::iter(vec![Ok(LlmChunk::Text(
                self.0.clone(),
            ))]))
        }
    }

    /// Drain both halves (`drain_cmd_requests` + `poll_progress`) each loop
    /// iteration — mirrors the desktop pump's per-frame `pump_commands` +
    /// `pump_progress` pair — until the session reports finished. Returns
    /// every `Progress` event observed, in order.
    ///
    /// Acks carry a `narrowed_snapshot` for the same reason the pump does:
    /// the worker's mirror is read only through `DocSink::state()`, which
    /// never reaches `chat` / `codegen` / `theme_presets`.
    fn drain_until_finished(session: &mut DesignSession, state: &mut EditorState) -> Vec<Progress> {
        let mut collected = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            for req in session.drain_cmd_requests() {
                let applied = match req.op {
                    DesignCmdOp::Apply(cmd) => state.apply(cmd),
                    DesignCmdOp::BeginUndoBatch | DesignCmdOp::EndUndoBatch => true,
                };
                let _ = req.ack.send(DesignCmdAck {
                    applied,
                    new_state: op_editor_core::request_snapshot::narrowed_snapshot(state),
                });
            }
            let poll = session.poll_progress();
            collected.extend(poll.progress);
            if poll.finished {
                return collected;
            }
            if Instant::now() > deadline {
                panic!("subtask retry worker did not finish within the deadline");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn start_subtask_retry_streams_started_then_done_and_ends_with_no_summary() {
        let node_script = r#"I(null, {"type":"frame","name":"Sec","x":0,"y":0,"width":400,"height":120,"children":[{"type":"text","content":"Hi","fontSize":18}]});"#;
        let llm = OneShotLlm(node_script.into());
        let mut state = EditorState::new();

        let mut session = start_subtask_retry(llm, make_req(), failed_subtask(), state.clone());
        let events = drain_until_finished(&mut session, &mut state);

        assert!(
            matches!(events.first(), Some(Progress::SubtaskStarted { id, .. }) if id == "hero"),
            "{events:?}"
        );
        assert!(
            matches!(
                events.last(),
                Some(Progress::SubtaskDone { id, node_count }) if id == "hero" && *node_count > 0
            ),
            "{events:?}"
        );
        assert_eq!(
            events.len(),
            2,
            "no extra events besides Started/Done: {events:?}"
        );
        // The real per-subtask runner inserted the section into the live
        // document via the RemoteDocSink bridge.
        assert_eq!(state.active_children().len(), 1);
    }

    #[test]
    fn start_subtask_retry_reports_failed_when_the_llm_produces_nothing() {
        let llm = OneShotLlm(String::new());
        let mut state = EditorState::new();

        let mut session = start_subtask_retry(llm, make_req(), failed_subtask(), state.clone());
        let events = drain_until_finished(&mut session, &mut state);

        assert!(
            matches!(events.first(), Some(Progress::SubtaskStarted { id, .. }) if id == "hero")
        );
        assert!(
            matches!(events.last(), Some(Progress::SubtaskFailed { id, .. }) if id == "hero"),
            "{events:?}"
        );
        assert_eq!(state.active_children().len(), 0);
    }
}

#[cfg(test)]
mod viewport_fit_tests {
    use super::*;

    fn state_with_root(height: f64) -> EditorState {
        let doc: jian_ops_schema::PenDocument = serde_json::from_str(&format!(
            r##"{{ "version": "1.0", "children": [{{
                "type": "frame", "id": "root", "name": "Screen",
                "width": 390, "height": {height}, "layout": "vertical",
                "fill": [{{ "type": "solid", "color": "#FFFFFF" }}]
            }}] }}"##
        ))
        .expect("doc");
        EditorState::from_document(doc)
    }

    #[test]
    fn design_canvas_size_reserves_the_right_rail_only_when_the_panel_is_visible() {
        const VIEWPORT_WIDTH: f32 = 1200.0;
        const VIEWPORT_HEIGHT: f32 = 800.0;
        const PROPERTY_PANEL_WIDTH: f32 = 280.0;

        let mut state = state_with_root(844.0);
        state.editor_ui.sidebar_open = false;
        state.editor_ui.property_panel_width = PROPERTY_PANEL_WIDTH;
        state.editor_ui.property_tab = op_editor_core::PropertyTab::Design;
        state.clear_selection();

        assert_eq!(
            design_canvas_size(&state, VIEWPORT_WIDTH, VIEWPORT_HEIGHT),
            (VIEWPORT_WIDTH, VIEWPORT_HEIGHT - TOP_BAR_HEIGHT),
            "Design with no selection must use the full canvas width"
        );

        state.set_single_selection(op_editor_core::NodeId::new("root"));
        assert_eq!(
            design_canvas_size(&state, VIEWPORT_WIDTH, VIEWPORT_HEIGHT),
            (
                VIEWPORT_WIDTH - PROPERTY_PANEL_WIDTH,
                VIEWPORT_HEIGHT - TOP_BAR_HEIGHT
            ),
            "Design with a live selection must reserve the property rail"
        );

        state.clear_selection();
        state.editor_ui.property_tab = op_editor_core::PropertyTab::Code;
        assert_eq!(
            design_canvas_size(&state, VIEWPORT_WIDTH, VIEWPORT_HEIGHT),
            (
                VIEWPORT_WIDTH - PROPERTY_PANEL_WIDTH,
                VIEWPORT_HEIGHT - TOP_BAR_HEIGHT
            ),
            "Code remains selection-independent and must reserve the property rail"
        );
    }

    #[test]
    fn growth_past_the_viewport_triggers_refit_and_refit_restores_visibility() {
        let mut state = state_with_root(844.0);
        // Frame the initial root.
        assert!(fit_design_viewport_to_content(&mut state, 1200.0, 800.0));
        assert!(design_content_fits_viewport(&state, 1200.0, 800.0));

        // The design grows past the framed height — no longer fully visible.
        let mut grown = state_with_root(2000.0);
        grown.viewport = state.viewport;
        assert!(
            !design_content_fits_viewport(&grown, 1200.0, 800.0),
            "grown content must report out-of-view"
        );

        // Refit restores full visibility.
        assert!(fit_design_viewport_to_content(&mut grown, 1200.0, 800.0));
        assert!(design_content_fits_viewport(&grown, 1200.0, 800.0));
    }

    #[test]
    fn zero_viewport_reports_fitting_so_headless_paths_never_loop() {
        let state = state_with_root(844.0);
        assert!(design_content_fits_viewport(&state, 0.0, 0.0));
    }
}
