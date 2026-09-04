//! Mobile image-search enrichment pump — the FFI counterpart of the desktop
//! `ImageSearchSession` (`op-host-desktop/src/image_search_session.rs`).
//!
//! A design run leaves `G(id, "search", "subject")` slots behind as
//! `PenNode::Image { src: "", image_search_query }`. On desktop a redraw-time
//! session detects those slots (`op_image_enrich::collect_targets_with_scene`),
//! resolves each through the shared provider ladder
//! (`op_image_enrich::net::fetch` — Openverse → two-keyword retry →
//! Wikimedia, materialized as a self-contained `data:` URL with skia
//! down-scaling), and writes the result back (`op_image_enrich::apply_result`).
//! Until this pump existed nothing on iOS / Android did that, so every
//! searched image slot stayed an empty grey frame.
//!
//! Deliberate deltas from desktop (documented, not drift):
//! - no generation jobs — mobile has no image-generation backend, so
//!   `Generate`-only slots resolve to the failed-search placeholder the same
//!   way desktop's `spawn_unavailable_gen_job` does;
//! - one-shot per (node, query) intent instead of the desktop's cross-run
//!   search memo — the `used_urls` de-dup set is kept, so sibling cards
//!   still get distinct images.
//!
//! Jobs run on plain `std::thread` workers; results land on the engine
//! thread during `op_frame`, mirroring `editor_chat`.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use op_editor_core::NodeId;
use op_host_native::WidgetHostNative;
use op_image_enrich::net::fetch::fetch_first_image_url_blocking;
use op_image_enrich::net::providers::WebOpenverseCredentials;
use op_image_enrich::{
    apply_result, collaboration_image_result_gate, collect_targets_with_scene, ImageRequestMode,
    ImageSearchTarget, SEARCH_FAILED_PLACEHOLDER_SRC,
};

use crate::lifecycle::Session;

/// Engine-thread repoll cadence while image jobs are in flight.
const IMAGE_POLL_INTERVAL_MS: u64 = 120;

/// Injectable resolver so FFI tests run hermetically (the default hits the
/// real provider ladder). Returns the resolved `data:`/`http(s)` URL, or
/// `None` for a failed search.
type ImageFetcher = fn(
    &ImageSearchTarget,
    Option<&WebOpenverseCredentials>,
    &Mutex<HashSet<String>>,
) -> Option<String>;

fn default_fetcher(
    target: &ImageSearchTarget,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    fetch_first_image_url_blocking(&target.query, target.aspect_ratio, credentials, used_urls)
}

struct ImageJob {
    node_id: NodeId,
    /// The query this job was enqueued for — a stale result (the node was
    /// edited/regenerated meanwhile) is discarded instead of applied.
    query: String,
    rx: Receiver<Option<String>>,
}

pub(crate) struct MobileImageSearch {
    jobs: Vec<ImageJob>,
    /// Node ids already enqueued or resolved this document generation, per
    /// query — feeds `collect_targets_with_scene`'s skip set and stops a
    /// failed search from being retried every frame.
    handled: HashMap<String, String>,
    /// Cross-job de-dup so sibling cards get distinct images (desktop
    /// `used_urls` parity).
    used_urls: Arc<Mutex<HashSet<String>>>,
    /// Perf gate (desktop parity): rescan only when the document or the
    /// active page changed.
    last_scan: Option<(u64, usize)>,
    fetcher: ImageFetcher,
}

impl Default for MobileImageSearch {
    fn default() -> Self {
        Self {
            jobs: Vec::new(),
            handled: HashMap::new(),
            used_urls: Arc::new(Mutex::new(HashSet::new())),
            last_scan: None,
            fetcher: default_fetcher,
        }
    }
}

impl MobileImageSearch {
    pub(crate) fn has_background_work(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Drop the result receivers for every in-flight enrichment request.
    /// Plain worker threads may finish their blocking fetch, but their sends
    /// then fail and no cancelled result can land in the document. Keep the
    /// handled/scan sets intact so the next foreground frame does not
    /// immediately recreate work the user cancelled through system UI.
    pub(crate) fn cancel_background_work(&mut self) {
        self.jobs.clear();
    }

    #[cfg(test)]
    pub(crate) fn with_fetcher(fetcher: ImageFetcher) -> Self {
        Self {
            fetcher,
            ..Self::default()
        }
    }

    /// Forget everything bound to the current document — MUST run whenever
    /// the document is replaced wholesale, for the same node-id-aliasing
    /// reason as the desktop session's `reset` (a stale in-flight result
    /// would otherwise land on an unrelated node that reused the id).
    pub(crate) fn reset(&mut self) {
        self.jobs.clear();
        self.handled.clear();
        self.used_urls.lock().map(|mut set| set.clear()).ok();
        self.last_scan = None;
    }

    /// Detect unresolved image slots and spawn one fetch job per new slot,
    /// then fold finished jobs back into the document. Returns the next
    /// engine-thread poll deadline while jobs are in flight.
    pub(crate) fn pump(&mut self, host: &mut WidgetHostNative, now_ms: u64) -> Option<u64> {
        let mut changed = self.enqueue_missing(host);
        changed |= self.poll_into(host);
        if changed {
            host.mark_editor_state_dirty();
        }
        (!self.jobs.is_empty()).then(|| now_ms.saturating_add(IMAGE_POLL_INTERVAL_MS))
    }

    fn enqueue_missing(&mut self, host: &mut WidgetHostNative) -> bool {
        let state = host.editor_state();
        if collaboration_image_result_gate(state).is_err() {
            return false;
        }
        let scan_key = (state.document_revision(), state.ui.active_page_index);
        if self.last_scan == Some(scan_key) {
            return false;
        }
        self.last_scan = Some(scan_key);
        let known: HashSet<String> = self.handled.keys().cloned().collect();
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
        let targets = collect_targets_with_scene(state, &known, &scene);
        let mut changed = false;
        for target in targets {
            let id = target.node_id.as_str().to_string();
            if self.handled.contains_key(&id) {
                continue;
            }
            self.handled.insert(id, target.query.clone());
            // Generation-only slots: no mobile generation backend — resolve
            // to the failed-search placeholder immediately (desktop
            // `spawn_unavailable_gen_job` parity) instead of leaving an
            // eternally-pending grey slot.
            if target.mode == ImageRequestMode::Generate {
                let state = host.editor_state_mut();
                changed |= apply_result(state, &target.node_id, SEARCH_FAILED_PLACEHOLDER_SRC);
                eprintln!(
                    "openpencil-mobile: image slot {} needs generation (unavailable on this device)",
                    target.node_id.as_str()
                );
                continue;
            }
            let credentials = openverse_credentials(host.editor_state());
            let (tx, rx) = mpsc::channel::<Option<String>>();
            let used_urls = Arc::clone(&self.used_urls);
            let fetcher = self.fetcher;
            let job_target = target.clone();
            let query = target.query.clone();
            std::thread::spawn(move || {
                let url = fetcher(&job_target, credentials.as_ref(), &used_urls);
                let _ = tx.send(url);
            });
            eprintln!(
                "openpencil-mobile: image search start ({} -> {:?})",
                target.node_id.as_str(),
                query
            );
            self.jobs.push(ImageJob {
                node_id: target.node_id,
                query,
                rx,
            });
        }
        changed
    }

    fn poll_into(&mut self, host: &mut WidgetHostNative) -> bool {
        let mut changed = false;
        let mut index = 0;
        while index < self.jobs.len() {
            match self.jobs[index].rx.try_recv() {
                Ok(url) => {
                    let job = self.jobs.swap_remove(index);
                    // Stale-intent guard (desktop fingerprint parity, light
                    // form): only apply when the node still asks for the
                    // exact query this job was spawned for.
                    let state = host.editor_state_mut();
                    if node_still_wants(state, &job.node_id, &job.query) {
                        let resolved =
                            url.unwrap_or_else(|| SEARCH_FAILED_PLACEHOLDER_SRC.to_string());
                        let applied = apply_result(state, &job.node_id, &resolved);
                        eprintln!(
                            "openpencil-mobile: image search done ({} applied={} failed={})",
                            job.node_id.as_str(),
                            applied,
                            resolved == SEARCH_FAILED_PLACEHOLDER_SRC
                        );
                        changed |= applied;
                    }
                }
                Err(TryRecvError::Empty) => index += 1,
                Err(TryRecvError::Disconnected) => {
                    let job = self.jobs.swap_remove(index);
                    let state = host.editor_state_mut();
                    if node_still_wants(state, &job.node_id, &job.query) {
                        changed |= apply_result(state, &job.node_id, SEARCH_FAILED_PLACEHOLDER_SRC);
                    }
                }
            }
        }
        changed
    }
}

fn openverse_credentials(state: &op_editor_core::EditorState) -> Option<WebOpenverseCredentials> {
    let settings = &state.editor_ui.agent_settings;
    WebOpenverseCredentials::from_parts(
        &settings.openverse_client_id,
        &settings.openverse_client_secret,
    )
}

/// True when `node_id` still names an unresolved image slot with `query` as
/// its search intent.
fn node_still_wants(state: &op_editor_core::EditorState, node_id: &NodeId, query: &str) -> bool {
    let Some(node) = op_editor_core::walkers::find_node(state.active_children(), node_id) else {
        return false;
    };
    match node {
        jian_ops_schema::node::PenNode::Image(image) => {
            image.src.as_str().trim().is_empty()
                && image.image_search_query.as_deref() == Some(query)
        }
        // Frame / rectangle empty-image-fill slots keep no query field; the
        // slot shape itself is the intent.
        _ => true,
    }
}

impl Session {
    pub(crate) fn pump_editor_image_search(&mut self, now_ms: u64) -> Option<u64> {
        let Session {
            editor,
            image_search,
            ..
        } = self;
        editor
            .as_mut()
            .and_then(|host| image_search.pump(host, now_ms))
    }
}

#[cfg(test)]
#[path = "editor_image_search_tests.rs"]
mod tests;
