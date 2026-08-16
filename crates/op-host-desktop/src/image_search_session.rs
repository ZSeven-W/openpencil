//! Background image-search enrichment for generated image nodes.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use op_editor_core::agent_settings::ImageGenProfile;
use op_editor_core::{EditorState, NodeId};
// Provider plumbing shared with the web daemon (single-sourced in
// op-host-services): keyword simplification, Openverse token exchange, and
// image mime handling. The desktop keeps its own `fetch_image_data_url`
// on top of `fetch_image_bytes` so the skia down-scale pass still runs.
pub(crate) use op_host_services::web_image_search::{
    fetch_openverse_token, normalize_image_mime_header, simplify_search_query,
    WebOpenverseCredentials,
};
// Only the sibling test file exercises the sniffing directly.
#[cfg(test)]
pub(crate) use op_host_services::web_image_search::sniff_image_mime;

const MAX_EMBEDDED_IMAGE_BYTES: usize = 4 * 1024 * 1024;

// Slot detection + result write-back moved to the shared `op-image-enrich`
// crate (pure code motion) so the headless MCP `enrich_images` tool and this
// desktop session share one predicate vocabulary; the desktop keeps its own
// provider fetches, memoization, and job bookkeeping. The old paths are
// re-exported so every existing importer and test stays stable.
pub(crate) use op_image_enrich::{
    apply_result, collaboration_image_result_gate, collect_targets, collect_targets_with_scene,
    image_request_mode, ImageAspectRatio, ImageRequestMode, ImageSearchTarget,
    SEARCH_FAILED_PLACEHOLDER_SRC,
};

/// Desktop wrapper over the shared credential pair — adds the
/// `EditorState` snapshot constructor the daemon side has no use for.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OpenverseCredentials(WebOpenverseCredentials);

impl OpenverseCredentials {
    pub(crate) fn from_state(state: &EditorState) -> Option<Self> {
        let settings = &state.editor_ui.agent_settings;
        WebOpenverseCredentials::from_parts(
            &settings.openverse_client_id,
            &settings.openverse_client_secret,
        )
        .map(Self)
    }

    pub(crate) fn as_web(&self) -> &WebOpenverseCredentials {
        &self.0
    }
}

struct ImageSearchJob {
    node_id: NodeId,
    /// Exact node intent at enqueue time. Production jobs always set this;
    /// hand-built unit jobs may omit it when testing unrelated bookkeeping.
    intent: Option<String>,
    rx: Receiver<Option<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SearchIntentKey {
    query: String,
    aspect_ratio: Option<ImageAspectRatio>,
}

enum SearchMemoEntry {
    Pending {
        request_id: u64,
        waiters: Vec<mpsc::Sender<Option<String>>>,
    },
    Ready(String),
}

#[derive(Default)]
pub(crate) struct ImageSearchSession {
    in_flight: HashSet<String>,
    completed: HashSet<String>,
    jobs: Vec<ImageSearchJob>,
    /// `(document revision, active page index)` at the last
    /// `enqueue_missing` walk. When unchanged, the tree walk is skipped —
    /// `enqueue_missing` runs on every `RedrawRequested`, so this gate keeps
    /// idle frames from re-walking the whole active page. Cleared whenever
    /// `in_flight`/`completed` mutate outside `enqueue_missing` (job
    /// completion/failure, session reset) so those events force one rescan
    /// even though the document revision did not move.
    ///
    /// The active page index is part of the key because `enqueue_missing`
    /// only walks the ACTIVE page (`collect_targets` → `state.active_children()`),
    /// while `set_active_page` / page reorder / page removal (see
    /// `op-editor-core/src/page_mutators.rs`) mutate `ui.active_page_index`
    /// WITHOUT bumping `document_revision` — a page switch is a UI-state
    /// change, not a document-content mutation. Keying on revision alone
    /// would make switching from a scanned page A to an unscanned page B at
    /// the same revision silently skip page B forever.
    last_scanned: Option<(u64, usize)>,
    /// Test-only: counts how many times `enqueue_missing` has actually
    /// walked the tree (as opposed to short-circuiting on the revision
    /// gate), so tests can assert the gate skips redundant walks instead of
    /// relying on `enqueue_missing`'s return value (which is already
    /// `false` on a repeat call for a reason unrelated to the gate: the
    /// target is already known via `in_flight`/`completed`).
    #[cfg(test)]
    scan_count: u32,
    /// Test-only: counts coherent current-intent snapshots built by
    /// `poll_into`. A batch of completed jobs must share one snapshot instead
    /// of rebuilding the active-page scene once per job.
    #[cfg(test)]
    stale_intent_scan_count: u32,
    /// Canonical provider identities plus compact content digests already used
    /// this session. Similar queries otherwise share their Openverse top hit
    /// and fill several cards with the same artwork (measured: test0711-22).
    /// Never stores full embedded data URLs.
    used_urls: Arc<Mutex<HashSet<String>>>,
    /// Full stock-search intent → pending waiters or the resolved photo.
    ///
    /// The dedup above must NOT fire when the SAME subject comes back: the
    /// model rebuilds a section mid-run (fresh node ids), the same query
    /// ("Bali Indonesia") searches again, and its own good photo is now in
    /// the used-image set — so the second search skipped it and took a junk result
    /// instead (measured 2026-07-12: a real Bali temple photo turned into a
    /// plain blue sky halfway through a run). One subject, one photo: a repeat
    /// query resolves from this memo with no network call at all.
    search_memo: Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>>,
    /// Monotonic identity for a memo fetch. It is deliberately not reset:
    /// an old network thread may finish after `reset()` and after the new
    /// document has enqueued the same intent. Matching the request id avoids
    /// that old completion consuming the new waiters (the classic ABA race).
    next_search_request_id: u64,
}

// Memo key for the authored stock-search intent — see
// `intent::search_intent_key` (moved there with the other intent
// fingerprints as pure code motion).

impl ImageSearchSession {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Full teardown of session state. MUST be the call used by every
    /// whole-document-replacement path (Figma import, MCP `ReplaceDocument`),
    /// not just `invalidate_scan_gate()` below: those replacements install a
    /// fresh `EditorState` whose node ids can collide with ids from the
    /// document that was just replaced (both start their id allocator over).
    /// Invalidating only the scan gate would leave `in_flight` / `completed`
    /// aliased against the OLD document, which then either (a) silently
    /// suppresses a same-id target in the NEW document (still `completed`),
    /// or (b) lets an old in-flight job apply its stale result to a same-id
    /// node in the NEW document once it resolves (`poll_into` matches by raw
    /// node id, not by document identity). Clearing `jobs` too drops any
    /// still-pending job outright so it can never reach `poll_into` again.
    pub(crate) fn reset(&mut self) {
        self.in_flight.clear();
        self.completed.clear();
        self.jobs.clear();
        self.invalidate_scan_gate();
        // Replace the generations, do not merely clear them. Detached network
        // threads still own the old Arcs and may finish after reset; writing to
        // those abandoned maps must not contaminate the replacement document.
        self.used_urls = Arc::new(Mutex::new(HashSet::new()));
        self.search_memo = Arc::new(Mutex::new(HashMap::new()));
    }

    pub(crate) fn is_pending(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Re-admit terminal stock-search failures for a bounded caller-managed
    /// retry. Callers must restore only Search/Auto nodes before invoking this;
    /// explicit Generate targets are intentionally never re-admitted here.
    pub(crate) fn retry_search_failures(&mut self, node_ids: &HashSet<NodeId>) {
        for node_id in node_ids {
            self.completed.remove(node_id.as_str());
        }
        self.invalidate_scan_gate();
    }

    /// Force the next `enqueue_missing` to re-walk the tree even when the
    /// `(revision, active page)` key looks unchanged (e.g. because a fresh
    /// `EditorState` restarts its revision at 0 and its active page index at
    /// 0, aliasing the previously scanned key).
    ///
    /// This clears ONLY the gate — `in_flight` / `completed` / `jobs` stay
    /// as-is. That is correct for job-completion / failure bookkeeping
    /// (`poll_into`'s two mutating arms, below) where those sets legitimately
    /// still describe the current document. It is NOT sufficient on its own
    /// for a whole-document replacement, where a fresh `EditorState`'s node
    /// ids can alias ids from the document just replaced — use `reset()` for
    /// that, which clears the sets/jobs too and calls this as its last step.
    /// Intentionally private: every whole-document-replacement call site
    /// (Figma import, MCP `ReplaceDocument`) must go through `reset()`
    /// instead.
    fn invalidate_scan_gate(&mut self) {
        self.last_scanned = None;
    }

    #[cfg(test)]
    pub(crate) fn enqueue_missing(&mut self, state: &EditorState) -> bool {
        self.enqueue_missing_with_optional_scene(state, None)
    }

    /// Production redraw path: reuse the host's already-current active-page
    /// scene instead of resolving the same page a second time just to obtain
    /// image-slot dimensions. This matters for very large Figma pages where a
    /// duplicate scene build can add hundreds of milliseconds per switch.
    pub(crate) fn enqueue_missing_with_scene(
        &mut self,
        state: &EditorState,
        scene: &op_editor_ui::layout_scene::LayoutScene,
    ) -> bool {
        self.enqueue_missing_with_optional_scene(state, Some(scene))
    }

    fn enqueue_missing_with_optional_scene(
        &mut self,
        state: &EditorState,
        scene: Option<&op_editor_ui::layout_scene::LayoutScene>,
    ) -> bool {
        // Automatic enrichment is an external-asset mutation, which M1 does
        // not admit into a bound collaboration document. Check before
        // starting network/provider work; do not show a rejection merely
        // because a collaborative canvas contains an empty placeholder.
        if collaboration_image_result_gate(state).is_err() {
            return false;
        }
        // Perf gate: this runs on every `RedrawRequested`. Skip the whole-tree
        // walk when the document content AND active page are unchanged since
        // the last scan, and no session-set mutation (job completion/failure,
        // reset) invalidated the gate in the meantime. The walk only covers
        // the active page (`collect_targets` below), so the active page index
        // must be part of the key — see `last_scanned`'s doc comment.
        let key = (state.document_revision(), state.ui.active_page_index);
        if self.last_scanned == Some(key) {
            return false;
        }
        self.last_scanned = Some(key);
        #[cfg(test)]
        {
            self.scan_count += 1;
        }
        let mut known = self.completed.clone();
        known.extend(self.in_flight.iter().cloned());
        let targets = match scene {
            Some(scene) => collect_targets_with_scene(state, &known, scene),
            None => collect_targets(state, &known),
        };
        if targets.is_empty() {
            return false;
        }
        // An explicit G(...,"search"|"generate",...) mode wins. Legacy and
        // heuristic slots remain Auto: configured generation first, otherwise
        // stock search. A generate request without a configured provider fails
        // visibly; it is never silently changed into a stock-photo request.
        let gen_profile = crate::image_panel_host::active_image_gen_profile(state).cloned();
        let credentials = OpenverseCredentials::from_state(state);
        for target in targets {
            let id = target.node_id.as_str().to_string();
            self.in_flight.insert(id);
            let job = match (target.mode, &gen_profile) {
                (ImageRequestMode::Generate, Some(profile))
                | (ImageRequestMode::Auto, Some(profile)) => spawn_gen_job(target, profile.clone()),
                (ImageRequestMode::Generate, None) => spawn_unavailable_gen_job(target),
                (ImageRequestMode::Search, _) | (ImageRequestMode::Auto, None) => {
                    let request_id = self.next_search_request_id;
                    self.next_search_request_id = self.next_search_request_id.wrapping_add(1);
                    spawn_job(
                        target,
                        credentials.clone(),
                        Arc::clone(&self.used_urls),
                        Arc::clone(&self.search_memo),
                        request_id,
                    )
                }
            };
            self.jobs.push(job);
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn poll_into(&mut self, state: &mut EditorState) -> bool {
        self.poll_into_with_optional_scene(state, None)
    }

    /// Production redraw path: validate every completed job against one
    /// coherent intent snapshot derived from the host's already-current
    /// active-page scene. This avoids rebuilding a large Figma page once per
    /// completed image job while preserving the stale-result guard.
    pub(crate) fn poll_into_with_scene(
        &mut self,
        state: &mut EditorState,
        scene: &op_editor_ui::layout_scene::LayoutScene,
    ) -> bool {
        self.poll_into_with_optional_scene(state, Some(scene))
    }

    fn poll_into_with_optional_scene(
        &mut self,
        state: &mut EditorState,
        scene: Option<&op_editor_ui::layout_scene::LayoutScene>,
    ) -> bool {
        let mut changed = false;
        let mut i = 0;
        // Production jobs always carry an intent. Build this lazily only when
        // at least one such job is actually ready, then reuse it for every
        // other completion drained by this poll. Pending-only frames stay at
        // zero target walks.
        let mut current_intents: Option<HashMap<String, String>> = None;
        while i < self.jobs.len() {
            match self.jobs[i].rx.try_recv() {
                Ok(url) => {
                    let job = self.jobs.swap_remove(i);
                    let id = job.node_id.as_str().to_string();
                    self.in_flight.remove(&id);
                    // `in_flight`/`completed` are mutated outside
                    // `enqueue_missing`, and a failed job updates them without
                    // any document-content change (so no revision bump) —
                    // invalidate the scan gate so the next `enqueue_missing`
                    // re-walks once. (A successful `apply_result` DOES bump the
                    // revision, but the gate invalidation still covers the
                    // failure path.)
                    self.last_scanned = None;
                    // A job may have started while the document was standalone
                    // and become ready only after Start/Join. Re-check the
                    // current phase and role immediately before the raw node
                    // mutation, discard the result on rejection, and keep it
                    // retryable after the collaboration session is left.
                    if let Err(reason) = collaboration_image_result_gate(state) {
                        state.editor_ui.collab.set_notice(reason.notice_kind(), 0);
                        changed = true;
                        continue;
                    }
                    if job.intent.is_some() && current_intents.is_none() {
                        #[cfg(test)]
                        {
                            self.stale_intent_scan_count += 1;
                        }
                        current_intents = Some(current_intent_fingerprints(state, scene));
                    }
                    if job.intent.as_ref().is_some_and(|expected| {
                        current_intents
                            .as_ref()
                            .and_then(|intents| intents.get(job.node_id.as_str()))
                            != Some(expected)
                    }) {
                        tracing::info!(
                            node_id = %job.node_id,
                            "discarding stale image result because the node's image intent changed"
                        );
                        continue;
                    }
                    // Ours: a failed search lands the theme-adaptive dashed
                    // placeholder (sentinel src) instead of a bare grey box.
                    let url = url.unwrap_or_else(|| SEARCH_FAILED_PLACEHOLDER_SRC.to_string());
                    if apply_result(state, &job.node_id, &url) {
                        changed = true;
                        if url == SEARCH_FAILED_PLACEHOLDER_SRC {
                            self.completed.insert(id);
                        }
                    } else {
                        self.completed.insert(id);
                    }
                }
                Err(TryRecvError::Empty) => {
                    i += 1;
                }
                Err(TryRecvError::Disconnected) => {
                    let job = self.jobs.swap_remove(i);
                    let id = job.node_id.as_str().to_string();
                    self.in_flight.remove(&id);
                    self.completed.insert(id);
                    // Same invalidation as the Ok arm: a failed job mutated the
                    // session sets, so force one rescan.
                    self.last_scanned = None;
                }
            }
        }
        changed
    }
}

fn spawn_job(
    target: ImageSearchTarget,
    credentials: Option<OpenverseCredentials>,
    used_urls: Arc<Mutex<HashSet<String>>>,
    search_memo: Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>>,
    request_id: u64,
) -> ImageSearchJob {
    let (tx, rx) = mpsc::channel();
    let node_id = target.node_id.clone();
    let aspect_ratio = target.aspect_ratio;
    let key = search_intent_key(&target.query, aspect_ratio);
    let intent = Some(intent_fingerprint(&target, None));
    // One full search intent, one in-flight request and one session result.
    // Rebuilt nodes subscribe to the same pending request instead of racing
    // duplicate searches; completed intents return from the memo.
    {
        let mut memo = search_memo.lock().unwrap();
        match memo.get_mut(&key) {
            Some(SearchMemoEntry::Ready(url)) => {
                let _ = tx.send(Some(url.clone()));
                return ImageSearchJob {
                    node_id,
                    intent,
                    rx,
                };
            }
            Some(SearchMemoEntry::Pending { waiters, .. }) => {
                waiters.push(tx);
                return ImageSearchJob {
                    node_id,
                    intent,
                    rx,
                };
            }
            None => {
                memo.insert(
                    key.clone(),
                    SearchMemoEntry::Pending {
                        request_id,
                        waiters: vec![tx],
                    },
                );
            }
        }
    }
    std::thread::spawn(move || {
        let url = fetch_first_image_url_blocking(
            &target.query,
            aspect_ratio,
            credentials.as_ref(),
            &used_urls,
        );
        publish_search_result(&search_memo, key, request_id, url);
    });
    ImageSearchJob {
        node_id,
        intent,
        rx,
    }
}

/// Publish only into the exact Pending entry that launched this request. A
/// reset may remove it and a new document may insert the same key before the
/// old thread returns; the request id keeps that old result isolated.
fn publish_search_result(
    search_memo: &Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>>,
    key: SearchIntentKey,
    request_id: u64,
    url: Option<String>,
) -> bool {
    let waiters = {
        let mut memo = search_memo.lock().unwrap();
        let matches_request = matches!(
            memo.get(&key),
            Some(SearchMemoEntry::Pending {
                request_id: pending_id,
                ..
            }) if *pending_id == request_id
        );
        if !matches_request {
            return false;
        }
        let Some(SearchMemoEntry::Pending { waiters, .. }) = memo.remove(&key) else {
            unreachable!("request identity was checked under the same lock")
        };
        if let Some(found) = url.as_ref() {
            memo.insert(key, SearchMemoEntry::Ready(found.clone()));
        }
        waiters
    };
    for waiter in waiters {
        let _ = waiter.send(url.clone());
    }
    true
}

/// Enrich via the configured image-GEN model instead of stock search. Prefers the
/// node's bound `image_prompt`; falls back to the search keyword so a node that
/// only carries `image_search_query` still generates. Same `ImageSearchJob`
/// channel as search, so `poll_into` applies the resulting src identically.
fn spawn_gen_job(target: ImageSearchTarget, profile: ImageGenProfile) -> ImageSearchJob {
    let (tx, rx) = mpsc::channel();
    let node_id = target.node_id.clone();
    let intent = Some(intent_fingerprint(&target, Some(&profile)));
    std::thread::spawn(move || {
        let prompt = target
            .prompt
            .as_deref()
            .filter(|p| !p.trim().is_empty())
            .unwrap_or(target.query.as_str());
        let url = crate::image_generate_host::run_generate_blocking(
            prompt,
            &profile,
            target.width,
            target.height,
        )
        .ok();
        let _ = tx.send(url);
    });
    ImageSearchJob {
        node_id,
        intent,
        rx,
    }
}

fn spawn_unavailable_gen_job(target: ImageSearchTarget) -> ImageSearchJob {
    let (tx, rx) = mpsc::channel();
    let node_id = target.node_id.clone();
    let intent = Some(intent_fingerprint(&target, None));
    let _ = tx.send(None);
    ImageSearchJob {
        node_id,
        intent,
        rx,
    }
}
mod fetch;
mod intent;

use fetch::fetch_first_image_url_blocking;
pub(crate) use fetch::fetch_image_data_url;
pub(crate) use intent::{current_intent_fingerprints, intent_fingerprint, search_intent_key};

// `apply_result` + the slot predicates + the target/mode/aspect types live in
// the shared `op-image-enrich` crate now (see the re-export block at the top
// of this file); the imports here cover only the desktop-local helpers.

#[cfg(test)]
#[path = "image_search_session_tests.rs"]
mod tests;
