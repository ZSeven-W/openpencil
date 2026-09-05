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
//! It is tempting to say "a text fill must not use `$--card`". White on
//! a dark board is correct and common — the shipped deck template's closing
//! slide does exactly that. The defect is the measured ratio, so that is what
//! is measured; the variable name is never consulted.

use crate::types::DocSink;

use std::collections::HashMap;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use jian_scene::layout_scene::SceneNode;
use op_design_lint::node_util::{is_node_visible, node_fills, node_id, opacity, resolve_color_ref};
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};

/// Palette tokens allowed as a replacement, most-preferred first.
///
/// All are emitted by `design_system` / `palette_harmonize`, so they exist in
/// any generated document. `--card` is included on purpose: on a dark
/// background it is the readable choice, and excluding it would leave dark
/// boards unrepairable.
/// These are the names `design_system` actually emits. An earlier version of
/// this list was written from memory (made-up text token names) and
/// matched NOTHING in a real document, so every repair silently no-opped —
/// the unit tests passed because their fixture used the invented names too.
const CANDIDATE_TOKENS: &[&str] = &[
    "--foreground",
    "--secondary-foreground",
    "--muted-foreground",
    "--muted-foreground",
    "--card",
    "--background",
];

/// Publication contrast target for every text repair. This matches the
/// design-agent quality gate, while the public lint detector deliberately
/// keeps its separately calibrated 2.5/2.0 informational thresholds.
const TARGET_RATIO: f64 = 4.5;

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

/// Above this height a filled container is not a chip/badge.
const CHIP_MAX_HEIGHT: f64 = 48.0;
/// A chip may be at most this fraction of its parent's width.
const CHIP_MAX_WIDTH_RATIO: f64 = 0.6;

/// One text node whose measured background evidence fails the quality gate.
struct ContrastOffender {
    node_id: String,
    background: Vec<String>,
    /// The quality-gate threshold the text was measured against; the
    /// replacement must clear it so the generic pass cannot re-flag it.
    threshold: f64,
}

/// Re-point text that is invisible against its own chip/badge background.
///
/// Fires only when every link of the chain is provable: the text's nearest
/// painted ancestor is chip-shaped (<= 48px tall, rounded or clipped, not the
/// root, <= 60% of its parent's width), its fill is a resolved solid or
/// gradient, and the measured ratio is below the publication quality gate's
/// text target. The
/// replacement colour comes from the document's own palette through the same
/// preference order as [`best_token_above`]. Returns how many fills were
/// re-pointed.
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

    let mut offenders: Vec<ContrastOffender> = Vec::new();
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
            best_token_above_colors(&offender.background, &variables, &theme, offender.threshold)
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
    out: &mut Vec<ContrastOffender>,
) {
    if !is_node_visible(node) {
        return;
    }
    if let PenNode::Text(text) = node {
        if let Some(text_color) = resolved_text_color(text.fill.as_ref(), variables, theme) {
            if let Some(background) =
                nearest_chip_background(ancestors, variables, theme, rects, root_id)
            {
                if let Some(offender) =
                    below_contrast_threshold(node_id(node), &text_color, background, TARGET_RATIO)
                {
                    out.push(offender);
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
/// painted ancestor is not a provable chip.
fn nearest_chip_background(
    ancestors: &[&PenNode],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    rects: &HashMap<String, (f64, f64)>,
    root_id: &str,
) -> Option<LocatedBackground> {
    let located = nearest_background(ancestors, variables, theme)?;
    let index = located.source_index?;
    let ancestor = ancestors[index];
    let parent = index.checked_sub(1).map(|parent| ancestors[parent]);
    is_chip_shape(ancestor, parent, rects, root_id).then_some(located)
}

enum FillKind {
    /// No usable colour at all — the ancestor is transparent for this purpose.
    Transparent,
    /// A usable colour whose source is not a solid or linear/radial gradient.
    Unprovable,
    /// A usable solid colour (unresolved — may still be a `$ref`).
    Solid(String),
    /// Gradient stop colours (unresolved — may still be `$ref`s).
    Gradient(Vec<String>),
}

enum ResolvedFill {
    Transparent,
    Unprovable,
    Solid([u8; 4]),
    Gradient(Vec<String>),
}

struct LocatedBackground {
    colors: Vec<String>,
    source_index: Option<usize>,
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
                if body.opacity == Some(0.0) || is_transparent_color(&body.color) {
                    continue;
                }
                if body.color.is_empty() {
                    continue;
                }
                return FillKind::Solid(body.color.clone());
            }
            PenFill::Image(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                return FillKind::Unprovable;
            }
            PenFill::LinearGradient(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                return FillKind::Gradient(
                    body.stops.iter().map(|stop| stop.color.clone()).collect(),
                );
            }
            PenFill::RadialGradient(body) => {
                if body.opacity == Some(0.0) {
                    continue;
                }
                return FillKind::Gradient(
                    body.stops.iter().map(|stop| stop.color.clone()).collect(),
                );
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

/// Resolve the first visible fill into the evidence the contrast pass can
/// prove. Images, meshes and shaders remain intentionally unprovable.
fn resolve_fill_kind(
    fills: Option<&Vec<PenFill>>,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> ResolvedFill {
    match first_usable_fill_kind(fills) {
        FillKind::Transparent => ResolvedFill::Transparent,
        FillKind::Unprovable => ResolvedFill::Unprovable,
        FillKind::Solid(raw) => {
            let Some(resolved) = resolve_color_ref(&raw, variables, theme) else {
                return ResolvedFill::Transparent;
            };
            let Some(rgba) = parse_color_rgba(&resolved) else {
                return ResolvedFill::Transparent;
            };
            if rgba[3] == 0 {
                ResolvedFill::Transparent
            } else {
                ResolvedFill::Solid(rgba)
            }
        }
        FillKind::Gradient(stops) => {
            let colors: Vec<String> = stops
                .iter()
                .filter_map(|stop| resolve_contrast_color(stop, variables, theme))
                .collect();
            if colors.is_empty() {
                ResolvedFill::Unprovable
            } else {
                ResolvedFill::Gradient(colors)
            }
        }
    }
}

/// Find the nearest rendered background. A semi-transparent solid is
/// composited over the next opaque solid ancestor; a missing background is
/// the white canvas, while a non-provable paint causes the caller to skip the
/// text.
fn nearest_background(
    ancestors: &[&PenNode],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> Option<LocatedBackground> {
    for (index, ancestor) in ancestors.iter().enumerate().rev() {
        if opacity(ancestor) == 0.0 || !is_node_visible(ancestor) {
            continue;
        }
        match resolve_fill_kind(node_fills(ancestor), variables, theme) {
            ResolvedFill::Transparent => continue,
            ResolvedFill::Unprovable => return None,
            ResolvedFill::Gradient(colors) => {
                return Some(LocatedBackground {
                    colors,
                    source_index: Some(index),
                });
            }
            ResolvedFill::Solid(rgba) => {
                let color = if rgba[3] == u8::MAX {
                    rgb_hex(rgba)
                } else {
                    let under = nearest_opaque_solid_color(index, ancestors, variables, theme)
                        .unwrap_or([u8::MAX; 4]);
                    composite_over(rgba, under)
                };
                return Some(LocatedBackground {
                    colors: vec![color],
                    source_index: Some(index),
                });
            }
        }
    }
    Some(LocatedBackground {
        colors: vec!["#FFFFFF".to_string()],
        source_index: None,
    })
}

fn nearest_opaque_solid_color(
    from_index: usize,
    ancestors: &[&PenNode],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> Option<[u8; 4]> {
    for ancestor in ancestors[..from_index].iter().rev() {
        if opacity(ancestor) == 0.0 || !is_node_visible(ancestor) {
            continue;
        }
        if let ResolvedFill::Solid(rgba) = resolve_fill_kind(node_fills(ancestor), variables, theme)
        {
            if rgba[3] == u8::MAX {
                return Some(rgba);
            }
        }
    }
    None
}

fn resolved_text_color(
    fills: Option<&Vec<PenFill>>,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> Option<String> {
    let raw = first_usable_solid_fill(fills)?;
    resolve_contrast_color(&raw, variables, theme)
}

fn below_contrast_threshold(
    node_id: &str,
    text_color: &str,
    background: LocatedBackground,
    threshold: f64,
) -> Option<ContrastOffender> {
    let best = background
        .colors
        .iter()
        .map(|color| op_design_lint::color::color_contrast(text_color, color))
        .filter(|ratio| ratio.is_finite())
        .max_by(|left, right| left.total_cmp(right))?;
    (best < threshold).then(|| ContrastOffender {
        node_id: node_id.to_string(),
        background: background.colors,
        threshold,
    })
}

/// The text fill, when its first usable colour comes from a solid fill. A
/// gradient text fill is unprovable and skipped, mirroring the chip rule.
fn first_usable_solid_fill(fills: Option<&Vec<PenFill>>) -> Option<String> {
    match first_usable_fill_kind(fills) {
        FillKind::Solid(color) => Some(color),
        _ => None,
    }
}

/// True for a fully transparent colour, including `#RRGGBBAA` and `rgba()`.
fn is_transparent_color(color: &str) -> bool {
    parse_color_rgba(color).is_some_and(|rgba| rgba[3] == 0)
}

fn parse_color_rgba(color: &str) -> Option<[u8; 4]> {
    const OPTIONS: op_util::hex_color::HexOptions = op_util::hex_color::HexOptions {
        require_hash: true,
        allow_rgb_shorthand: true,
        allow_rgba_shorthand: false,
        allow_alpha: true,
    };
    if let Some(rgba) = op_util::hex_color::parse_hex_rgba8(color, OPTIONS) {
        return Some(rgba);
    }

    let lower = color.trim().to_ascii_lowercase();
    let (body, has_alpha) = if let Some(body) = lower.strip_prefix("rgba(") {
        (body.strip_suffix(')')?, true)
    } else if let Some(body) = lower.strip_prefix("rgb(") {
        (body.strip_suffix(')')?, false)
    } else {
        return None;
    };
    let parts: Vec<&str> = body.split(',').map(str::trim).collect();
    if parts.len() != if has_alpha { 4 } else { 3 } {
        return None;
    }
    let channel = |value: &str| -> Option<u8> {
        let value = value.parse::<f64>().ok()?;
        (value.is_finite() && (0.0..=255.0).contains(&value)).then_some(value.round() as u8)
    };
    let alpha = if has_alpha {
        let value = parts[3].parse::<f64>().ok()?;
        (value.is_finite() && (0.0..=1.0).contains(&value))
            .then_some((value * 255.0).round() as u8)?
    } else {
        u8::MAX
    };
    Some([
        channel(parts[0])?,
        channel(parts[1])?,
        channel(parts[2])?,
        alpha,
    ])
}

fn resolve_contrast_color(
    raw: &str,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> Option<String> {
    let resolved = resolve_color_ref(raw, variables, theme)?;
    let rgba = parse_color_rgba(&resolved)?;
    Some(rgb_hex(rgba))
}

fn rgb_hex(rgba: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2])
}

/// Composite an RGBA foreground over an opaque background in sRGB channels.
fn composite_over(foreground: [u8; 4], background: [u8; 4]) -> String {
    let alpha = f64::from(foreground[3]) / 255.0;
    let channel = |front: u8, back: u8| {
        (f64::from(front) * alpha + f64::from(back) * (1.0 - alpha))
            .round()
            .clamp(0.0, 255.0) as u8
    };
    rgb_hex([
        channel(foreground[0], background[0]),
        channel(foreground[1], background[1]),
        channel(foreground[2], background[2]),
        u8::MAX,
    ])
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

fn collect_contrast_offenders(
    node: &PenNode,
    ancestors: &[&PenNode],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    out: &mut Vec<ContrastOffender>,
) {
    if !is_node_visible(node) {
        return;
    }
    if let PenNode::Text(text) = node {
        if let Some(text_color) = resolved_text_color(text.fill.as_ref(), variables, theme) {
            if let Some(background) = nearest_background(ancestors, variables, theme) {
                if let Some(offender) =
                    below_contrast_threshold(node_id(node), &text_color, background, TARGET_RATIO)
                {
                    out.push(offender);
                }
            }
        }
    }
    let mut next = ancestors.to_vec();
    next.push(node);
    for child in node_children_of(node) {
        collect_contrast_offenders(child, &next, variables, theme, out);
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
    let variables = doc.variables.clone().unwrap_or_default();
    let theme = op_design_lint::node_util::default_theme(doc.themes.as_ref());
    let mut offenders = Vec::new();
    collect_contrast_offenders(root, &[], &variables, &theme, &mut offenders);

    let mut patches: Vec<(String, String)> = Vec::new();
    for offender in offenders {
        let Some(token) =
            best_token_above_colors(&offender.background, &variables, &theme, offender.threshold)
        else {
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
/// picks `--background`, which is readable but semantically a background
/// token used as ink. Preference order encodes what the token MEANS, and the
/// ratio only decides whether it is usable — so ink wins on light boards and
/// the light tokens take over once ink stops being readable.
#[cfg(test)]
fn best_token(
    bg: &str,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> Option<String> {
    best_token_above(bg, variables, theme, TARGET_RATIO)
}

/// [`best_token`] with a caller-chosen bar. Both finalizer branches pass the
/// offender's own threshold so a repair cannot remain below the gate that
/// selected it — same token list, same preference order, exact acceptance
/// bar.
#[cfg(test)]
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

fn best_token_above_colors(
    backgrounds: &[String],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    bar: f64,
) -> Option<String> {
    if backgrounds.is_empty() {
        return None;
    }
    CANDIDATE_TOKENS.iter().find_map(|token| {
        let hex = token_hex(token, variables, theme)?;
        let worst = backgrounds
            .iter()
            .map(|background| op_design_lint::color::color_contrast(&hex, background))
            .filter(|ratio| ratio.is_finite())
            .min_by(|left, right| left.total_cmp(right))?;
        (worst >= bar).then(|| (*token).to_string())
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
