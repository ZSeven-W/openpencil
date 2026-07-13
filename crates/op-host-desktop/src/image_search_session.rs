//! Background image-search enrichment for generated image nodes.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use jian_ops_schema::node::{PenNode, TextContent};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{ImageFillBody, ImageFillMode, PenFill};
use op_editor_core::agent_settings::ImageGenProfile;
use op_editor_core::{walkers, EditorState, NodeId, PenNodeExt as _};
use reqwest::header::CONTENT_TYPE;

const MAX_EMBEDDED_IMAGE_BYTES: usize = 4 * 1024 * 1024;

/// Sentinel `src` written when EVERY search avenue failed (junk-only
/// results, network error, empty corpus). The canvas paints it as the
/// theme-adaptive dashed placeholder (see `canvas_viewport_image`), so a
/// failed slot reads as "image goes here" instead of a bare grey box. The
/// bound query stays on the node for manual re-search.
pub(crate) const SEARCH_FAILED_PLACEHOLDER_SRC: &str = "placeholder://image-search-failed";
/// Design-artifact words that are pure noise against a PHOTO library: an
/// open-license corpus has no "album covers" or "playlist art" — those are
/// design deliverables, not photographed subjects. Stripping them turns
/// "synthwave album cover neon" into "synthwave neon", which the corpus DOES
/// cover (measured: the artifact words matched magazine covers, flowers and
/// a van sticker across a whole music screen, test0711-22). The full prompt
/// stays on the node for the image-GEN path, which wants the artifact words.
const IMAGE_SEARCH_ARTIFACT_WORDS: &[&str] = &[
    "album",
    "cover",
    "playlist",
    "artwork",
    "poster",
    "thumbnail",
    "logo",
    "icon",
    "banner",
    "mockup",
    "screenshot",
    "wallpaper",
];

const IMAGE_SEARCH_STOP_WORDS: &[&str] = &[
    "a",
    "an",
    "the",
    "and",
    "or",
    "but",
    "in",
    "on",
    "at",
    "to",
    "for",
    "of",
    "with",
    "by",
    "from",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "have",
    "has",
    "had",
    "do",
    "does",
    "did",
    "will",
    "would",
    "could",
    "should",
    "may",
    "might",
    "shall",
    "can",
    "that",
    "this",
    "these",
    "those",
    "it",
    "its",
    "very",
    "really",
    "just",
    "also",
    "about",
    "above",
    "after",
    "before",
    "between",
    "into",
    "through",
    "during",
    "each",
    "some",
    "such",
    "no",
    "not",
    "only",
    "same",
    "so",
    "than",
    "too",
    "up",
    "out",
    "if",
    "then",
    "once",
    "here",
    "there",
    "when",
    "where",
    "how",
    "all",
    "both",
    "few",
    "more",
    "most",
    "other",
    "any",
    "as",
    "while",
    "using",
    "showing",
    "featuring",
    "looking",
    "style",
    "styled",
    "inspired",
    "based",
];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImageSearchTarget {
    pub node_id: NodeId,
    /// Stock-search keyword (`image_search_query` ?? name ?? …).
    pub query: String,
    pub aspect_ratio: Option<ImageAspectRatio>,
    /// AI-generation prompt bound to the node (`image_prompt`), if any. Used when
    /// an image-gen model is configured; falls back to `query`.
    pub prompt: Option<String>,
    /// Explicit `G()` acquisition mode, or Auto for legacy/heuristic slots.
    pub mode: ImageRequestMode,
    /// Resolved numeric dimensions (for the gen provider's aspect mapping).
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageRequestMode {
    Auto,
    Search,
    Generate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ImageAspectRatio {
    Wide,
    Tall,
    Square,
}

impl ImageAspectRatio {
    fn as_openverse_param(self) -> &'static str {
        match self {
            Self::Wide => "wide",
            Self::Tall => "tall",
            Self::Square => "square",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OpenverseCredentials {
    client_id: String,
    client_secret: String,
}

impl OpenverseCredentials {
    pub(crate) fn from_state(state: &EditorState) -> Option<Self> {
        let settings = &state.editor_ui.agent_settings;
        let client_id = settings.openverse_client_id.trim();
        let client_secret = settings.openverse_client_secret.trim();
        if client_id.is_empty() || client_secret.is_empty() {
            None
        } else {
            Some(Self {
                client_id: client_id.to_string(),
                client_secret: client_secret.to_string(),
            })
        }
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
struct SearchIntentKey {
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

/// Memo key for the authored stock-search intent.
///
/// This deliberately does NOT use `simplify_search_query`: that function is a
/// lossy provider adapter (it drops words such as `album` / `cover` and caps
/// the request at four keywords). Those transformations are useful for a
/// photo corpus, but they must not make two distinct authored subjects share a
/// cached image or make the stale-result guard treat a changed intent as the
/// same intent. Case, punctuation, and repeated whitespace are canonicalized;
/// every authored word remains part of identity. Aspect remains part of intent
/// so a square cover never reuses a wide hero.
fn search_intent_key(query: &str, aspect_ratio: Option<ImageAspectRatio>) -> SearchIntentKey {
    SearchIntentKey {
        query: canonical_search_intent_query(query),
        aspect_ratio,
    }
}

fn canonical_search_intent_query(query: &str) -> String {
    let mut canonical = String::with_capacity(query.len());
    let mut pending_separator = false;
    for character in query.trim().to_lowercase().chars() {
        if character.is_alphanumeric() {
            if pending_separator && !canonical.is_empty() {
                canonical.push(' ');
            }
            canonical.push(character);
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }
    if canonical.is_empty() {
        query.trim().to_lowercase()
    } else {
        canonical
    }
}

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

    pub(crate) fn enqueue_missing(&mut self, state: &EditorState) -> bool {
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
        let targets = collect_targets(state, &known);
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

    pub(crate) fn poll_into(&mut self, state: &mut EditorState) -> bool {
        let mut changed = false;
        let mut i = 0;
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
                    if job.intent.as_ref().is_some_and(|expected| {
                        current_intent_fingerprint(state, &job.node_id).as_ref() != Some(expected)
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

fn intent_fingerprint(target: &ImageSearchTarget, profile: Option<&ImageGenProfile>) -> String {
    let generate = target.mode == ImageRequestMode::Generate
        || (target.mode == ImageRequestMode::Auto && profile.is_some());
    if generate {
        let (profile_id, model) = profile
            .map(|profile| (profile.id.as_str(), profile.model.as_str()))
            .unwrap_or(("unconfigured", "unconfigured"));
        format!(
            "generate|{profile_id}|{model}|{}|{:?}|{:?}",
            target
                .prompt
                .as_deref()
                .filter(|prompt| !prompt.trim().is_empty())
                .unwrap_or(target.query.as_str())
                .trim(),
            target.width.map(f64::to_bits),
            target.height.map(f64::to_bits)
        )
    } else {
        let key = search_intent_key(&target.query, target.aspect_ratio);
        format!("search|{}|{:?}", key.query, key.aspect_ratio)
    }
}

fn current_intent_fingerprint(state: &EditorState, node_id: &NodeId) -> Option<String> {
    let target = collect_targets(state, &HashSet::new())
        .into_iter()
        .find(|target| &target.node_id == node_id)?;
    let profile = crate::image_panel_host::active_image_gen_profile(state);
    Some(intent_fingerprint(&target, profile))
}

pub(crate) fn collect_targets(
    state: &EditorState,
    known_node_ids: &HashSet<String>,
) -> Vec<ImageSearchTarget> {
    let resolved_sizes = resolved_node_sizes(state);
    let mut targets = Vec::new();
    collect_from_children(
        state.active_children(),
        known_node_ids,
        &resolved_sizes,
        &mut targets,
        &[],
    );
    targets
}

/// Resolved node dimensions from the real layout pass. This map is used only
/// after a node has independently qualified as media; geometry never turns an
/// anonymous surface into an image target. It lets a G()-created fill/fill
/// child inherit the actual slot aspect for search, generation, and stale-job
/// detection instead of losing that intent as `None`.
fn resolved_node_sizes(state: &EditorState) -> HashMap<String, (f64, f64)> {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let mut out = HashMap::new();
    fn walk(
        nodes: &[op_editor_ui::layout_scene::SceneNode],
        out: &mut HashMap<String, (f64, f64)>,
    ) {
        for node in nodes {
            let bounds = node.aggregate_bounds();
            if bounds.size.x > 0.0 && bounds.size.y > 0.0 {
                out.insert(
                    node.id.clone(),
                    (f64::from(bounds.size.x), f64::from(bounds.size.y)),
                );
            }
            walk(&node.children, out);
        }
    }
    for page in &scene.pages {
        walk(&page.children, &mut out);
    }
    out
}

fn collect_from_children(
    children: &[PenNode],
    known_node_ids: &HashSet<String>,
    resolved_sizes: &HashMap<String, (f64, f64)>,
    targets: &mut Vec<ImageSearchTarget>,
    parent_names: &[String],
) {
    // Direct sibling text of a bare anonymous slot may name its subject
    // ("Blinding Lights" next to a nameless 120px square = that track's
    // cover). Do not search sibling container subtrees or inherit text across
    // levels: at a rail/list boundary those words belong to cousin cards, not
    // to this slot.
    for (index, node) in children.iter().enumerate() {
        let context: Vec<String> = children
            .iter()
            .enumerate()
            .filter(|(other_index, _)| *other_index != index)
            .filter_map(|(_, other)| match other {
                PenNode::Text(text) => match &text.content {
                    TextContent::Plain(value) if !value.trim().is_empty() => {
                        Some(value.trim().to_string())
                    }
                    _ => None,
                },
                _ => None,
            })
            .take(2)
            .collect();

        if let Some(target) =
            image_search_target_for(node, known_node_ids, resolved_sizes, parent_names, &context)
        {
            targets.push(target);
        }

        if is_image_placeholder_frame(node)
            || is_image_area_frame_by_heuristic(node)
            || is_image_area_rectangle_by_heuristic(node)
        {
            continue;
        }
        if let Some(grand) = node.children() {
            let mut child_parent_names = Vec::with_capacity(parent_names.len() + 1);
            child_parent_names.push(node.base().name.clone().unwrap_or_default());
            child_parent_names.extend(parent_names.iter().cloned());
            collect_from_children(
                grand,
                known_node_ids,
                resolved_sizes,
                targets,
                &child_parent_names,
            );
        }
    }
}

fn image_search_target_for(
    node: &PenNode,
    known_node_ids: &HashSet<String>,
    resolved_sizes: &HashMap<String, (f64, f64)>,
    parent_names: &[String],
    sibling_text: &[String],
) -> Option<ImageSearchTarget> {
    let id = node.base().id.as_str();
    if known_node_ids.contains(id) {
        return None;
    }

    // Anonymous EMPTY solid square (>=48px, rounded/clipping) whose card
    // carries text siblings — DeepSeek V4 builds whole album grids this
    // way with no names and no G() bindings (measured test0711-2-ds); the
    // sibling text is the only, and a good, subject source.
    let bare_slot_with_context = !sibling_text.is_empty()
        && !has_non_media_context(parent_names)
        && is_bare_anonymous_slot(node);
    let needs_image = match node {
        PenNode::Image(image) => is_placeholder_src(&image.src),
        PenNode::Frame(_) => is_frame_placeholder_still_unfilled(node) || bare_slot_with_context,
        // (rectangles fall through to the arm below)
        PenNode::Rectangle(_) => {
            is_image_area_rectangle_by_heuristic(node)
                || is_unnamed_media_slot_in_context(node, parent_names)
                || bare_slot_with_context
        }
        _ => false,
    };
    if !needs_image {
        return None;
    }

    let mut query = extract_query_for_node(node, parent_names);
    if bare_slot_with_context {
        // The generic name-derived fallback ("placeholder") loses to the
        // card's own text; an EXPLICIT imageSearchQuery binding still wins.
        let explicit = match node {
            PenNode::Frame(f) => f
                .image_search_query
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_string(),
            _ => String::new(),
        };
        // Precedence: an explicit binding, then the nearest ancestor that NAMES
        // the subject (a card called "Bali, Indonesia"), and only then the
        // words found around the slot. Context text is the last resort because
        // it can only ever come from siblings — and a card with no words of its
        // own would otherwise borrow its neighbour's ("Santorini" on the Bali
        // card, measured while fixing this).
        query = if !explicit.is_empty() {
            explicit
        } else if let Some(name) = parent_semantic_name(parent_names) {
            name
        } else {
            sibling_text.join(" ")
        };
    }
    if query.is_empty() {
        return None;
    }

    // `image_prompt` is the author's AI-gen prompt (Image nodes only); placeholder
    // frames / rectangles carry only a name-derived query.
    let prompt = match node {
        PenNode::Image(image) => image.image_prompt.clone(),
        _ => None,
    };
    let mode = match node {
        // Legacy / script-generated Image nodes intentionally carry both
        // fields: generation uses the richer prompt when a profile exists,
        // otherwise stock search falls back to the query. `G("search")` and
        // `G("generate")` remain unambiguous because they emit only one field.
        PenNode::Image(image)
            if image
                .image_prompt
                .as_deref()
                .is_some_and(|prompt| !prompt.trim().is_empty())
                && image
                    .image_search_query
                    .as_deref()
                    .is_some_and(|query| !query.trim().is_empty()) =>
        {
            ImageRequestMode::Auto
        }
        PenNode::Image(image)
            if image
                .image_prompt
                .as_deref()
                .is_some_and(|prompt| !prompt.trim().is_empty()) =>
        {
            ImageRequestMode::Generate
        }
        PenNode::Image(image)
            if image
                .image_search_query
                .as_deref()
                .is_some_and(|query| !query.trim().is_empty()) =>
        {
            ImageRequestMode::Search
        }
        PenNode::Frame(frame)
            if frame
                .image_search_query
                .as_deref()
                .is_some_and(|query| !query.trim().is_empty()) =>
        {
            ImageRequestMode::Search
        }
        _ => ImageRequestMode::Auto,
    };
    let (width, height) = resolved_sizes
        .get(id)
        .copied()
        .map(|(width, height)| (Some(width), Some(height)))
        .unwrap_or_else(|| (node.width_px(), node.height_px()));
    Some(ImageSearchTarget {
        node_id: NodeId::new(id),
        query,
        aspect_ratio: infer_aspect_ratio(width, height),
        prompt,
        mode,
        width,
        height,
    })
}

fn is_placeholder_src(src: &str) -> bool {
    src.trim().is_empty() || src.starts_with("data:image/svg+xml;charset=utf-8,%3Csvg")
}

fn is_image_placeholder_frame(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_)) && node.base().role.as_deref() == Some("image-placeholder")
}

fn is_frame_placeholder_still_unfilled(node: &PenNode) -> bool {
    is_unfilled_image_placeholder_frame(node) || is_image_area_frame_by_heuristic(node)
}

fn is_unfilled_image_placeholder_frame(node: &PenNode) -> bool {
    if !is_image_placeholder_frame(node) {
        return false;
    }
    let PenNode::Frame(frame) = node else {
        return false;
    };
    match frame.container.fill.as_deref() {
        None | Some([]) => true,
        Some([PenFill::Image(_), ..]) => false,
        Some(_) => true,
    }
}

fn is_image_area_frame_by_heuristic(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    if frame.base.role.as_deref() == Some("image-placeholder") {
        return false;
    }
    let Some(name) = frame.base.name.as_deref() else {
        return false;
    };
    if !has_image_area_keyword(name) {
        return false;
    }
    if !is_image_area_size(&frame.container.width, &frame.container.height)
        && !is_small_thumb_size(&frame.container.width, &frame.container.height)
    {
        return false;
    }
    if !matches!(frame.container.fill.as_deref(), Some([PenFill::Solid(_)])) {
        return false;
    }
    let Some(children) = frame.children.as_ref() else {
        return true;
    };
    matches!(children.as_slice(), [] | [PenNode::IconFont(_)])
        || matches!(children.as_slice(), [only] if is_empty_unfilled_frame(only))
}

/// A bare structural stub inside a media slot (an empty fill×fill frame the
/// model left as "where the picture goes") must not disqualify the slot.
fn is_empty_unfilled_frame(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    frame.container.fill.is_none()
        && frame
            .children
            .as_ref()
            .is_none_or(|children| children.is_empty())
}

/// An UNNAMED small square solid rectangle inside a media-named ancestor
/// ("Mini Player" > bare 44×44 rectangle, measured test0711-2-ds) — the
/// name-keyword gate lives on the ANCESTOR chain, so the artwork slot the
/// model left anonymous still enriches. The query is derived from the
/// surrounding names/labels as usual.
/// Nameless, childless, solid, rounded/clipping, roughly-square slot of at
/// least thumbnail size — the shape signature of a cover box. Only ever
/// consulted when TEXT SIBLINGS exist to supply the subject.
fn is_bare_anonymous_slot(node: &PenNode) -> bool {
    let (base, container) = match node {
        PenNode::Frame(f) => (&f.base, &f.container),
        PenNode::Rectangle(r) => (&r.base, &r.container),
        _ => return false,
    };
    if base.name.as_deref().is_some_and(|n| !n.trim().is_empty()) {
        return false;
    }
    let rounded = container.corner_radius.is_some() || container.clip_content == Some(true);
    if !rounded {
        return false;
    }
    let (Some(w), Some(h)) = (
        dimension_number(&container.width),
        dimension_number(&container.height),
    ) else {
        return false;
    };
    if w < 48.0 || h < 48.0 || w / h > 1.6 || h / w > 1.6 {
        return false;
    }
    if !matches!(container.fill.as_deref(), Some([PenFill::Solid(_)])) {
        return false;
    }
    node.children().is_none_or(|c| c.is_empty())
}

fn is_unnamed_media_slot_in_context(node: &PenNode, parent_names: &[String]) -> bool {
    let PenNode::Rectangle(rect) = node else {
        return false;
    };
    if has_non_media_context(parent_names) {
        return false;
    }
    if rect
        .base
        .name
        .as_deref()
        .is_some_and(|name| !name.trim().is_empty())
    {
        return false;
    }
    if !is_small_thumb_size(&rect.container.width, &rect.container.height) {
        return false;
    }
    if !matches!(rect.container.fill.as_deref(), Some([PenFill::Solid(_)])) {
        return false;
    }
    if rect
        .children
        .as_ref()
        .is_some_and(|children| !children.is_empty())
    {
        return false;
    }
    const CONTEXT_WORDS: [&str; 6] = ["player", "art", "cover", "album", "media", "track"];
    parent_names.iter().any(|name| {
        let lowered = name.to_ascii_lowercase();
        CONTEXT_WORDS.iter().any(|word| {
            lowered
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|token| token == *word)
        })
    })
}

/// Explicit structural/control vocabulary vetoes the anonymous-slot fallback.
/// These surfaces often have the same small rounded solid geometry as cover
/// art, but filling a KPI, swatch, badge, or button with a photo is always a
/// worse failure than leaving an ambiguous box untouched. Explicit image
/// nodes, placeholder roles, and media-named slots do not depend on this
/// fallback and remain eligible.
fn has_non_media_context(parent_names: &[String]) -> bool {
    const NON_MEDIA_WORDS: [&str; 18] = [
        "kpi",
        "metric",
        "stat",
        "stats",
        "analytics",
        "chart",
        "graph",
        "swatch",
        "palette",
        "button",
        "control",
        "badge",
        "indicator",
        "progress",
        "separator",
        "divider",
        "toggle",
        "status",
    ];
    parent_names.iter().any(|name| {
        name.to_ascii_lowercase()
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| NON_MEDIA_WORDS.contains(&token))
    })
}

fn is_image_area_rectangle_by_heuristic(node: &PenNode) -> bool {
    let PenNode::Rectangle(rect) = node else {
        return false;
    };
    let Some(name) = rect.base.name.as_deref() else {
        return false;
    };
    if !has_image_area_keyword(name) {
        return false;
    }
    if !is_image_area_size(&rect.container.width, &rect.container.height) {
        return false;
    }
    if !matches!(rect.container.fill.as_deref(), Some([PenFill::Solid(_)])) {
        return false;
    }
    let Some(children) = rect.children.as_ref() else {
        return true;
    };
    matches!(children.as_slice(), [] | [PenNode::IconFont(_)])
}

fn is_image_area_size(width: &Option<SizingBehavior>, height: &Option<SizingBehavior>) -> bool {
    let (width_ok, width_concrete) = image_area_dimension_ok(width, 80.0);
    let (height_ok, height_concrete) = image_area_dimension_ok(height, 60.0);
    width_ok && height_ok && (width_concrete || height_concrete)
}

/// Small keyword-named media slots — a 44×44 mini-player "Art" square sits
/// well below the generic 80×60 floor but is unmistakably an image slot
/// (measured: the mini-player artwork routinely shipped as an empty grey
/// square, test0711-22). Keyword gating keeps random small frames out.
fn is_small_thumb_size(width: &Option<SizingBehavior>, height: &Option<SizingBehavior>) -> bool {
    let (width_ok, width_concrete) = image_area_dimension_ok(width, 32.0);
    let (height_ok, height_concrete) = image_area_dimension_ok(height, 32.0);
    width_ok && height_ok && width_concrete && height_concrete
}

fn image_area_dimension_ok(size: &Option<SizingBehavior>, min_px: f64) -> (bool, bool) {
    match size {
        Some(SizingBehavior::Number(px)) if *px >= min_px => (true, true),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)) => (true, false),
        _ => (false, false),
    }
}

fn infer_aspect_ratio(width: Option<f64>, height: Option<f64>) -> Option<ImageAspectRatio> {
    let (Some(width), Some(height)) = (width, height) else {
        return None;
    };
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let ratio = width / height;
    if ratio > 1.3 {
        Some(ImageAspectRatio::Wide)
    } else if ratio < 0.77 {
        Some(ImageAspectRatio::Tall)
    } else {
        Some(ImageAspectRatio::Square)
    }
}

fn dimension_number(size: &Option<SizingBehavior>) -> Option<f64> {
    match size {
        Some(SizingBehavior::Number(px)) => Some(*px),
        _ => None,
    }
}

fn has_image_area_keyword(name: &str) -> bool {
    let compact: String = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    if compact.ends_with("img")
        || compact.ends_with("image")
        || compact.ends_with("photo")
        || compact.ends_with("cover")
        || compact.ends_with("thumbnail")
        || compact.ends_with("artwork")
        || compact.ends_with("media")
    {
        return true;
    }
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .any(|word| {
            matches!(
                word.as_str(),
                "image"
                    | "photo"
                    | "cover"
                    | "hero"
                    | "thumbnail"
                    | "thumb"
                    | "picture"
                    | "banner"
                    | "poster"
                    | "art"
                    | "artwork"
                    | "album"
                    | "avatar"
                    // The abbreviations weak models actually write: MiniMax-M3
                    // built every destination card around a rectangle named
                    // "img" (and a "ph" placeholder inside a frame named
                    // "img"), so a page of grey boxes shipped with no images at
                    // all (measured test0711-1-m3, 2026-07-12).
                    | "img"
                    | "pic"
                    | "media"
                    | "graphic"
                    | "illustration"
                    | "placeholder"
                    | "ph"
            )
        })
}

fn extract_query_for_node(node: &PenNode, parent_names: &[String]) -> String {
    if let PenNode::Image(image) = node {
        if let Some(query) = image
            .image_search_query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            return query.to_string();
        }
    }

    if let PenNode::Frame(frame) = node {
        if let Some(query) = frame
            .image_search_query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            return query.to_string();
        }
    }

    if is_image_placeholder_frame(node) {
        if let Some(label) = placeholder_label_text(node) {
            return label;
        }
    }

    if let Some(name) = node
        .base()
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if !is_generic_placeholder_name(name) {
            return name.to_string();
        }
    }

    if let Some(parent_name) = parent_semantic_name(parent_names) {
        return parent_name;
    }

    node.base()
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("placeholder")
        .to_string()
}

fn placeholder_label_text(node: &PenNode) -> Option<String> {
    let children = node.children()?;
    for child in children {
        let PenNode::Text(text) = child else {
            continue;
        };
        if text.base.role.as_deref() != Some("image-placeholder-label") {
            continue;
        }
        let label = match &text.content {
            TextContent::Plain(content) => content.trim().to_string(),
            TextContent::Styled(segments) => segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<String>()
                .trim()
                .to_string(),
        };
        if !label.is_empty() {
            return Some(label);
        }
    }
    None
}

fn parent_semantic_name(parent_names: &[String]) -> Option<String> {
    parent_names.iter().take(3).find_map(|name| {
        let trimmed = name.trim();
        if trimmed.is_empty()
            || is_generic_placeholder_name(trimmed)
            || is_layout_context_name(trimmed)
        {
            return None;
        }
        Some(trimmed.to_string())
    })
}

fn is_generic_placeholder_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "image"
            | "photo"
            | "cover"
            | "hero"
            | "thumbnail"
            | "thumb"
            | "picture"
            | "banner"
            | "poster"
            | "image placeholder"
            | "placeholder icon"
            | "placeholder"
            // A slot named "img" / "ph" carries no subject of its own — the
            // picture it wants is named by the card AROUND it ("Santorini").
            | "img"
            | "ph"
            | "pic"
            | "media"
            | "graphic"
            | "card image"
            | "card photo"
            | "product image"
            | "item image"
    )
}

fn is_layout_context_name(name: &str) -> bool {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .any(|word| {
            matches!(
                word.as_str(),
                "card"
                    | "wrapper"
                    | "container"
                    | "section"
                    | "frame"
                    | "root"
                    | "page"
                    | "stack"
                    | "row"
                    | "column"
                    | "content"
            )
        })
}

pub(crate) fn apply_result(state: &mut EditorState, node_id: &NodeId, url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() {
        return false;
    }
    let Some(node) = walkers::find_node_mut(state.active_children_mut(), node_id) else {
        return false;
    };
    let is_unfilled_placeholder_frame = is_frame_placeholder_still_unfilled(node);
    let is_unfilled_placeholder_rectangle = is_image_area_rectangle_by_heuristic(node);
    let changed = match node {
        PenNode::Image(image) => {
            if image.src == url {
                return false;
            }
            image.src = url.into();
            true
        }
        PenNode::Frame(frame) if is_unfilled_placeholder_frame => {
            frame.container.fill = Some(vec![PenFill::Image(ImageFillBody {
                url: url.into(),
                mode: Some(ImageFillMode::Crop),
                original_size: None,
                transform: None,
                explain: None,
                opacity: None,
                exposure: None,
                contrast: None,
                saturation: None,
                temperature: None,
                tint: None,
                highlights: None,
                shadows: None,
            })]);
            frame.children = Some(Vec::new());
            true
        }
        PenNode::Rectangle(rect) if is_unfilled_placeholder_rectangle => {
            rect.container.fill = Some(vec![PenFill::Image(ImageFillBody {
                url: url.into(),
                mode: Some(ImageFillMode::Crop),
                original_size: None,
                transform: None,
                explain: None,
                opacity: None,
                exposure: None,
                contrast: None,
                saturation: None,
                temperature: None,
                tint: None,
                highlights: None,
                shadows: None,
            })]);
            rect.children = Some(Vec::new());
            true
        }
        _ => false,
    };
    if changed {
        // This writes document content through raw `active_children_mut()`
        // outside the command/history path, so bump the revision. The
        // layer-panel row cache + save-dirty tracking key on
        // `document_revision()`; the placeholder-frame/rectangle branches
        // also clear `children`, which changes the visible layer rows.
        state.mark_document_changed();
    }
    changed
}

fn fetch_first_image_url_blocking(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&OpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(fetch_first_image_url(
        query,
        aspect_ratio,
        credentials,
        used_urls,
    ))
}

async fn fetch_first_image_url(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&OpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let query = simplify_search_query(query);
    if let Some(url) = fetch_openverse(&client, &query, aspect_ratio, credentials, used_urls).await
    {
        return Some(url);
    }
    let words: Vec<&str> = query.split_whitespace().filter(|w| !w.is_empty()).collect();
    if words.len() > 2 {
        let truncated = words[..2].join(" ");
        if let Some(url) =
            fetch_openverse(&client, &truncated, aspect_ratio, credentials, used_urls).await
        {
            return Some(url);
        }
        if let Some(url) = fetch_wikimedia(&client, &truncated, used_urls).await {
            return Some(url);
        }
    }
    fetch_wikimedia(&client, &query, used_urls).await
}

pub(crate) fn simplify_search_query(prompt: &str) -> String {
    let mut normalized = String::with_capacity(prompt.len());
    for ch in prompt.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || ch == '-' {
            normalized.push(ch);
        } else {
            normalized.push(' ');
        }
    }
    let keywords: Vec<&str> = normalized
        .split_whitespace()
        .filter(|word| word.len() > 2 && !IMAGE_SEARCH_STOP_WORDS.contains(word))
        .take(6)
        .collect();
    // Drop artifact words ONLY when aesthetic words remain — "logo" alone
    // must not become an empty query.
    let non_artifact: Vec<&str> = keywords
        .iter()
        .copied()
        .filter(|word| !IMAGE_SEARCH_ARTIFACT_WORDS.contains(word))
        .collect();
    let keywords: Vec<&str> = if non_artifact.is_empty() {
        keywords
    } else {
        non_artifact
    }
    .into_iter()
    .take(4)
    .collect();
    if keywords.is_empty() {
        prompt.chars().take(30).collect()
    } else {
        keywords.join(" ")
    }
}

async fn fetch_openverse(
    client: &reqwest::Client,
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&OpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    let url = openverse_search_url(query, aspect_ratio)?;
    let mut request = client.get(url);
    if let Some(credentials) = credentials {
        if let Some(token) = fetch_openverse_token(client, credentials).await {
            request = request.bearer_auth(token);
        }
    }
    let resp = request.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let results = json.get("results")?.as_array()?;
    let (result, identity) = claim_openverse_result(results, query, used_urls)?;
    let mut candidates = Vec::new();
    push_candidate_url(
        &mut candidates,
        result.get("thumbnail").and_then(serde_json::Value::as_str),
    );
    push_candidate_url(
        &mut candidates,
        result.get("url").and_then(serde_json::Value::as_str),
    );
    let outcome = first_unused_renderable_image_src(client, candidates, used_urls).await;
    settle_provider_identity(used_urls, &identity, outcome)
}

/// Titles that mark a result as noise no matter how well it ranks — the
/// classic is a literal "File not found" artwork Openverse serves for
/// weakly-matching queries (measured: it landed in a music-app card,
/// test0711-22). Junk-titled results are skipped; if EVERY result is junk
/// the slot stays empty rather than filling with a meaningless picture.
const JUNK_TITLE_MARKERS: [&str; 8] = [
    "not found",
    "404",
    "placeholder",
    "no image",
    "missing",
    "error",
    "broken",
    "deleted",
];

fn result_title(result: &serde_json::Value) -> String {
    result
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_lowercase()
}

/// Pick the best of the returned results instead of blindly trusting rank 1:
/// drop junk-titled entries, then prefer the first whose title shares a word
/// with the query (Openverse relevance degrades fast on niche queries), then
/// the first non-junk entry.
pub(crate) fn select_openverse_result<'results>(
    results: &'results [serde_json::Value],
    query: &str,
    used_urls: &HashSet<String>,
) -> Option<&'results serde_json::Value> {
    let is_used = |result: &serde_json::Value| {
        openverse_result_identity(result).is_some_and(|identity| used_urls.contains(&identity))
            // Backward-compatible URL keys keep the pure selector useful to
            // callers/tests, but production claims canonical result identity.
            || ["url", "thumbnail"].iter().any(|key| {
                result
                    .get(*key)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|url| used_urls.contains(url))
            })
    };
    let non_junk: Vec<&serde_json::Value> = results
        .iter()
        .filter(|result| {
            let title = result_title(result);
            !is_used(result)
                && !JUNK_TITLE_MARKERS
                    .iter()
                    .any(|marker| title.contains(marker))
        })
        .collect();
    let query_words: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .map(str::to_string)
        .collect();
    non_junk
        .iter()
        .find(|result| {
            let title = result_title(result);
            query_words.iter().any(|word| title.contains(word.as_str()))
        })
        .copied()
        .or_else(|| non_junk.first().copied())
}

fn openverse_result_identity(result: &serde_json::Value) -> Option<String> {
    if let Some(id) = result.get("id") {
        if let Some(id) = id.as_str().filter(|id| !id.trim().is_empty()) {
            return Some(format!("openverse:{id}"));
        }
        if id.is_number() {
            return Some(format!("openverse:{id}"));
        }
    }
    ["url", "thumbnail"].iter().find_map(|field| {
        result
            .get(*field)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(|url| format!("openverse-url:{url}"))
    })
}

fn claim_openverse_result<'results>(
    results: &'results [serde_json::Value],
    query: &str,
    used_images: &Mutex<HashSet<String>>,
) -> Option<(&'results serde_json::Value, String)> {
    let mut used = used_images.lock().unwrap();
    let result = select_openverse_result(results, query, &used)?;
    let identity = openverse_result_identity(result)?;
    used.insert(identity.clone()).then_some((result, identity))
}

fn openverse_search_url(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
) -> Option<reqwest::Url> {
    let query = simplify_search_query(query);
    let mut url = reqwest::Url::parse_with_params(
        "https://api.openverse.org/v1/images/",
        &[("q", query.as_str()), ("page_size", "10")],
    )
    .ok()?;
    if let Some(aspect_ratio) = aspect_ratio {
        url.query_pairs_mut()
            .append_pair("aspect_ratio", aspect_ratio.as_openverse_param());
    }
    Some(url)
}

pub(crate) async fn fetch_openverse_token(
    client: &reqwest::Client,
    credentials: &OpenverseCredentials,
) -> Option<String> {
    let resp = client
        .post("https://api.openverse.org/v1/auth_tokens/token/")
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.as_str()),
        ])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("access_token")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

async fn fetch_wikimedia(
    client: &reqwest::Client,
    query: &str,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    let url = reqwest::Url::parse_with_params(
        "https://commons.wikimedia.org/w/api.php",
        &[
            ("action", "query"),
            ("generator", "search"),
            ("gsrsearch", query),
            ("gsrnamespace", "6"),
            ("gsrlimit", "1"),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|mime"),
            ("iiurlwidth", "800"),
            ("format", "json"),
            ("origin", "*"),
        ],
    )
    .ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let pages = json.get("query")?.get("pages")?.as_object()?;
    for page in pages.values() {
        let Some(identity) = wikimedia_page_identity(page) else {
            continue;
        };
        if !used_urls.lock().unwrap().insert(identity.clone()) {
            continue;
        }
        // An empty candidate list deliberately settles as Unavailable below,
        // releasing the reservation for pages with missing/empty imageinfo.
        let candidates = wikimedia_image_candidates(page);
        let outcome = first_unused_renderable_image_src(client, candidates, used_urls).await;
        if let Some(src) = settle_provider_identity(used_urls, &identity, outcome) {
            return Some(src);
        }
    }
    None
}

fn wikimedia_page_identity(page: &serde_json::Value) -> Option<String> {
    if let Some(page_id) = page.get("pageid") {
        if page_id.is_number() || page_id.is_string() {
            return Some(format!("wikimedia:{page_id}"));
        }
    }
    page.get("title")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(|title| format!("wikimedia-title:{title}"))
}

fn wikimedia_image_candidates(page: &serde_json::Value) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(info) = page
        .get("imageinfo")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| items.first())
    {
        push_candidate_url(
            &mut candidates,
            info.get("thumburl").and_then(serde_json::Value::as_str),
        );
        push_candidate_url(
            &mut candidates,
            info.get("url").and_then(serde_json::Value::as_str),
        );
    }
    candidates
}

fn push_candidate_url(candidates: &mut Vec<String>, url: Option<&str>) {
    let Some(url) = url.map(str::trim).filter(|url| !url.is_empty()) else {
        return;
    };
    if !candidates.iter().any(|candidate| candidate == url) {
        candidates.push(url.to_string());
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ImageCandidateClaim {
    /// A renderable image won the session-wide content claim.
    Claimed(String),
    /// A renderable download matched content another provider result already
    /// owns. The provider identity stays used because its artwork is known to
    /// be a duplicate even though this request cannot return it.
    Duplicate,
    /// No candidate URL produced a renderable image. The provider reservation
    /// must be released so a later request can retry a transient failure.
    Unavailable,
}

fn settle_provider_identity(
    used_urls: &Mutex<HashSet<String>>,
    identity: &str,
    outcome: ImageCandidateClaim,
) -> Option<String> {
    match outcome {
        ImageCandidateClaim::Claimed(src) => Some(src),
        ImageCandidateClaim::Duplicate => None,
        ImageCandidateClaim::Unavailable => {
            used_urls.lock().unwrap().remove(identity);
            None
        }
    }
}

async fn first_unused_renderable_image_src(
    client: &reqwest::Client,
    candidates: Vec<String>,
    used_urls: &Mutex<HashSet<String>>,
) -> ImageCandidateClaim {
    let mut found_duplicate = false;
    for candidate in candidates {
        if let Some(src) = fetch_image_data_url(client, &candidate).await {
            // Claim the embedded result under one lock. Different queries run
            // concurrently and may resolve to the same underlying image; a
            // snapshot-then-insert check lets both win. The atomic claim lets
            // the loser continue to another candidate/fallback instead.
            if claim_unused_image_src(used_urls, &src) {
                return ImageCandidateClaim::Claimed(src);
            }
            found_duplicate = true;
        }
    }
    if found_duplicate {
        ImageCandidateClaim::Duplicate
    } else {
        ImageCandidateClaim::Unavailable
    }
}

fn claim_unused_image_src(used_urls: &Mutex<HashSet<String>>, src: &str) -> bool {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    used_urls
        .lock()
        .unwrap()
        .insert(format!("content:{:016x}", hasher.finish()))
}

pub(crate) async fn fetch_image_data_url(client: &reqwest::Client, url: &str) -> Option<String> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    if resp
        .content_length()
        .is_some_and(|len| len > MAX_EMBEDDED_IMAGE_BYTES as u64)
    {
        return None;
    }
    let header_mime = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(normalize_image_mime_header);
    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_EMBEDDED_IMAGE_BYTES {
        return None;
    }
    let mime = header_mime.or_else(|| sniff_image_mime(&bytes).map(str::to_string))?;
    image_bytes_to_data_url(&mime, &bytes)
}

fn image_bytes_to_data_url(mime: &str, bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let mime = normalize_image_mime_header(mime)?;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    // Shrink an oversized fetched image before it enters the document —
    // same rationale as the file-pick path (see `image_downscale`).
    if let Some((scaled_mime, scaled)) = crate::image_downscale::maybe_downscale(bytes) {
        return Some(format!("data:{scaled_mime};base64,{}", B64.encode(&scaled)));
    }
    Some(format!("data:{mime};base64,{}", B64.encode(bytes)))
}

fn normalize_image_mime_header(value: &str) -> Option<String> {
    let mime = value.split(';').next()?.trim().to_ascii_lowercase();
    if mime == "image/jpg" {
        return Some("image/jpeg".to_string());
    }
    if mime.starts_with("image/") && mime != "image/svg+xml" {
        Some(mime)
    } else {
        None
    }
}

fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

#[cfg(test)]
#[path = "image_search_session_tests.rs"]
mod tests;
