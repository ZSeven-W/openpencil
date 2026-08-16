//! Re-point text whose colour is invisible against its own background.
//!
//! `op_design_lint::detectors::typography` has detected this since 2026-05,
//! but nothing in the generation path ever called it: the orchestrator uses
//! exactly one lint detector (`detect_missing_progress_rings`), and the rest
//! only run through the MCP `lint_document` tool a user invokes by hand. So a
//! deck cover shipped with its title at **1.10:1** — `#FFFFFF` on `#F1F5F9`,
//! effectively blank (measured 2026-08-01, deepseek-v4-pro).
//!
//! ## Why this repairs rather than echoes
//!
//! The detector deliberately suggests nothing, because "which brand colour
//! belongs here" is an intent question. Picking a *readable* colour is not the
//! same question: when the resolved ratio is near 1:1 the text is not styled,
//! it is missing, and every candidate below comes from the document's own
//! palette. So this stays inside the "contract, auto-fixable" half of the
//! self-check split — it never invents a colour, it re-points the fill at a
//! token the document already defines.
//!
//! ## Why the judgement is contrast, never the variable name
//!
//! It is tempting to say "a text fill must not use `$color-surface`". White on
//! a dark board is correct and common — the shipped deck template's closing
//! slide does exactly that. The defect is the measured ratio, so that is what
//! is measured; the variable name is never consulted.

use crate::types::DocSink;

use std::collections::HashMap;

use jian_ops_schema::node::{FontWeight, PenNode, TextNode};
use jian_ops_schema::style::PenFill;
use jian_scene::layout_scene::SceneNode;
use op_design_lint::node_util::{is_node_visible, node_fills, node_id, opacity, resolve_color_ref};
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};

/// Palette tokens allowed as a replacement, most-preferred first.
///
/// All are emitted by `design_system` / `palette_harmonize`, so they exist in
/// any generated document. `color-surface` is included on purpose: on a dark
/// background it is the readable choice, and excluding it would leave dark
/// boards unrepairable.
/// These are the names `design_system` actually emits. An earlier version of
/// this list was written from memory (`color-text`, `color-text-strong`) and
/// matched NOTHING in a real document, so every repair silently no-opped —
/// the unit tests passed because their fixture used the invented names too.
const CANDIDATE_TOKENS: &[&str] = &[
    "color-text-primary",
    "color-text-body",
    "color-text-muted",
    "color-text-subtle",
    "color-surface",
    "color-bg-deep",
];

/// Contrast a repair must reach before it is worth making. Matches the
/// detector's own normal-text threshold rather than WCAG AA — raising the bar
/// is a separate decision with measured noise implications (see the contrast
/// threshold note), and this pass exists to fix invisible text, not to
/// relitigate the threshold.
const TARGET_RATIO: f64 = 2.0;

// ── chip/badge contrast branch (DS P1-a, pass 2) ────────────────────────────
//
// Measured 0814-08-14: a deck's dark card carried a light badge chip whose
// light text was invisible on it (白底白字). The generic contrast repair
// above measures text against the NEAREST solid ancestor, which for a chip is
// the chip itself — so why a separate branch? Because the generic detector
// also accepts a gradient's first stop as a background, and a chip whose fill
// is a gradient/image makes "the background colour" unprovable. This branch
// only fires where the chip-shape AND its solid background are provable from
// the tree, and leaves every other text to the generic pass.

/// Detection thresholds replicated from
/// `op_design_lint::detectors::typography` (`DEFAULT_NORMAL_THRESHOLD` /
/// `DEFAULT_LARGE_THRESHOLD`) — the thresholds the existing contrast repair
/// path (`repair_text_contrast` via `low_contrast_text`) already applies.
/// Deliberately NOT changed: moving them needs an ab replay first.
const CHIP_NORMAL_THRESHOLD: f64 = 2.5;
const CHIP_LARGE_THRESHOLD: f64 = 2.0;
/// Above this height a filled container is not a chip/badge.
const CHIP_MAX_HEIGHT: f64 = 48.0;
/// A chip may be at most this fraction of its parent's width.
const CHIP_MAX_WIDTH_RATIO: f64 = 0.6;

/// One chip whose text needs re-pointing.
struct ChipOffender {
    node_id: String,
    bg_color: String,
    /// The size-dependent threshold the text was measured against; the
    /// replacement must clear it so the generic pass cannot re-flag it.
    threshold: f64,
}

/// Re-point text that is invisible against its own chip/badge background.
///
/// Fires only when every link of the chain is provable: the text's nearest
/// ancestor with a usable SOLID fill is chip-shaped (<= 48px tall, rounded or
/// clipped, not the root, <= 60% of its parent's width), the chip fill is
/// solid (gradient/image chips are skipped), and the measured ratio is below
/// the same size-dependent threshold the generic detector uses. The
/// replacement colour comes from the document's own palette through the same
/// preference order as [`best_token`]. Returns how many fills were re-pointed.
pub(crate) fn repair_chip_text_contrast(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|node| node.id_str() == root_id)
    else {
        return 0;
    };
    let doc = document_for_lint(sink.state());
    let variables = doc.variables.clone().unwrap_or_default();
    let theme = op_design_lint::node_util::default_theme(doc.themes.as_ref());
    let rects = resolved_sizes(sink.state());

    let mut offenders: Vec<ChipOffender> = Vec::new();
    collect_chip_offenders(
        root,
        &[],
        &variables,
        &theme,
        &rects,
        root_id,
        &mut offenders,
    );
    let mut patches: Vec<(String, String)> = Vec::new();
    for offender in &offenders {
        let Some(token) =
            best_token_above(&offender.bg_color, &variables, &theme, offender.threshold)
        else {
            continue;
        };
        patches.push((offender.node_id.clone(), token));
    }
    let applied = patches.len();
    for (node_id, token) in patches {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(&node_id),
            patch_json: format!(r#"{{"fill":[{{"type":"solid","color":"${token}"}}]}}"#),
            page_id: None,
        });
    }
    applied
}

fn collect_chip_offenders(
    node: &PenNode,
    ancestors: &[&PenNode],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    rects: &HashMap<String, (f64, f64)>,
    root_id: &str,
    out: &mut Vec<ChipOffender>,
) {
    if !is_node_visible(node) {
        return;
    }
    if let PenNode::Text(text) = node {
        if let Some(raw_text) = first_usable_solid_fill(text.fill.as_ref()) {
            if let Some(text_color) = resolve_color_ref(&raw_text, variables, theme) {
                if let Some(bg) =
                    nearest_chip_background(ancestors, variables, theme, rects, root_id)
                {
                    let ratio = op_design_lint::color::color_contrast(&text_color, &bg);
                    if ratio.is_finite() {
                        let threshold = if is_large_text(text) {
                            CHIP_LARGE_THRESHOLD
                        } else {
                            CHIP_NORMAL_THRESHOLD
                        };
                        if ratio < threshold {
                            out.push(ChipOffender {
                                node_id: node_id(node).to_string(),
                                bg_color: bg,
                                threshold,
                            });
                        }
                    }
                }
            }
        }
    }
    let mut next = ancestors.to_vec();
    next.push(node);
    for child in node_children_of(node) {
        collect_chip_offenders(child, &next, variables, theme, rects, root_id, out);
    }
}

fn node_children_of(node: &PenNode) -> &[PenNode] {
    node.children().map(Vec::as_slice).unwrap_or(&[])
}

/// The chip background behind `node`, or `None` when the text's nearest
/// solid-fill ancestor is not a provable chip.
///
/// Walks ancestors closest-first exactly like the detector's
/// `ancestor_bg_color`, with one stricter rule: an ancestor whose first
/// USABLE fill is a gradient/image/shader has no provable solid background —
/// the layers behind it are hidden by it — so the text is skipped rather than
/// measured against a colour that cannot be proven to be the one rendered.
fn nearest_chip_background(
    ancestors: &[&PenNode],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    rects: &HashMap<String, (f64, f64)>,
    root_id: &str,
) -> Option<String> {
    for (index, ancestor) in ancestors.iter().enumerate().rev() {
        if opacity(ancestor) == 0.0 {
            continue;
        }
        if !is_node_visible(ancestor) {
            continue;
        }
        match first_usable_fill_kind(node_fills(ancestor)) {
            FillKind::Transparent => continue,
            // A gradient/image sits between this ancestor and the text; the
            // effective background is unprovable — leave the text alone.
            FillKind::Unprovable => return None,
            FillKind::Solid(raw) => {
                let bg = resolve_color_ref(&raw, variables, theme)?;
                let parent = index.checked_sub(1).map(|parent| ancestors[parent]);
                return is_chip_shape(ancestor, parent, rects, root_id).then_some(bg);
            }
        }
    }
    None
}

enum FillKind {
    /// No usable colour at all — the ancestor is transparent for this purpose.
    Transparent,
    /// A usable colour whose source is not a solid fill — unprovable.
    Unprovable,
    /// A usable solid colour (unresolved — may still be a `$ref`).
    Solid(String),
}

/// The first usable colour in a fill list, classified by source. Skips
/// effectively-transparent fills the same way the detector's
/// `first_solid_color` does (opacity 0 or `#RRGGBB00`).
fn first_usable_fill_kind(fills: Option<&Vec<PenFill>>) -> FillKind {
    let Some(fills) = fills else {
        return FillKind::Transparent;
    };
    for fill in fills {
        match fill {
            PenFill::Solid(body) => {
                if body.opacity == Some(0.0) || is_transparent_hex(&body.color) {
                    continue;
                }
                if body.color.is_empty() {
                    continue;
                }
                return FillKind::Solid(body.color.clone());
            }
            PenFill::Image(_) => continue,
            PenFill::LinearGradient(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                return FillKind::Unprovable;
            }
            PenFill::RadialGradient(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                return FillKind::Unprovable;
            }
            PenFill::MeshGradient(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                return FillKind::Unprovable;
            }
            PenFill::Shader(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                return FillKind::Unprovable;
            }
        }
    }
    FillKind::Transparent
}

/// The text fill, when its first usable colour comes from a solid fill. A
/// gradient text fill is unprovable and skipped, mirroring the chip rule.
fn first_usable_solid_fill(fills: Option<&Vec<PenFill>>) -> Option<String> {
    match first_usable_fill_kind(fills) {
        FillKind::Solid(color) => Some(color),
        _ => None,
    }
}

/// True for a 9-char `#RRGGBBAA` hex whose alpha pair is `00` — mirrored
/// from the detector's `is_transparent_hex`.
fn is_transparent_hex(color: &str) -> bool {
    color.len() == 9 && color[7..].eq_ignore_ascii_case("00")
}

/// The chip-shape gate, every clause required:
/// container, not the pass root, <= [`CHIP_MAX_HEIGHT`] tall, rounded or
/// clipped, and no wider than [`CHIP_MAX_WIDTH_RATIO`] of its parent. Sizes
/// read the authored numeric value first and fall back to the resolved
/// layout; an unknown size fails the gate (narrow predicate).
fn is_chip_shape(
    node: &PenNode,
    parent: Option<&PenNode>,
    rects: &HashMap<String, (f64, f64)>,
    root_id: &str,
) -> bool {
    if node.id_str() == root_id {
        return false;
    }
    let props = match node {
        PenNode::Frame(frame) => &frame.container,
        PenNode::Group(group) => &group.container,
        PenNode::Rectangle(rect) => &rect.container,
        _ => return false,
    };
    let resolved = |node: &PenNode, axis: usize| -> Option<f64> {
        let authored = if axis == 0 {
            node.width_px()
        } else {
            node.height_px()
        };
        authored.or_else(|| {
            rects
                .get(node.id_str())
                .map(|(w, h)| if axis == 0 { *w } else { *h })
        })
    };
    let Some(height) = resolved(node, 1) else {
        return false;
    };
    if !(height > 0.0 && height <= CHIP_MAX_HEIGHT) {
        return false;
    }
    if props.corner_radius.is_none() && props.clip_content != Some(true) {
        return false;
    }
    let (Some(width), Some(parent)) = (resolved(node, 0), parent) else {
        return false;
    };
    let Some(parent_width) = resolved(parent, 0).filter(|w| *w > 0.0) else {
        return false;
    };
    width <= parent_width * CHIP_MAX_WIDTH_RATIO
}

/// Port of the detector's `is_large_text`: `fontSize >= 24`, or `>= 19` with
/// a numeric weight `>= 700`.
fn is_large_text(text: &TextNode) -> bool {
    let Some(font_size) = text.font_size else {
        return false;
    };
    if font_size >= 24.0 {
        return true;
    }
    let weight = match &text.font_weight {
        Some(FontWeight::Number(weight)) => Some(*weight),
        Some(FontWeight::Keyword(_)) | None => None,
    };
    font_size >= 19.0 && weight.is_some_and(|weight| weight >= 700)
}

/// Resolved `(width, height)` per node id through the SAME jian layout pass
/// the geometry validation loop uses.
fn resolved_sizes(state: &EditorState) -> HashMap<String, (f64, f64)> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let mut map = HashMap::new();
    if let Some(page) = scene.active_page() {
        collect_sizes(&page.children, &mut map);
    }
    map
}

fn collect_sizes(nodes: &[SceneNode], map: &mut HashMap<String, (f64, f64)>) {
    for node in nodes {
        let bounds = node.aggregate_bounds();
        map.insert(
            node.id.clone(),
            (f64::from(bounds.size.x), f64::from(bounds.size.y)),
        );
        collect_sizes(&node.children, map);
    }
}

/// Repair invisible text under `root_id`. Returns how many fills were
/// re-pointed.
pub(crate) fn repair_text_contrast(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|node| op_editor_core::PenNodeExt::id_str(*node) == root_id)
    else {
        return 0;
    };
    let doc = document_for_lint(sink.state());
    let offenders = op_design_lint::detectors::typography::low_contrast_text(root, &doc);
    if offenders.is_empty() {
        return 0;
    }
    let variables = doc.variables.clone().unwrap_or_default();
    let theme = op_design_lint::node_util::default_theme(doc.themes.as_ref());

    let mut patches: Vec<(String, String)> = Vec::new();
    for offender in offenders {
        let Some(token) = best_token(&offender.bg_color, &variables, &theme) else {
            continue;
        };
        // Leave it alone when the palette has nothing better than what is
        // already there — a no-op patch would only add churn.
        patches.push((offender.node_id, token));
    }
    let applied = patches.len();
    for (node_id, token) in patches {
        sink.apply(op_editor_core::EditorCommand::PatchNodeData {
            node_id: op_editor_core::NodeId::new(&node_id),
            patch_json: format!(r#"{{"fill":[{{"type":"solid","color":"${token}"}}]}}"#),
            page_id: None,
        });
    }
    applied
}

/// The FIRST palette token, in preference order, that clears
/// [`TARGET_RATIO`] against `bg`.
///
/// Deliberately not "the highest contrast available": on a light board that
/// picks `color-bg-deep`, which is readable but semantically a background
/// token used as ink. Preference order encodes what the token MEANS, and the
/// ratio only decides whether it is usable — so ink wins on light boards and
/// the light tokens take over once ink stops being readable.
fn best_token(
    bg: &str,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> Option<String> {
    best_token_above(bg, variables, theme, TARGET_RATIO)
}

/// [`best_token`] with a caller-chosen bar. The chip branch passes its
/// size-dependent DETECTION threshold so a repaired chip text also stays
/// below the generic detector's radar — same token list, same preference
/// order, one stricter acceptance bar.
fn best_token_above(
    bg: &str,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    bar: f64,
) -> Option<String> {
    CANDIDATE_TOKENS.iter().find_map(|token| {
        let hex = token_hex(token, variables, theme)?;
        let ratio = op_design_lint::color::color_contrast(&hex, bg);
        (ratio.is_finite() && ratio >= bar).then(|| (*token).to_string())
    })
}

/// Resolve one palette token to a hex string.
///
/// Goes through the lint crate's own resolver rather than reading the
/// variable's JSON: a shipped variable is a PER-THEME ARRAY
/// (`[{value:"#FFFFFF",theme:{Mode:Light}}, {value:"#1E293B",theme:{Mode:Dark}}]`),
/// and a hand-rolled `get("value")` returns `None` for every one of them —
/// which is exactly how the first version of this pass repaired nothing while
/// its tests passed against a single-value fixture.
fn token_hex(
    token: &str,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> Option<String> {
    let hex = op_design_lint::node_util::resolve_color_ref(&format!("${token}"), variables, theme)?;
    hex.starts_with('#').then_some(hex)
}

/// The lint crate reads a `PenDocument`; the orchestrator holds an
/// `EditorState`. Project just enough for the detector: its walk needs the
/// node tree plus the variable and theme tables.
fn document_for_lint(state: &op_editor_core::EditorState) -> jian_ops_schema::PenDocument {
    // Clone the document and swap in the ACTIVE page's nodes: the detector
    // walks `children`, and the orchestrator may be working on a page that is
    // not the document's first. Cloning rather than rebuilding field-by-field
    // keeps this correct when the schema grows a field.
    let mut doc = state.doc.clone();
    doc.children = state.active_children().to_vec();
    doc.pages = None;
    doc
}

#[cfg(test)]
#[path = "text_contrast_repair_tests.rs"]
mod text_contrast_repair_tests;
