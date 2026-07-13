//! Background image-search enrichment for generated image nodes.

use std::collections::{HashMap, HashSet};
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
    /// Resolved numeric dimensions (for the gen provider's aspect mapping).
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    rx: Receiver<Option<String>>,
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
    /// Result URLs already used this session — similar queries ("playlist
    /// cover daily mix" / "... chill vibes" / "... discover weekly") share
    /// their Openverse top hit, which filled three different cards with the
    /// SAME photo (measured: test0711-22). Selection skips these best-effort.
    used_urls: Arc<Mutex<HashSet<String>>>,
    /// Query → the photo that query already resolved to, this session.
    ///
    /// The dedup above must NOT fire when the SAME subject comes back: the
    /// model rebuilds a section mid-run (fresh node ids), the same query
    /// ("Bali Indonesia") searches again, and its own good photo is now in
    /// `used_urls` — so the second search skipped it and took a junk result
    /// instead (measured 2026-07-12: a real Bali temple photo turned into a
    /// plain blue sky halfway through a run). One subject, one photo: a repeat
    /// query resolves from this memo with no network call at all.
    resolved: Arc<Mutex<HashMap<String, String>>>,
}

/// Memo key — the subject, not its spelling.
fn query_key(query: &str) -> String {
    query.trim().to_ascii_lowercase()
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
        self.used_urls.lock().unwrap().clear();
        self.resolved.lock().unwrap().clear();
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
        // Strategy: a configured image-GEN model wins (generate from the bound
        // `image_prompt`); otherwise fall back to stock SEARCH (Openverse). The
        // prompt/query stay on the node either way, so the UI can re-gen/re-search
        // and a later config change re-resolves on the next enqueue.
        let gen_profile = crate::image_panel_host::active_image_gen_profile(state).cloned();
        let credentials = OpenverseCredentials::from_state(state);
        for target in targets {
            let id = target.node_id.as_str().to_string();
            self.in_flight.insert(id);
            let job = match &gen_profile {
                Some(profile) => spawn_gen_job(target, profile.clone()),
                None => spawn_job(
                    target,
                    credentials.clone(),
                    Arc::clone(&self.used_urls),
                    Arc::clone(&self.resolved),
                ),
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
    resolved: Arc<Mutex<HashMap<String, String>>>,
) -> ImageSearchJob {
    let (tx, rx) = mpsc::channel();
    let node_id = target.node_id.clone();
    let aspect_ratio = target.aspect_ratio;
    let key = query_key(&target.query);
    // One subject, one photo: a query this session already answered resolves
    // from the memo — no network, and (crucially) no dedup-forced downgrade
    // when a rebuilt section asks for the same picture again.
    if let Some(url) = resolved.lock().unwrap().get(&key).cloned() {
        let _ = tx.send(Some(url));
        return ImageSearchJob { node_id, rx };
    }
    std::thread::spawn(move || {
        let url = fetch_first_image_url_blocking(
            &target.query,
            aspect_ratio,
            credentials.as_ref(),
            &used_urls,
        );
        if let Some(found) = url.as_ref() {
            resolved.lock().unwrap().insert(key, found.clone());
        }
        let _ = tx.send(url);
    });
    ImageSearchJob { node_id, rx }
}

/// Enrich via the configured image-GEN model instead of stock search. Prefers the
/// node's bound `image_prompt`; falls back to the search keyword so a node that
/// only carries `image_search_query` still generates. Same `ImageSearchJob`
/// channel as search, so `poll_into` applies the resulting src identically.
fn spawn_gen_job(target: ImageSearchTarget, profile: ImageGenProfile) -> ImageSearchJob {
    let (tx, rx) = mpsc::channel();
    let node_id = target.node_id.clone();
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
    ImageSearchJob { node_id, rx }
}

pub(crate) fn collect_targets(
    state: &EditorState,
    known_node_ids: &HashSet<String>,
) -> Vec<ImageSearchTarget> {
    let mut targets = Vec::new();
    collect_from_children(state.active_children(), known_node_ids, &mut targets, &[]);
    targets
}

fn collect_from_children(
    children: &[PenNode],
    known_node_ids: &HashSet<String>,
    targets: &mut Vec<ImageSearchTarget>,
    parent_names: &[String],
) {
    // Sibling text of a bare anonymous slot IS its subject ("Blinding
    // Lights" next to a nameless 120px square = that track's cover).
    let sibling_text: Vec<String> = children
        .iter()
        .filter_map(|c| match c {
            PenNode::Text(t) => match &t.content {
                TextContent::Plain(text) if !text.trim().is_empty() => {
                    Some(text.trim().to_string())
                }
                _ => None,
            },
            _ => None,
        })
        .take(2)
        .collect();
    for node in children {
        if let Some(target) =
            image_search_target_for(node, known_node_ids, parent_names, &sibling_text)
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
            collect_from_children(grand, known_node_ids, targets, &child_parent_names);
        }
    }
}

fn image_search_target_for(
    node: &PenNode,
    known_node_ids: &HashSet<String>,
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
    let bare_slot_with_context = !sibling_text.is_empty() && is_bare_anonymous_slot(node);
    let needs_image = match node {
        PenNode::Image(image) => is_placeholder_src(&image.src),
        PenNode::Frame(_) => is_frame_placeholder_still_unfilled(node) || bare_slot_with_context,
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
        query = if explicit.is_empty() {
            sibling_text.join(" ")
        } else {
            explicit
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
    Some(ImageSearchTarget {
        node_id: NodeId::new(id),
        query,
        aspect_ratio: infer_aspect_ratio(node),
        prompt,
        width: node.width_px(),
        height: node.height_px(),
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

fn infer_aspect_ratio(node: &PenNode) -> Option<ImageAspectRatio> {
    let (width, height) = match node {
        PenNode::Image(image) => (&image.width, &image.height),
        PenNode::Frame(frame) => (&frame.container.width, &frame.container.height),
        PenNode::Rectangle(rect) => (&rect.container.width, &rect.container.height),
        _ => return None,
    };
    let (Some(width), Some(height)) = (dimension_number(width), dimension_number(height)) else {
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
    let picked = runtime.block_on(fetch_first_image_url(
        query,
        aspect_ratio,
        credentials,
        used_urls,
    ));
    if let Some(url) = picked.as_ref() {
        used_urls.lock().unwrap().insert(url.clone());
    }
    picked
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
        if let Some(url) = fetch_wikimedia(&client, &truncated).await {
            return Some(url);
        }
    }
    fetch_wikimedia(&client, &query).await
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
    let used = used_urls.lock().unwrap().clone();
    let result = select_openverse_result(results, query, &used)?;
    let mut candidates = Vec::new();
    push_candidate_url(
        &mut candidates,
        result.get("thumbnail").and_then(serde_json::Value::as_str),
    );
    push_candidate_url(
        &mut candidates,
        result.get("url").and_then(serde_json::Value::as_str),
    );
    first_renderable_image_src(client, candidates).await
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
        ["url", "thumbnail"].iter().any(|key| {
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

async fn fetch_wikimedia(client: &reqwest::Client, query: &str) -> Option<String> {
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
        if let Some(info) = page
            .get("imageinfo")
            .and_then(serde_json::Value::as_array)
            .and_then(|items| items.first())
        {
            let mut candidates = Vec::new();
            push_candidate_url(
                &mut candidates,
                info.get("thumburl").and_then(serde_json::Value::as_str),
            );
            push_candidate_url(
                &mut candidates,
                info.get("url").and_then(serde_json::Value::as_str),
            );
            if let Some(src) = first_renderable_image_src(client, candidates).await {
                return Some(src);
            }
        }
    }
    None
}

fn push_candidate_url(candidates: &mut Vec<String>, url: Option<&str>) {
    let Some(url) = url.map(str::trim).filter(|url| !url.is_empty()) else {
        return;
    };
    if !candidates.iter().any(|candidate| candidate == url) {
        candidates.push(url.to_string());
    }
}

async fn first_renderable_image_src(
    client: &reqwest::Client,
    candidates: Vec<String>,
) -> Option<String> {
    for candidate in candidates {
        if let Some(src) = fetch_image_data_url(client, &candidate).await {
            return Some(src);
        }
    }
    None
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
