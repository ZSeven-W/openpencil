//! Side-by-side design directions ("variants") — the pure half.
//!
//! A variants request runs the ordinary orchestrator pipeline N times for
//! one brief, concurrently, each run pinned to a DIFFERENT style guide so
//! the directions are genuinely different rather than N samples of the
//! same palette. This module owns the decisions that need no model:
//!
//! - which style guides the directions use ([`choose_variant_style_guides`]):
//!   deterministic, platform-compatible, spread across the catalogue, with
//!   the brief's own style (a pin, or a guide the brief names) kept for
//!   direction A;
//! - the per-direction request ([`variant_request`]);
//! - how a direction's progress is told apart in the shared transcript
//!   ([`scope_variant_progress`]);
//! - how a finished direction is made self-contained and placed on the
//!   shared page ([`variant_roots_for_merge`], [`plan_variant_relayout`]).
//!
//! The concurrent driver lives in `variants_run.rs`.

use jian_ops_schema::node::PenNode;
use op_ai_skills::style_guide::{
    find_style_guide, style_guide_registry, ParsedStyleGuide, Platform,
};
use op_editor_core::{EditorState, PenNodeExt};

use crate::agent_identity::AgentIdentity;
use crate::design_type::{detect_design_type, DesignType};
use crate::style_guide_context::{
    infer_tags_from_prompt, rank_style_guides_for_prompt, style_guide_prompt_score,
};
use crate::types::{DesignRequest, Progress};

/// Horizontal gap between two directions on the page, in document px.
pub const VARIANT_GAP: f64 = 240.0;

/// Penalty per tag a candidate shares with an already-chosen direction.
const SHARED_TAG_PENALTY: i32 = 6;
/// Bonus for a light/dark mode no chosen direction uses yet.
const NEW_MODE_BONUS: i32 = 12;

/// One planned direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantPlan {
    /// Slot in the run (`0` = A).
    pub index: usize,
    /// Display name (`方案 A`); hosts localize it, the default is English.
    pub name: String,
    /// The style guide id the direction is pinned to.
    pub style_guide: String,
    /// Human label for the guide (`Editorial Orange Light`).
    pub style_label: String,
}

impl VariantPlan {
    /// `方案 A · Editorial Orange Light`.
    pub fn label(&self) -> String {
        format!("{} · {}", self.name, self.style_label)
    }

    /// Prefix stamped on each of the direction's board names.
    pub fn name_prefix(&self) -> String {
        format!("{} · ", self.label())
    }
}

/// Parallel sub-agents each direction keeps however small the shared
/// worker budget is.
pub const MIN_DIRECTION_WORKERS: u32 = 2;

/// Style-guide tags that describe product UI rather than a marketing page.
const PRODUCT_UI_TAGS: [&str; 6] = [
    "dashboard",
    "data-focused",
    "sidebar",
    "terminal",
    "developer",
    "code-inspired",
];

/// The style-guide shelf a brief draws from — the same platform routing
/// the planning prompt's catalogue uses, plus the deck shelf.
pub fn variant_platform(prompt: &str) -> Platform {
    match detect_design_type(prompt).type_ {
        DesignType::MobileScreen => Platform::Mobile,
        DesignType::Card => Platform::Card,
        DesignType::Slides => Platform::Slides,
        _ => Platform::Webapp,
    }
}

/// `editorial-orange-light` → `Editorial Orange Light`.
pub fn humanize_style_guide_name(name: &str) -> String {
    let name = name.strip_prefix("user:").unwrap_or(name);
    name.split(['-', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn mode_of(guide: &ParsedStyleGuide) -> Option<&'static str> {
    let has = |tag: &str| guide.tags.iter().any(|t| t == tag);
    if has("dark-mode") || guide.name.ends_with("-dark") {
        Some("dark")
    } else if has("light-mode") || guide.name.ends_with("-light") {
        Some("light")
    } else {
        None
    }
}

/// The corpus guide a brief names outright (`…用 zen-paper-light 风格…`).
fn guide_named_in(prompt: &str) -> Option<&'static ParsedStyleGuide> {
    let lower = prompt.to_lowercase();
    style_guide_registry()
        .iter()
        .filter(|guide| lower.contains(guide.name.as_str()))
        // The longest match wins so `saas-modern-light` beats `saas-modern`.
        .max_by_key(|guide| guide.name.len())
}

/// Choose `count` distinct style guides for a variants run.
///
/// Deterministic for a given `(prompt, pinned, count)`:
///
/// 1. Direction A keeps the brief's own style: the user's pinned guide
///    when it resolves, else a corpus guide the brief names, else the
///    catalogue's best match for the brief.
/// 2. Every other direction comes from the brief's platform shelf (a
///    phone brief never gets a web-dashboard guide), ranked by how well it
///    fits the brief, minus a penalty for each tag it shares with a
///    direction already chosen, plus a bonus for a light/dark mode none of
///    them uses. Ties keep ranking order.
/// 3. A shelf too small for `count` tops up from the web shelf, the only
///    one that is always large enough.
///
/// Returns fewer than `count` plans only when the whole catalogue is
/// smaller than `count`.
pub fn choose_variant_style_guides(
    prompt: &str,
    pinned: Option<&str>,
    count: usize,
) -> Vec<VariantPlan> {
    let count = count.max(1);
    let platform = variant_platform(prompt);
    let tags = infer_tags_from_prompt(prompt);
    let ranked = rank_style_guides_for_prompt(&tags, platform);

    let mut chosen: Vec<(String, String, Vec<String>, Option<&'static str>)> = Vec::new();

    // Direction A: the brief's own style.
    let pinned_guide = pinned
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .and_then(find_style_guide);
    if let Some(guide) = pinned_guide {
        chosen.push((
            guide.id().to_string(),
            guide.name.clone(),
            guide.tags.clone(),
            mode_of(&guide),
        ));
    } else if let Some(guide) = guide_named_in(prompt) {
        chosen.push((
            guide.name.clone(),
            guide.name.clone(),
            guide.tags.clone(),
            mode_of(guide),
        ));
    }

    // A landing page never borrows a product-UI guide (dashboard, terminal,
    // developer) unless the brief itself is about that kind of product: the
    // light/dark diversity bonus once handed a coffee-bean shop the data
    // dashboard look.
    let landing = detect_design_type(prompt).type_ == DesignType::LandingPage;
    let suits_brief = |guide: &&'static ParsedStyleGuide| {
        let mut product_tags = guide
            .tags
            .iter()
            .filter(|tag| PRODUCT_UI_TAGS.contains(&tag.as_str()))
            .peekable();
        // Not a landing page, not a product-UI guide, or the brief is about
        // that very kind of product (it shares one of the guide's tags).
        !landing
            || product_tags.peek().is_none()
            || product_tags.any(|tag| tags.iter().any(|t| t == tag))
    };
    let shelf: Vec<&'static ParsedStyleGuide> = ranked
        .iter()
        .copied()
        .filter(|guide| guide.platform == platform)
        .filter(suits_brief)
        .collect();
    let fallback: Vec<&'static ParsedStyleGuide> = ranked
        .iter()
        .copied()
        .filter(|guide| guide.platform != platform && guide.platform == Platform::Webapp)
        .filter(suits_brief)
        .collect();

    while chosen.len() < count {
        let unused =
            |guide: &&'static ParsedStyleGuide| !chosen.iter().any(|(id, ..)| id == &guide.name);
        // The brief's own shelf is used up before any other is touched:
        // a diversity penalty must never trade a fitting guide for a
        // guide written for a different kind of deliverable.
        let candidates: Vec<&'static ParsedStyleGuide> = if shelf.iter().any(unused) {
            shelf.iter().copied().filter(unused).collect()
        } else {
            fallback.iter().copied().filter(unused).collect()
        };
        let mut best: Option<(i32, &'static ParsedStyleGuide)> = None;
        for guide in &candidates {
            let mut score = style_guide_prompt_score(guide, &tags, platform);
            for (_, _, chosen_tags, _) in &chosen {
                let shared = guide
                    .tags
                    .iter()
                    .filter(|t| chosen_tags.contains(t))
                    .count();
                score -= SHARED_TAG_PENALTY * shared as i32;
            }
            if let Some(mode) = mode_of(guide) {
                if !chosen.is_empty() && chosen.iter().all(|(.., m)| *m != Some(mode)) {
                    score += NEW_MODE_BONUS;
                }
            }
            // Strict `>` keeps the first (best-ranked) guide on a tie.
            if best.is_none_or(|(top, _)| score > top) {
                best = Some((score, guide));
            }
        }
        let Some((_, guide)) = best else {
            break;
        };
        chosen.push((
            guide.name.clone(),
            guide.name.clone(),
            guide.tags.clone(),
            mode_of(guide),
        ));
    }

    chosen
        .into_iter()
        .enumerate()
        .map(|(index, (id, name, ..))| VariantPlan {
            index,
            name: format!("Direction {}", op_editor_core::variant_letter(index)),
            style_guide: id,
            style_label: humanize_style_guide_name(&name),
        })
        .collect()
}

/// The request one direction runs with.
///
/// Every direction is a whole new design: no append context, and the
/// direction's guide is pinned. Direction A keeps the brief's design.md
/// (a user's written-down design system outranks any catalogue pick);
/// the others drop it, or they would all come out in the same system.
/// The worker budget is shared across the directions so N directions do
/// not multiply the provider load by N — but never below
/// [`MIN_DIRECTION_WORKERS`] each: with the default single worker a
/// GLM-5.3-Flash run of three directions took 21–43 minutes, one section
/// at a time per direction. Rate limits are handled by the providers'
/// 429 backoff, not by starving the run.
pub fn variant_request(base: &DesignRequest, plan: &VariantPlan, count: usize) -> DesignRequest {
    let mut request = base.clone();
    request.append_context = None;
    request.continuation_context = None;
    request.pinned_style_guide = Some(plan.style_guide.clone());
    if plan.index > 0 {
        request.design_md = None;
    }
    let count = count.max(1) as u32;
    request.concurrency = base
        .concurrency
        .max(1)
        .div_ceil(count)
        .max(MIN_DIRECTION_WORKERS);
    request
}

/// Prefix an activity id with the direction letter so two directions'
/// sections (both called `hero`) stay two transcript rows.
fn scoped_id(index: usize, id: &str) -> String {
    format!("{}-{id}", op_editor_core::variant_letter(index))
}

fn scope_ids(index: usize, event: Progress) -> Progress {
    let s = |id: String| scoped_id(index, &id);
    match event {
        Progress::Planned { subtasks } => Progress::Planned {
            subtasks: subtasks
                .into_iter()
                .map(|(id, label)| (s(id), label))
                .collect(),
        },
        Progress::SubtaskStarted { id, label } => Progress::SubtaskStarted { id: s(id), label },
        Progress::SubtaskDone { id, node_count } => Progress::SubtaskDone {
            id: s(id),
            node_count,
        },
        Progress::SubtaskFailed { id, error } => Progress::SubtaskFailed { id: s(id), error },
        Progress::SubtaskIncomplete {
            id,
            expected,
            delivered,
        } => Progress::SubtaskIncomplete {
            id: s(id),
            expected,
            delivered,
        },
        Progress::SubtaskLanguageMismatch {
            id,
            checked,
            mismatched,
        } => Progress::SubtaskLanguageMismatch {
            id: s(id),
            checked,
            mismatched,
        },
        Progress::SubtaskSkills {
            id,
            included,
            dropped,
            budget_used,
            budget_max,
        } => Progress::SubtaskSkills {
            id: s(id),
            included,
            dropped,
            budget_used,
            budget_max,
        },
        Progress::SubtaskRetry {
            id,
            attempt,
            reason,
        } => Progress::SubtaskRetry {
            id: s(id),
            attempt,
            reason,
        },
        Progress::GeometryEcho { id, issue_count } => Progress::GeometryEcho {
            id: s(id),
            issue_count,
        },
        Progress::SubtaskNodes { id, nodes_so_far } => Progress::SubtaskNodes {
            id: s(id),
            nodes_so_far,
        },
        other => other,
    }
}

/// Wrap one direction's progress event so the shared transcript gives
/// every direction its own bubble (`group_idx` = direction slot) and its
/// own activity rows. A direction's own screen-group envelopes are
/// unwrapped first: the direction, not the screen group, is the unit the
/// user watches here.
pub fn scope_variant_progress(
    index: usize,
    label: &str,
    identity: &AgentIdentity,
    event: Progress,
) -> Progress {
    let mut event = event;
    while let Progress::WorkerScoped(worker) = event {
        event = *worker.event;
    }
    // Direction-level reports are already addressed; never re-wrap them.
    if matches!(
        event,
        Progress::VariantReady(_) | Progress::VariantFailed { .. }
    ) {
        return event;
    }
    Progress::worker_scoped(index, label, identity.clone(), scope_ids(index, event))
}

/// A finished direction's top-level boards, made self-contained for a
/// page they will share with other directions: component instances are
/// expanded and every `$variable` is resolved against the direction's OWN
/// palette. Without this the last direction to write the shared variables
/// table would repaint every other direction in its colours.
pub fn variant_roots_for_merge(state: &EditorState) -> Vec<PenNode> {
    let roots = state.active_children();
    let mut roots = if op_editor_core::ref_resolve::roots_have_refs(roots) {
        op_editor_core::ref_resolve::resolve_refs_for_canvas_roots(roots, &state.doc)
    } else {
        roots.to_vec()
    };
    op_editor_core::variables_resolve::resolve_roots_for_canvas(
        &mut roots,
        &state.doc,
        &state.ui.variables.active_theme,
    );
    roots
}

/// Stamp `prefix` on each root's name so the canvas frame labels say
/// which direction a board belongs to.
pub fn prefix_root_names(roots: &mut [PenNode], prefix: &str) {
    for root in roots {
        let base = root.base_mut();
        let name = base.name.take().unwrap_or_default();
        base.name = Some(if name.is_empty() {
            prefix.trim_end_matches([' ', '·']).to_string()
        } else {
            format!("{prefix}{name}")
        });
    }
}

/// Document-space bounds `(min_x, min_y, max_x, max_y)` of a root list;
/// `None` for an empty list. A root without an explicit size counts as a
/// point at its origin.
pub fn roots_bounds(roots: &[PenNode]) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for root in roots {
        let x = root.base().x.unwrap_or(0.0);
        let y = root.base().y.unwrap_or(0.0);
        let w = root.width_px().unwrap_or(0.0);
        let h = root.height_px().unwrap_or(0.0);
        bounds = Some(match bounds {
            None => (x, y, x + w, y + h),
            Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x + w), d.max(y + h)),
        });
    }
    bounds
}

/// Translate every root by `(dx, dy)`.
pub fn translate_roots(roots: &mut [PenNode], dx: f64, dy: f64) {
    for root in roots {
        let base = root.base_mut();
        base.x = Some(base.x.unwrap_or(0.0) + dx);
        base.y = Some(base.y.unwrap_or(0.0) + dy);
    }
}

/// Where each root of each direction should sit so the directions read
/// left to right in slot order, `gap` apart, top-aligned at `origin_y`,
/// starting at `origin_x`. Each direction keeps its own internal layout.
///
/// `groups` is `(slot, roots)` in any order. Returns `(root id, x, y)` for
/// every root whose position changes.
pub fn plan_variant_relayout(
    groups: &[(usize, Vec<&PenNode>)],
    origin_x: f64,
    origin_y: f64,
    gap: f64,
) -> Vec<(String, f64, f64)> {
    let mut ordered: Vec<&(usize, Vec<&PenNode>)> = groups.iter().collect();
    ordered.sort_by_key(|(slot, _)| *slot);
    let mut cursor = origin_x;
    let mut moves = Vec::new();
    for (_, roots) in ordered {
        let owned: Vec<PenNode> = roots.iter().map(|root| (*root).clone()).collect();
        let Some((min_x, min_y, max_x, _)) = roots_bounds(&owned) else {
            continue;
        };
        let dx = cursor - min_x;
        let dy = origin_y - min_y;
        for root in roots {
            let x = root.base().x.unwrap_or(0.0);
            let y = root.base().y.unwrap_or(0.0);
            if dx.abs() > f64::EPSILON || dy.abs() > f64::EPSILON {
                moves.push((root.id_str().to_string(), x + dx, y + dy));
            }
        }
        cursor += (max_x - min_x) + gap;
    }
    moves
}

#[cfg(test)]
#[path = "variants_tests.rs"]
mod tests;
