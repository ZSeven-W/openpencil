//! Root-seed guard: decide whether a design turn's first batch needs a
//! synthetic root frame (mobile vs desktop), seed it, and keep the mobile
//! status bar canonical + de-duplicated. Split out of
//! `design_agent_tools.rs` to keep the spine under the 800-line cap.

use super::*;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootSeedTarget {
    Mobile,
    Desktop,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MobileNormalizeReport {
    pub status_bars_inserted: usize,
    pub status_bars_replaced: usize,
    pub duplicate_status_bars_removed: usize,
    pub viewport_heights_fixed: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct RootSeedProfile {
    target: RootSeedTarget,
    width: f64,
    height: f64,
    background_color: Option<String>,
    inherited: bool,
}

impl RootSeedTarget {
    fn from_mobile(mobile: bool) -> Self {
        if mobile {
            Self::Mobile
        } else {
            Self::Desktop
        }
    }

    fn dimensions(self) -> (f64, f64) {
        match self {
            Self::Mobile => (390.0, 844.0),
            Self::Desktop => (1440.0, 900.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RootSeedGuard {
    target: RootSeedTarget,
    continuation: bool,
    consumed: bool,
}

impl RootSeedGuard {
    pub fn from_prompt(prompt: &str) -> Self {
        Self {
            target: root_seed_target_for_prompt(prompt),
            continuation: root_seed_prompt_is_continuation(prompt),
            consumed: false,
        }
    }

    pub fn disabled() -> Self {
        Self {
            target: RootSeedTarget::Desktop,
            continuation: false,
            consumed: true,
        }
    }

    fn pending_profile(&self) -> Option<(RootSeedTarget, bool)> {
        (!self.consumed).then_some((self.target, self.continuation))
    }

    fn mark_consumed(&mut self) {
        self.consumed = true;
    }
}

pub fn root_seed_target_for_prompt(prompt: &str) -> RootSeedTarget {
    RootSeedTarget::from_mobile(root_seed_prompt_is_mobile(prompt))
}

/// Whether the REQUEST asks to continue an existing screen set.
///
/// This is the same signal the orchestrator's `ContinuationContext` is built
/// from (`chat_design_request::sibling_continuation_context`), so both paths
/// agree on what counts as a continuation: a prompt that promises named
/// sibling screens.
///
/// Without this gate, "the canvas happens to hold a frame of the same class"
/// was enough to inherit — and because a continuation deliberately keeps the
/// guard pending, EVERY later top-level frame of that turn was then stamped
/// with the first screen's size and background, including the values the model
/// wrote itself.
pub fn root_seed_prompt_is_continuation(prompt: &str) -> bool {
    !crate::chat_intent::listed_whole_screen_names(prompt).is_empty()
}

pub fn root_seed_prompt_is_mobile(prompt: &str) -> bool {
    if prompt.contains("手机") {
        return true;
    }
    let lower = prompt.to_ascii_lowercase();
    // "web app" / "webapp" name desktop products — a dashboard "web app"
    // must not be seeded 390x844. Strip the desktop-ish phrases before the
    // word scan so the bare "app" signal only fires for actual mobile asks.
    if lower.contains("web app") || lower.contains("webapp") || lower.contains("desktop") {
        let desktopish = lower.replace("web app", " ").replace("webapp", " ");
        return desktopish
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| matches!(word, "mobile" | "phone" | "ios" | "android"));
    }
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| matches!(word, "mobile" | "app" | "phone" | "ios" | "android"))
}

/// Bring every mobile-sized top-level screen in `state` to the product's
/// canonical shape, whatever path produced it.
pub fn normalize_mobile_screens(state: &mut EditorState) -> MobileNormalizeReport {
    let mut report = MobileNormalizeReport::default();
    let mobile_root_indices = state
        .active_children()
        .iter()
        .enumerate()
        .filter_map(|(index, node)| is_mobile_sized_root(node).then_some(index))
        .collect::<Vec<_>>();

    for index in mobile_root_indices {
        let Some(root) = state.active_children_mut().get_mut(index) else {
            continue;
        };
        let change = apply_mobile_status_bar(root, true);
        report.status_bars_inserted += change.inserted;
        report.status_bars_replaced += change.replaced;

        let has_canonical_status_bar = root
            .children()
            .into_iter()
            .flatten()
            .any(is_canonical_status_bar);
        if has_canonical_status_bar && is_fit_content_height(root) {
            root.set_height_px(812.0);
            report.viewport_heights_fixed += 1;
        }
    }

    // Run this after replacement/insertion as well as for bars that were
    // already canonical, so a root with several hand-drawn bars is fully
    // canonical in this same pass and the next pass is idempotent.
    report.duplicate_status_bars_removed = remove_nested_duplicate_status_bars(state);
    report
}

fn is_mobile_sized_root(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_))
        && node
            .width_px()
            .is_some_and(|width| width.is_finite() && (320.0..=430.0).contains(&width))
}

fn is_fit_content_height(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    matches!(
        frame.container.height,
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    )
}

pub(super) fn should_track_root_seed_candidate(
    _state: &EditorState,
    name: &str,
    indicator_epoch: Option<u64>,
    root_seed_guard: Option<&RootSeedGuard>,
) -> bool {
    if name != "batch_design" {
        return false;
    }
    root_seed_guard
        .and_then(RootSeedGuard::pending_profile)
        .is_some()
        || indicator_epoch
            .and_then(op_editor_core::agent_indicators::root_seed_hint_if_pending)
            .is_some()
}

pub(super) fn collect_active_top_level_node_ids(state: &EditorState) -> HashSet<String> {
    state
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect()
}

pub(super) fn maybe_apply_root_seed_guard(
    state: &mut EditorState,
    ids_before: &HashSet<String>,
    indicator_epoch: Option<u64>,
    root_seed_guard: Option<&mut RootSeedGuard>,
) -> Option<String> {
    let explicit_profile = root_seed_guard
        .as_ref()
        .and_then(|guard| guard.pending_profile());
    let epoch_profile = indicator_epoch
        .and_then(op_editor_core::agent_indicators::root_seed_hint_if_pending)
        .map(|hint| (RootSeedTarget::from_mobile(hint.mobile), hint.continuation));
    let (target, continuation) = explicit_profile.or(epoch_profile)?;
    let profile = resolve_root_seed_profile(state, ids_before, target, continuation);
    let allow_existing_single_root = !profile.inherited;
    let seed_hint =
        seed_root_frame_if_needed(state, ids_before, &profile, allow_existing_single_root);
    // Mobile chrome parity with the orchestrator scaffold: the loop's first
    // batch gets the SAME pre-inserted status bar, so `mobile-app.md`'s
    // "status bar is already pre-inserted" contract holds on this path too.
    // Runs even when the model authored explicit root dimensions (the seed
    // above early-returns then, but the chrome must still land).
    let chrome_hint = (profile.target == RootSeedTarget::Mobile)
        .then(|| inject_mobile_status_bar_if_missing(state, ids_before, allow_existing_single_root))
        .flatten();

    // A fresh-design guard is a one-shot default for the first root. A
    // continuation contract is turn-scoped instead: DeepSeek commonly emits
    // one sibling screen per batch, and a first batch may only modify the old
    // screen. Keep the guard pending so every later top-level root inherits
    // the same artboard and chrome contract.
    if !profile.inherited {
        if let Some(guard) = root_seed_guard {
            guard.mark_consumed();
        } else if let Some(epoch) = indicator_epoch {
            op_editor_core::agent_indicators::mark_root_seed_guard_consumed(epoch);
        }
    }

    match (seed_hint, chrome_hint) {
        (Some(seed), Some(chrome)) => Some(format!("{seed} {chrome}")),
        (Some(seed), None) => Some(seed),
        (None, Some(chrome)) => Some(chrome),
        (None, None) => None,
    }
}

/// Insert the standard iOS status-bar chrome as the mobile root's FIRST
/// child unless the model already built one. Reuses the orchestrator
/// scaffold's exact node tree (`scaffold::mobile_status_bar_node`) so both
/// generation paths ship byte-identical chrome. Returns the model-facing
/// hint when the bar was injected.
pub(super) fn inject_mobile_status_bar_if_missing(
    state: &mut EditorState,
    ids_before: &HashSet<String>,
    allow_existing_single_root: bool,
) -> Option<String> {
    let candidate_indices =
        root_seed_candidate_indices(state, ids_before, allow_existing_single_root);
    let mut inserted = false;
    let mut replaced = false;
    for index in candidate_indices {
        let Some(root) = state.active_children_mut().get_mut(index) else {
            continue;
        };
        let change = apply_mobile_status_bar(root, false);
        replaced |= change.replaced > 0;
        inserted |= change.inserted > 0;
    }
    if replaced {
        Some(
            "The status bar you built was replaced with the standard iOS status bar \
             (62px, role=status-bar) - do NOT rebuild or restyle it."
                .to_string(),
        )
    } else if inserted {
        Some(
            "A standard iOS status bar (62px, role=status-bar) was pre-inserted as the root's \
             first child - do NOT create another status bar; start your content below it."
                .to_string(),
        )
    } else {
        None
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct MobileStatusBarChange {
    inserted: usize,
    replaced: usize,
}

/// Apply the shared canonical status-bar construction to one root. The
/// `initialize_missing_children` flag is only for the public full-state
/// normalizer; the historical root-seed hook keeps its prior no-op behavior
/// for a frame whose optional children field is absent.
fn apply_mobile_status_bar(
    root: &mut PenNode,
    initialize_missing_children: bool,
) -> MobileStatusBarChange {
    // OS chrome has exactly one canonical form. A model-built status bar
    // (name matches, structure doesn't — no role, ad-hoc children) is
    // REPLACED in place rather than kept: every hand-rolled variant we
    // measured deviated visibly from the iOS reference (GLM-5.2 2026-07-11).
    let noncanonical_index = root
        .children()
        .into_iter()
        .flatten()
        .position(|child| is_status_bar_node(child) && !is_canonical_status_bar(child));
    if let Some(index) = noncanonical_index {
        let root_id = root.id_str().to_string();
        let fill_hex = op_editor_core::first_solid_fill_hex(root)
            .unwrap_or("#ffffff")
            .to_string();
        let width = root.width_px().unwrap_or(390.0);
        let Ok(bar) = op_orchestrator::scaffold::mobile_status_bar_node(&root_id, &fill_hex, width)
        else {
            return MobileStatusBarChange::default();
        };
        if let Some(children) = root.children_mut() {
            children[index] = bar;
            return MobileStatusBarChange {
                replaced: 1,
                ..Default::default()
            };
        }
        return MobileStatusBarChange::default();
    }
    if root
        .children()
        .into_iter()
        .flatten()
        .any(is_status_bar_node)
    {
        return MobileStatusBarChange::default();
    }

    let root_id = root.id_str().to_string();
    let fill_hex = op_editor_core::first_solid_fill_hex(root)
        .unwrap_or("#ffffff")
        .to_string();
    let width = root.width_px().unwrap_or(390.0);
    let Ok(bar) = op_orchestrator::scaffold::mobile_status_bar_node(&root_id, &fill_hex, width)
    else {
        return MobileStatusBarChange::default();
    };
    if let Some(children) = root.children_mut() {
        children.insert(0, bar);
        return MobileStatusBarChange {
            inserted: 1,
            ..Default::default()
        };
    }
    if initialize_missing_children {
        if let PenNode::Frame(frame) = root {
            frame.children = Some(vec![bar]);
            return MobileStatusBarChange {
                inserted: 1,
                ..Default::default()
            };
        }
    }
    MobileStatusBarChange::default()
}

/// The injected/scaffold chrome shape: role tag + the Time/Levels pair.
/// Anything else that merely NAMES itself a status bar is a hand-rolled
/// variant slated for replacement.
pub(super) fn is_canonical_status_bar(node: &PenNode) -> bool {
    node.base().role.as_deref() == Some("status-bar")
        && node.children().is_some_and(|children| {
            children
                .iter()
                .any(|c| c.base().name.as_deref() == Some("Levels"))
        })
}

pub(super) fn is_status_bar_node(node: &PenNode) -> bool {
    if node.base().role.as_deref() == Some("status-bar") {
        return true;
    }
    node.base()
        .name
        .as_deref()
        .is_some_and(|name| name.to_ascii_lowercase().contains("status bar"))
}

/// Contract sweep, every batch: once a root carries the canonical status
/// bar as a direct child, any OTHER status-bar-looking node in that root is
/// removed on the spot. The first-batch injection hook can't see later
/// batches — measured (test0711-22 23:07 run): the model hand-built a
/// second "21:30" bar inside the Header several batches in, which then sat
/// on screen until finalize. OS chrome is a contract, so the duplicate is
/// removed deterministically rather than echoed.
pub(super) fn remove_nested_duplicate_status_bars(state: &mut EditorState) -> usize {
    let mut removed = 0;
    for root in state.active_children_mut() {
        let has_canonical = root
            .children()
            .into_iter()
            .flatten()
            .any(is_canonical_status_bar);
        if !has_canonical {
            continue;
        }
        fn prune(node: &mut PenNode, keep_canonical: bool, removed: &mut usize) {
            let Some(children) = node.children_mut() else {
                return;
            };
            children.retain(|child| {
                let duplicate = is_status_bar_node(child)
                    && !(keep_canonical && is_canonical_status_bar(child));
                if duplicate {
                    *removed += 1;
                }
                !duplicate
            });
            for child in children {
                prune(child, false, removed);
            }
        }
        prune(root, true, &mut removed);
    }
    removed
}

fn seed_root_frame_if_needed(
    state: &mut EditorState,
    ids_before: &HashSet<String>,
    profile: &RootSeedProfile,
    allow_existing_single_root: bool,
) -> Option<String> {
    let candidate_indices =
        root_seed_candidate_indices(state, ids_before, allow_existing_single_root);
    let mut changed = 0usize;
    for index in candidate_indices {
        let Some(root) = state.active_children_mut().get_mut(index) else {
            continue;
        };
        let width_before = root.width_px();
        let height_before = root.height_px();
        if width_before.is_none() || (profile.inherited && width_before != Some(profile.width)) {
            root.set_width_px(profile.width);
            changed += 1;
        }
        if height_before.is_none() || (profile.inherited && height_before != Some(profile.height)) {
            root.set_height_px(profile.height);
            changed += 1;
        }
        // The artboard is a contract — sibling screens of one product that do
        // not share a frame size are a continuation bug, so a wrong numeric
        // width/height above is corrected. The background is not: it is
        // authored design intent, so an inherited colour only SEEDS a root the
        // model left unfilled and never overwrites one it chose.
        if let Some(color) = profile.background_color.as_deref() {
            if op_editor_core::first_solid_fill_hex(root).is_none()
                && op_editor_core::fills::set_primary_fill_hex(root, color)
            {
                changed += 1;
            }
        }
        default_root_layout_to_vertical(root);
    }
    if changed == 0 {
        return None;
    }

    Some(format!(
        "root seeded to {}x{} - grow height if content exceeds.",
        format_seed_dimension(profile.width),
        format_seed_dimension(profile.height)
    ))
}

fn resolve_root_seed_profile(
    state: &EditorState,
    ids_before: &HashSet<String>,
    target: RootSeedTarget,
    continuation: bool,
) -> RootSeedProfile {
    // Inheriting is a CONTINUATION contract, not a canvas fact. Without the
    // gate, any turn on a non-empty canvas kept the guard pending forever and
    // rewrote every root it produced to the first screen's dimensions.
    let matching_existing_screen = continuation.then(|| {
        state.active_children().iter().rev().find(|node| {
            if !ids_before.contains(node.id_str())
                || !matches!(node, PenNode::Frame(_))
                || node.children().is_none_or(|children| children.is_empty())
            {
                return false;
            }
            let Some(width) = node.width_px() else {
                return false;
            };
            let Some(height) = node.height_px() else {
                return false;
            };
            let mobile_width = (320.0..=480.0).contains(&width);
            width.is_finite()
                && height.is_finite()
                && width > 0.0
                && height > 0.0
                && mobile_width == (target == RootSeedTarget::Mobile)
        })
    });
    if let Some(screen) = matching_existing_screen.flatten() {
        return RootSeedProfile {
            target,
            width: screen.width_px().expect("matched numeric width"),
            height: screen.height_px().expect("matched numeric height"),
            background_color: op_editor_core::first_solid_fill_hex(screen).map(str::to_string),
            inherited: true,
        };
    }
    let (width, height) = target.dimensions();
    RootSeedProfile {
        target,
        width,
        height,
        background_color: None,
        inherited: false,
    }
}

fn root_seed_candidate_indices(
    state: &EditorState,
    ids_before: &HashSet<String>,
    allow_existing_single_root: bool,
) -> Vec<usize> {
    let roots = state.active_children();
    let new_root_indices = roots
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            (!ids_before.contains(node.id_str()) && matches!(node, PenNode::Frame(_)))
                .then_some(index)
        })
        .collect::<Vec<_>>();
    if !new_root_indices.is_empty() {
        return new_root_indices;
    }
    if allow_existing_single_root && roots.len() == 1 && matches!(roots[0], PenNode::Frame(_)) {
        vec![0]
    } else {
        Vec::new()
    }
}

pub(super) fn default_root_layout_to_vertical(node: &mut PenNode) {
    if let PenNode::Frame(frame) = node {
        if frame.container.layout.is_none() {
            frame.container.layout = Some(LayoutMode::Vertical);
        }
    }
}

pub(super) fn format_seed_dimension(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}
