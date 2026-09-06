use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenFill, SolidFillBody};
use op_editor_core::{walkers, EditorState, NodeId, PenNodeExt as _};
use op_image_enrich::{
    fallback_request_mode, fallback_search_query_for_host, is_failed_image_slot_for_host,
    is_image_fallback, restore_image_fallback_node,
};

use crate::image_search_session::{
    collect_targets, image_request_mode, ImageRequestMode, ImageSearchSession,
    SEARCH_FAILED_PLACEHOLDER_SRC,
};

use super::{EnrichError, EnrichSummary, POLL_INTERVAL};

// Stock providers occasionally return a transient empty result. Three total
// attempts bound latency while recovering isolated failures in large batches.
const MAX_STOCK_SEARCH_ATTEMPTS: usize = 3;

pub(super) trait EnrichSession {
    fn enqueue(
        &mut self,
        state: &EditorState,
        scene: &op_editor_ui::layout_scene::LayoutScene,
    ) -> bool;
    fn poll(
        &mut self,
        state: &mut EditorState,
        scene: &op_editor_ui::layout_scene::LayoutScene,
    ) -> bool;
    fn is_pending(&self) -> bool;
    fn prepare_search_retry(&mut self, node_ids: &HashSet<NodeId>);
}

impl EnrichSession for ImageSearchSession {
    fn enqueue(
        &mut self,
        state: &EditorState,
        scene: &op_editor_ui::layout_scene::LayoutScene,
    ) -> bool {
        self.enqueue_missing_with_scene(state, scene)
    }

    fn poll(
        &mut self,
        state: &mut EditorState,
        scene: &op_editor_ui::layout_scene::LayoutScene,
    ) -> bool {
        self.poll_into_with_scene(state, scene)
    }

    fn is_pending(&self) -> bool {
        self.is_pending()
    }

    fn prepare_search_retry(&mut self, node_ids: &HashSet<NodeId>) {
        self.retry_search_failures(node_ids);
    }
}

#[derive(Clone)]
struct TargetRecord {
    mode: ImageRequestMode,
    query: String,
    reset_node: PenNode,
}

pub(super) fn enrich_state_with_session<S: EnrichSession>(
    state: &mut EditorState,
    timeout: Duration,
    session: &mut S,
) -> Result<EnrichSummary, EnrichError> {
    // One deadline covers every retry on every page. Document load and the
    // final atomic save intentionally sit outside it.
    let started = Instant::now();
    let timeout_seconds = timeout.as_secs();
    let page_count = state.page_count();
    let mut targets = 0usize;
    let mut failed = 0usize;
    let mut unresolved = 0usize;

    for page in 0..page_count {
        if !state.set_active_page(page) {
            return Err(EnrichError::InvalidPage { page, page_count });
        }
        let records = collect_target_records(state);
        targets += records.len();

        // A previous invocation may have persisted a failure sentinel. Only
        // Search/Auto nodes are cleared; Generate remains a terminal failure.
        let preexisting_retry_ids = retryable_failure_ids(state, &records);
        restore_retry_nodes(state, &records, &preexisting_retry_ids);
        session.prepare_search_retry(&preexisting_retry_ids);

        for attempt in 0..MAX_STOCK_SEARCH_ATTEMPTS {
            drive_until_quiescent(state, session, page, started, timeout, timeout_seconds)?;
            let retry_ids = retryable_failure_ids(state, &records);
            if retry_ids.is_empty() || attempt + 1 == MAX_STOCK_SEARCH_ATTEMPTS {
                break;
            }
            restore_retry_nodes(state, &records, &retry_ids);
            session.prepare_search_retry(&retry_ids);
        }
        report_failed_targets(state, &records);

        let remaining_ids: HashSet<NodeId> = collect_targets(state, &HashSet::new())
            .into_iter()
            .map(|target| target.node_id)
            .collect();
        unresolved += records
            .keys()
            .filter(|node_id| remaining_ids.contains(*node_id))
            .count();
        failed += records
            .keys()
            .filter(|node_id| node_has_failure_sentinel(state, node_id))
            .count();
    }

    Ok(EnrichSummary {
        pages: page_count,
        targets,
        resolved: targets.saturating_sub(failed.saturating_add(unresolved)),
        failed,
        unresolved,
    })
}

fn drive_until_quiescent<S: EnrichSession>(
    state: &mut EditorState,
    session: &mut S,
    page: usize,
    started: Instant,
    timeout: Duration,
    timeout_seconds: u64,
) -> Result<(), EnrichError> {
    loop {
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
        let enqueued = session.enqueue(state, &scene);
        let was_pending = session.is_pending();
        let changed = session.poll(state, &scene);
        if !session.is_pending() && !enqueued && !was_pending && !changed {
            return Ok(());
        }
        // A synchronous or just-completed job gets an immediate quiescence
        // pass at the deadline. Only pending work can time out.
        if !session.is_pending() {
            continue;
        }
        if started.elapsed() >= timeout {
            let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
            let _ = session.poll(state, &scene);
            if !session.is_pending() {
                continue;
            }
            return Err(EnrichError::Timeout {
                page,
                seconds: timeout_seconds,
            });
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn collect_target_records(state: &EditorState) -> HashMap<NodeId, TargetRecord> {
    let mut records = HashMap::new();
    for target in collect_targets(state, &HashSet::new()) {
        if let Some(node) = walkers::find_node(state.active_children(), &target.node_id) {
            if is_image_fallback(node) {
                continue;
            }
            records.insert(
                target.node_id,
                TargetRecord {
                    mode: target.mode,
                    query: target.query,
                    reset_node: retry_reset_node(node, target.mode),
                },
            );
        }
    }
    collect_sentinel_records(state.active_children(), &mut records);
    records
}

fn collect_sentinel_records(children: &[PenNode], records: &mut HashMap<NodeId, TargetRecord>) {
    for node in children {
        if node_contains_failure_sentinel(node) {
            if let Some(node_id) = NodeId::new_opt(node.id_str()) {
                let mode = fallback_request_mode(node).unwrap_or_else(|| image_request_mode(node));
                records.entry(node_id).or_insert_with(|| TargetRecord {
                    mode,
                    query: fallback_search_query_for_host(node),
                    reset_node: retry_reset_node(node, mode),
                });
            }
        }
        if let Some(children) = node.children() {
            collect_sentinel_records(children, records);
        }
    }
}

fn report_failed_targets(state: &EditorState, records: &HashMap<NodeId, TargetRecord>) {
    for (node_id, record) in records {
        if !node_has_failure_sentinel(state, node_id) {
            continue;
        }
        let name = record.reset_node.base().name.as_deref().unwrap_or("");
        eprintln!(
            "[ENRICH] failed node={} mode={:?} name={:?} query={:?}",
            node_id.as_str(),
            record.mode,
            name,
            record.query
        );
    }
}

fn retry_reset_node(node: &PenNode, mode: ImageRequestMode) -> PenNode {
    if let Some(restored) = restore_image_fallback_node(node) {
        return restored;
    }
    let mut reset = node.clone();
    if mode == ImageRequestMode::Generate {
        return reset;
    }
    match &mut reset {
        PenNode::Image(image) => image.src = "".into(),
        PenNode::Frame(frame) => reset_failure_fills(frame.container.fill.as_mut()),
        PenNode::Rectangle(rectangle) => {
            reset_failure_fills(rectangle.container.fill.as_mut());
        }
        _ => {}
    }
    reset
}

fn reset_failure_fills(fills: Option<&mut Vec<PenFill>>) {
    let Some(fills) = fills else {
        return;
    };
    for fill in fills {
        if matches!(
            fill,
            PenFill::Image(image) if image.url == SEARCH_FAILED_PLACEHOLDER_SRC
        ) {
            *fill = PenFill::Solid(SolidFillBody {
                color: "#D1D5DB".to_string(),
                explain: None,
                opacity: None,
                blend_mode: None,
            });
        }
    }
}

fn retryable_failure_ids(
    state: &EditorState,
    records: &HashMap<NodeId, TargetRecord>,
) -> HashSet<NodeId> {
    records
        .iter()
        .filter(|(node_id, record)| {
            record.mode != ImageRequestMode::Generate && node_has_failure_sentinel(state, node_id)
        })
        .map(|(node_id, _)| node_id.clone())
        .collect()
}

fn restore_retry_nodes(
    state: &mut EditorState,
    records: &HashMap<NodeId, TargetRecord>,
    node_ids: &HashSet<NodeId>,
) {
    let mut changed = false;
    for node_id in node_ids {
        let Some(record) = records.get(node_id) else {
            continue;
        };
        let Some(node) = walkers::find_node_mut(state.active_children_mut(), node_id) else {
            continue;
        };
        *node = record.reset_node.clone();
        changed = true;
    }
    if changed {
        state.mark_document_changed();
    }
}

fn node_has_failure_sentinel(state: &EditorState, node_id: &NodeId) -> bool {
    walkers::find_node(state.active_children(), node_id).is_some_and(node_contains_failure_sentinel)
}

fn node_contains_failure_sentinel(node: &PenNode) -> bool {
    is_failed_image_slot_for_host(node)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet, VecDeque};

    use super::*;
    use crate::image_search_session::apply_result;

    #[derive(Default)]
    struct ScriptedSession {
        outcomes: HashMap<String, VecDeque<Option<String>>>,
        attempts: HashMap<String, usize>,
        completed: HashSet<String>,
        pending: Vec<(NodeId, ImageRequestMode)>,
    }

    impl ScriptedSession {
        fn with_outcomes(node_id: &str, outcomes: Vec<Option<&str>>) -> Self {
            Self {
                outcomes: HashMap::from([(
                    node_id.to_string(),
                    outcomes
                        .into_iter()
                        .map(|value| value.map(str::to_string))
                        .collect(),
                )]),
                ..Self::default()
            }
        }

        fn and_outcomes(mut self, node_id: &str, outcomes: Vec<Option<&str>>) -> Self {
            self.outcomes.insert(
                node_id.to_string(),
                outcomes
                    .into_iter()
                    .map(|value| value.map(str::to_string))
                    .collect(),
            );
            self
        }
    }

    impl EnrichSession for ScriptedSession {
        fn enqueue(
            &mut self,
            state: &EditorState,
            _scene: &op_editor_ui::layout_scene::LayoutScene,
        ) -> bool {
            let mut known = self.completed.clone();
            known.extend(
                self.pending
                    .iter()
                    .map(|(node_id, _)| node_id.as_str().to_string()),
            );
            let targets = collect_targets(state, &known);
            for target in targets {
                *self
                    .attempts
                    .entry(target.node_id.as_str().to_string())
                    .or_default() += 1;
                self.pending.push((target.node_id, target.mode));
            }
            !self.pending.is_empty()
        }

        fn poll(
            &mut self,
            state: &mut EditorState,
            _scene: &op_editor_ui::layout_scene::LayoutScene,
        ) -> bool {
            let pending = std::mem::take(&mut self.pending);
            let mut changed = false;
            for (node_id, _) in pending {
                let id = node_id.as_str().to_string();
                let outcome = self
                    .outcomes
                    .get_mut(&id)
                    .and_then(VecDeque::pop_front)
                    .flatten();
                let url = outcome.as_deref().unwrap_or(SEARCH_FAILED_PLACEHOLDER_SRC);
                changed |= apply_result(state, &node_id, url);
                self.completed.insert(id);
            }
            changed
        }

        fn is_pending(&self) -> bool {
            !self.pending.is_empty()
        }

        fn prepare_search_retry(&mut self, node_ids: &HashSet<NodeId>) {
            for node_id in node_ids {
                self.completed.remove(node_id.as_str());
            }
        }
    }

    fn load(source: &str) -> EditorState {
        op_host_services::doc_io::load_editor_state_from_source(
            source,
            op_editor_core::Locale::EnUs,
        )
        .expect("load enrichment fixture")
    }

    #[test]
    fn retryable_failure_succeeds_without_replacing_first_round_success() {
        let mut state = load(
            r#"{"version":"1.0","children":[{"type":"image","id":"stable","src":"","imageSearchQuery":"forest trail","width":160,"height":90},{"type":"image","id":"retry","src":"","imageSearchQuery":"mountain lake","width":160,"height":90}]}"#,
        );
        let mut session =
            ScriptedSession::with_outcomes("stable", vec![Some("data:image/png;base64,AA==")])
                .and_outcomes("retry", vec![None, Some("data:image/png;base64,AQ==")]);

        let summary =
            enrich_state_with_session(&mut state, Duration::ZERO, &mut session).expect("enrich");

        assert_eq!(session.attempts.get("stable"), Some(&1));
        assert_eq!(session.attempts.get("retry"), Some(&2));
        assert_eq!(
            summary,
            EnrichSummary {
                pages: 1,
                targets: 2,
                resolved: 2,
                failed: 0,
                unresolved: 0,
            }
        );
        let sources: Vec<_> = state
            .active_children()
            .iter()
            .map(|node| match node {
                PenNode::Image(image) => image.src.to_string(),
                _ => panic!("expected image"),
            })
            .collect();
        assert_eq!(
            sources,
            ["data:image/png;base64,AA==", "data:image/png;base64,AQ=="]
        );
    }

    #[test]
    fn persisted_fallback_tile_is_restored_and_retried_as_an_image() {
        let mut state = load(
            r#"{"version":"1.0","children":[{"type":"image","id":"retry","name":"Forest","src":"","imageSearchQuery":"forest trail","width":160,"height":90}]}"#,
        );
        let id = NodeId::new("retry");
        assert!(apply_result(&mut state, &id, SEARCH_FAILED_PLACEHOLDER_SRC));
        assert!(op_image_enrich::is_image_fallback(
            walkers::find_node(state.active_children(), &id).expect("fallback")
        ));

        let mut session =
            ScriptedSession::with_outcomes("retry", vec![Some("data:image/png;base64,AQ==")]);
        let summary =
            enrich_state_with_session(&mut state, Duration::ZERO, &mut session).expect("retry");

        assert_eq!(summary.resolved, 1);
        let PenNode::Image(image) =
            walkers::find_node(state.active_children(), &id).expect("image after retry")
        else {
            panic!("retry must restore an image node");
        };
        assert_eq!(image.src, "data:image/png;base64,AQ==");
        assert_eq!(image.image_search_query.as_deref(), Some("forest trail"));
    }

    #[test]
    fn explicit_generate_failure_is_not_retried() {
        let mut state = load(
            r#"{"version":"1.0","children":[{"type":"image","id":"art","src":"","imagePrompt":"paint a moonlit forest","width":160,"height":90}]}"#,
        );
        let mut session = ScriptedSession::with_outcomes("art", vec![None]);

        let summary =
            enrich_state_with_session(&mut state, Duration::ZERO, &mut session).expect("enrich");

        assert_eq!(session.attempts.get("art"), Some(&1));
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.unresolved, 0);
    }

    #[test]
    fn exhausted_search_retries_still_write_the_partial_result() {
        let mut state = load(
            r#"{"version":"1.0","children":[{"type":"image","id":"photo","src":"","imageSearchQuery":"mountain lake","width":160,"height":90}]}"#,
        );
        let mut session = ScriptedSession::with_outcomes("photo", vec![None, None, None]);
        let summary =
            enrich_state_with_session(&mut state, Duration::ZERO, &mut session).expect("enrich");
        assert_eq!(session.attempts.get("photo"), Some(&3));

        let directory =
            std::env::temp_dir().join(format!("openpencil-enrich-retry-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create fixture directory");
        let output = directory.join("output.op");
        std::fs::write(&output, b"keep-existing-output").expect("seed output");

        let result = super::super::save_enriched_state(&mut state, &output, summary);

        // The run still fails loudly, but the document the caller authored is
        // written rather than discarded — the seeded placeholder is gone.
        assert!(matches!(result, Err(EnrichError::Failed(_))));
        let written = std::fs::read(&output).expect("read written output");
        assert_ne!(written, b"keep-existing-output");
        let reloaded =
            op_host_services::doc_io::load_editor_state(&output, op_editor_core::Locale::EnUs)
                .expect("the partial result parses");
        assert_eq!(reloaded.active_children().len(), 1);
        std::fs::remove_dir_all(directory).expect("remove fixture directory");
    }
}
