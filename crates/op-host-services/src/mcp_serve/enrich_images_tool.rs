//! MCP tool `enrich_images` — resolve unresolved image slots on the active
//! page with real stock photos (Openverse, Wikimedia fallback).
//!
//! This is the in-process version of `openpencil-desktop --enrich-images` for
//! MCP clients: it reuses the exact slot-detection predicates and write-back
//! semantics of the desktop image-search session (both lifted into the shared
//! `op-image-enrich` crate), but drives them synchronously on the calling
//! thread through the `web_image_search` backend instead of the desktop's
//! background job session. One search + one write-back per target.
//!
//! ## Search-only contract
//!
//! A slot whose acquisition mode is explicitly `Generate` is counted as
//! failed and NEVER silently converted into a stock search — the same
//! contract the CLI mode enforces. `Auto` and `Search` slots are resolved
//! through stock search.
//!
//! ## Blocking + timeout
//!
//! The tool blocks until every target is resolved, the `timeout_seconds`
//! budget is spent, or no targets remain. Production provider ladders receive
//! the remaining overall budget, so one slow Openverse/Wikimedia search cannot
//! outlive the MCP deadline. Targets never started are reported as
//! `unresolved`.
//!
//! ## Mutations ride the applier
//!
//! Like the other write tools, the tool runs against a clone of its snapshot
//! and returns the landed writes as one `EditorCommand::Batch` of
//! `PatchNodeData` commands (one per changed node, carrying exactly the
//! top-level keys `apply_result` changed), so the host applies and saves them
//! through the normal command path.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use op_editor_core::{walkers, EditorCommand, EditorState, NodeId, PenNodeExt as _};
use op_image_enrich::{
    apply_result, collect_targets, ImageRequestMode, ImageSearchTarget,
    SEARCH_FAILED_PLACEHOLDER_SRC,
};
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;

/// Default overall budget, matching the `--enrich-images` CLI default.
const DEFAULT_TIMEOUT_SECONDS: u64 = 120;
/// Upper bound on `timeout_seconds`, matching the task contract. Values above
/// it are clamped rather than rejected.
const MAX_TIMEOUT_SECONDS: u64 = 600;
/// Keep small commerce grids inside one overall deadline without turning a
/// page with many inferred slots into an unbounded request burst.
const MAX_PARALLEL_SEARCHES: usize = 3;

/// Injectable stock-search backend — the seam tests use to run the loop
/// without touching the network. The production implementation wraps the
/// shared `web_image_search` ladder.
pub(crate) trait ImageSearchBackend: Send + Sync {
    /// Resolve one target to a landed url, or `None` when every provider
    /// avenue failed. The caller decides what `None` means (the failed-search
    /// sentinel, per the enrichment contract).
    fn search(&self, target: &ImageSearchTarget) -> Option<String>;

    /// Deadline-aware entry point used by the enrichment loop. Injected test
    /// backends retain the original synchronous seam through this default;
    /// production overrides it to bound the complete async provider ladder.
    fn search_before(&self, target: &ImageSearchTarget, _deadline: Instant) -> Option<String> {
        self.search(target)
    }

    /// Resolve a target list under one absolute deadline. Test backends keep
    /// the original serial seam by default; production overrides this with a
    /// bounded parallel implementation.
    fn search_many_before(
        &self,
        targets: &[ImageSearchTarget],
        deadline: Instant,
    ) -> Vec<ImageSearchAttempt> {
        targets
            .iter()
            .map(|target| {
                if Instant::now() >= deadline {
                    ImageSearchAttempt::NotStarted
                } else {
                    ImageSearchAttempt::Completed(self.search_before(target, deadline))
                }
            })
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ImageSearchAttempt {
    NotStarted,
    Completed(Option<String>),
}

pub(crate) fn search_parallel_before(
    backend: &dyn ImageSearchBackend,
    targets: &[ImageSearchTarget],
    deadline: Instant,
) -> Vec<ImageSearchAttempt> {
    let mut attempts = Vec::with_capacity(targets.len());
    for chunk in targets.chunks(MAX_PARALLEL_SEARCHES) {
        if Instant::now() >= deadline {
            attempts.resize_with(targets.len(), || ImageSearchAttempt::NotStarted);
            break;
        }
        let results = std::thread::scope(|scope| {
            // Keep the owned-clone spawn shape: rewriting it to borrow the
            // chunk items changes the capture set and measurably broke the
            // bounded-parallelism behavior under test (workers resolved 0/6).
            #[allow(clippy::redundant_iter_cloned)]
            let handles: Vec<_> = chunk
                .iter()
                .map(|target| scope.spawn(move || backend.search_before(target, deadline)))
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    ImageSearchAttempt::Completed(
                        handle.join().expect("image search worker panicked"),
                    )
                })
                .collect::<Vec<_>>()
        });
        attempts.extend(results);
    }
    attempts
}

/// Production backend: the daemon's Openverse → two-keyword retry →
/// Wikimedia ladder, first hit wins. The first hit is the full `data:` URL.
struct WebSearchBackend {
    credentials: Option<crate::web_image_search::WebOpenverseCredentials>,
}

impl WebSearchBackend {
    fn from_state(state: &EditorState) -> Self {
        let settings = &state.editor_ui.agent_settings;
        Self {
            credentials: crate::web_image_search::WebOpenverseCredentials::from_parts(
                &settings.openverse_client_id,
                &settings.openverse_client_secret,
            ),
        }
    }
}

impl ImageSearchBackend for WebSearchBackend {
    fn search(&self, target: &ImageSearchTarget) -> Option<String> {
        let outcome =
            crate::web_image_search::run_search_blocking(&target.query, self.credentials.as_ref());
        outcome
            .results
            .into_iter()
            .next()
            .map(|hit| hit.thumb_data_url)
    }

    fn search_before(&self, target: &ImageSearchTarget, deadline: Instant) -> Option<String> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return None;
        }
        crate::web_image_search::run_first_search_blocking_with_timeout(
            &target.query,
            self.credentials.as_ref(),
            remaining,
        )
        .results
        .into_iter()
        .next()
        .map(|hit| hit.thumb_data_url)
    }

    fn search_many_before(
        &self,
        targets: &[ImageSearchTarget],
        deadline: Instant,
    ) -> Vec<ImageSearchAttempt> {
        search_parallel_before(self, targets, deadline)
    }
}

/// The `enrich_images` tool: a snapshot of the document at registration time.
/// `backend` is `None` in production (the Web backend is built per call from
/// the snapshot's persisted Openverse credentials); tests inject a stub.
pub struct EnrichImagesTool {
    state: EditorState,
    backend: Option<Arc<dyn ImageSearchBackend>>,
}

/// Snapshot constructor used by the registry.
pub fn enrich_images_snapshot(doc: &EditorState) -> EnrichImagesTool {
    EnrichImagesTool {
        state: doc.clone(),
        backend: None,
    }
}

#[cfg(test)]
impl EnrichImagesTool {
    /// Test-only constructor with an injected search backend (never network).
    pub(crate) fn for_test(
        doc: &EditorState,
        backend: Arc<dyn ImageSearchBackend>,
    ) -> EnrichImagesTool {
        EnrichImagesTool {
            state: doc.clone(),
            backend: Some(backend),
        }
    }
}

impl McpTool for EnrichImagesTool {
    fn name(&self) -> &str {
        "enrich_images"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let timeout_seconds = match parse_timeout_seconds(args) {
            Ok(seconds) => seconds,
            Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
        };
        let scope = match parse_root_scope(args, &self.state) {
            Ok(scope) => scope,
            Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
        };
        let mut state = self.state.clone();
        let backend: &dyn ImageSearchBackend = match self.backend.as_deref() {
            Some(injected) => injected,
            None => &WebSearchBackend::from_state(&state),
        };
        let budget = Duration::from_secs(timeout_seconds);
        let run = run_enrich_sync(&mut state, scope.as_deref(), budget, backend);
        let json = serde_json::json!({
            "targets": run.summary.targets,
            "resolved": run.summary.resolved,
            "failed": run.summary.failed,
            "unresolved": run.summary.unresolved,
        })
        .to_string();
        if run.commands.is_empty() {
            ToolOutcome::OkJson(json)
        } else {
            ToolOutcome::OkJsonWithCommand(
                json,
                EditorCommand::Batch {
                    commands: run.commands,
                },
            )
        }
    }
}

/// `timeout_seconds`: numeric string, default [`DEFAULT_TIMEOUT_SECONDS`],
/// clamped to [`MAX_TIMEOUT_SECONDS`]. Zero / negative / non-numeric reject.
fn parse_timeout_seconds(args: &BTreeMap<String, String>) -> Result<u64, String> {
    let raw = args
        .get("timeout_seconds")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let Some(raw) = raw else {
        return Ok(DEFAULT_TIMEOUT_SECONDS);
    };
    let parsed = raw
        .parse::<u64>()
        .map_err(|_| format!("invalid timeout_seconds: {raw}"))?;
    if parsed == 0 {
        return Err("timeout_seconds must be greater than zero".to_string());
    }
    Ok(parsed.min(MAX_TIMEOUT_SECONDS))
}

/// `root_ids`: optional JSON array / comma-separated list scoping the walk to
/// those subtrees. `None` = the whole active page. Unknown ids reject (a
/// filter that matches nothing must not read as "no slots").
fn parse_root_scope(
    args: &BTreeMap<String, String>,
    state: &EditorState,
) -> Result<Option<Vec<String>>, String> {
    let raw = args
        .get("root_ids")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let Some(raw) = raw else {
        return Ok(None);
    };
    let ids: Vec<String> = if raw.starts_with('[') {
        match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(serde_json::Value::Array(items)) => items
                .into_iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect(),
            _ => return Err("root_ids must be a JSON array of node id strings".to_string()),
        }
    } else {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    };
    if ids.is_empty() {
        return Err("root_ids must name at least one node".to_string());
    }
    for id in &ids {
        if walkers::find_node(state.active_children(), &NodeId::new(id.clone())).is_none() {
            return Err(format!("root_ids: node not found on the active page: {id}"));
        }
    }
    Ok(Some(ids))
}

/// What the run did — the `--enrich-images` CLI's summary semantics, minus
/// the `pages` field (this tool walks the active page only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EnrichSummary {
    pub targets: usize,
    pub resolved: usize,
    pub failed: usize,
    pub unresolved: usize,
}

/// The run's outcome: the summary plus the replayable writes.
#[derive(Debug, Default)]
pub(crate) struct EnrichRun {
    pub summary: EnrichSummary,
    pub commands: Vec<EditorCommand>,
}

/// The synchronous enrich loop: collect targets on the active page (optionally
/// scoped to `root_ids` subtrees), then search in deterministic batches of at
/// most [`MAX_PARALLEL_SEARCHES`] and write results back in target order.
/// Explicit `Generate` targets land the failed-search sentinel up front and
/// never reach a search. The deadline gates STARTING a new batch; production
/// also bounds every started provider ladder by the same remaining overall
/// budget. Unstarted targets stay untouched and count as `unresolved`.
/// The budget clock starts at the search phase, after target collection:
/// local scene/collection work never consumes provider wall-clock.
///
/// End-state accounting mirrors the CLI (`image_enrich_cli/retry.rs`):
/// `unresolved` = targets still empty afterwards, `failed` = targets whose
/// node carries the failure sentinel, `resolved` = the remainder.
pub(crate) fn run_enrich_sync(
    state: &mut EditorState,
    scope: Option<&[String]>,
    budget: Duration,
    backend: &dyn ImageSearchBackend,
) -> EnrichRun {
    let mut run = EnrichRun::default();
    let all_targets = collect_targets(state, &std::collections::HashSet::new());
    let targets: Vec<ImageSearchTarget> = match scope {
        None => all_targets,
        Some(roots) => {
            let subtree_ids = subtree_ids_of(state, roots);
            all_targets
                .into_iter()
                .filter(|target| subtree_ids.contains(target.node_id.as_str()))
                .collect()
        }
    };
    run.summary.targets = targets.len();

    // Explicit Generate slots fail immediately: they are a local write-back,
    // not provider work, so the wall-clock budget never gates them.
    // `acted` tracks exactly which targets this run touched so the end-state
    // accounting below can judge them against the mutated tree.
    let mut acted: Vec<NodeId> = Vec::new();
    let mut searchable = Vec::new();
    for target in targets {
        if target.mode == ImageRequestMode::Generate {
            record_apply(
                state,
                &target.node_id,
                SEARCH_FAILED_PLACEHOLDER_SRC,
                &mut run.commands,
            );
            acted.push(target.node_id);
            continue;
        }
        searchable.push(target);
    }

    // The budget clock starts HERE, at the search phase. Target collection
    // resolves the page layout scene (a cold text-measure/font init can cost
    // ~1s in a fresh process); charging that local work against the provider
    // budget silently starved short-budget runs before any search began.
    let deadline = Instant::now() + budget;
    let attempts = backend.search_many_before(&searchable, deadline);
    for (target, attempt) in searchable.into_iter().zip(attempts) {
        if let ImageSearchAttempt::Completed(url) = attempt {
            let url = url.unwrap_or_else(|| SEARCH_FAILED_PLACEHOLDER_SRC.to_string());
            record_apply(state, &target.node_id, &url, &mut run.commands);
            acted.push(target.node_id);
        }
    }

    // End-state accounting, mirroring the CLI's `enrich_state_with_session`:
    // `unresolved` = still-empty slots, `failed` = slots carrying the
    // failure sentinel, `resolved` = the remainder.
    let remaining: std::collections::HashSet<NodeId> = collect_targets(state, &Default::default())
        .into_iter()
        .map(|target| target.node_id)
        .collect();
    for node_id in &acted {
        if node_has_failure_sentinel(state, node_id) {
            run.summary.failed += 1;
        } else if remaining.contains(node_id) {
            run.summary.unresolved += 1;
        }
    }
    // Targets the deadline prevented us from starting were never touched, so
    // they are still empty slots and count as unresolved.
    run.summary.unresolved += run.summary.targets.saturating_sub(acted.len());
    run.summary.resolved = run
        .summary
        .targets
        .saturating_sub(run.summary.failed + run.summary.unresolved);
    run
}

/// The ids of every subtree under the requested roots, so a scoped run can
/// filter targets by membership.
fn subtree_ids_of(state: &EditorState, roots: &[String]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for root in roots {
        let Some(node) = walkers::find_node(state.active_children(), &NodeId::new(root.clone()))
        else {
            continue;
        };
        collect_subtree_ids(node, &mut out);
    }
    out
}

fn collect_subtree_ids(node: &PenNode, out: &mut std::collections::HashSet<String>) {
    out.insert(node.id_str().to_string());
    if let Some(children) = node.children() {
        for child in children {
            collect_subtree_ids(child, out);
        }
    }
}

/// Land `url` on `node_id` through the shared `apply_result`, then record the
/// change as a `PatchNodeData` carrying exactly the top-level keys that
/// changed — so the host's replay of the batch reproduces the clone's end
/// state without any parallel write path.
fn record_apply(
    state: &mut EditorState,
    node_id: &NodeId,
    url: &str,
    commands: &mut Vec<EditorCommand>,
) {
    let Some(before) = walkers::find_node(state.active_children(), node_id).cloned() else {
        return;
    };
    if !apply_result(state, node_id, url) {
        return;
    }
    let Some(after) = walkers::find_node(state.active_children(), node_id) else {
        return;
    };
    let (Ok(before_value), Ok(after_value)) =
        (serde_json::to_value(before), serde_json::to_value(after))
    else {
        return;
    };
    let (Some(before_obj), Some(after_obj)) = (before_value.as_object(), after_value.as_object())
    else {
        return;
    };
    let mut patch = serde_json::Map::new();
    for (key, value) in after_obj {
        if before_obj.get(key) != Some(value) {
            patch.insert(key.clone(), value.clone());
        }
    }
    if patch.is_empty() {
        return;
    }
    commands.push(EditorCommand::PatchNodeData {
        node_id: node_id.clone(),
        patch_json: serde_json::Value::Object(patch).to_string(),
        page_id: None,
    });
}

/// Whether a node carries the failed-search sentinel — the CLI's
/// `node_contains_failure_sentinel`, on image nodes and image fills alike.
fn node_has_failure_sentinel(state: &EditorState, node_id: &NodeId) -> bool {
    walkers::find_node(state.active_children(), node_id).is_some_and(node_contains_failure_sentinel)
}

fn node_contains_failure_sentinel(node: &PenNode) -> bool {
    match node {
        PenNode::Image(image) => image.src == SEARCH_FAILED_PLACEHOLDER_SRC,
        PenNode::Frame(frame) => fills_have_failure_sentinel(frame.container.fill.as_deref()),
        PenNode::Rectangle(rectangle) => {
            fills_have_failure_sentinel(rectangle.container.fill.as_deref())
        }
        _ => false,
    }
}

fn fills_have_failure_sentinel(fills: Option<&[PenFill]>) -> bool {
    fills.is_some_and(|fills| {
        fills.iter().any(|fill| {
            matches!(
                fill,
                PenFill::Image(image) if image.url == SEARCH_FAILED_PLACEHOLDER_SRC
            )
        })
    })
}
