//! Unify horizontal margins across transparent sibling sections (DS P1.5).
//!
//! Measured 0815-08-15 on a 1080x1440 portrait card: the root carried NO
//! horizontal padding and delegated the page margin down to its sections —
//! of seven `fill_container` sections, four authored their own horizontal
//! padding ([0,80] / [24,80]) and three carried none, so their content sat
//! flush against the canvas left edge and every section's content left edge
//! drifted. This pass pulls the margin duty back to ONE group norm: every
//! transparent sibling section gets the group's MAXIMUM horizontal padding
//! per side. Max-raise means the repair can only ADD whitespace — it cannot
//! create overflow, and it never touches vertical rhythm or content.
//!
//! ## Why the predicate is narrow (provability, DS P1-a iron law)
//!
//! - The root's own padding must be absent or all-zero: a root that already
//!   carries margins owns them, and double-insetting sections would be a
//!   new defect.
//! - Every candidate section must be a direct child Frame whose authored
//!   width is `fill_container` — a fixed-width band's flush edge is authored
//!   geometry, not margin drift.
//! - Every candidate must have NO visible own fill: a coloured band owns its
//!   edge-to-edge look (the 0808 "深色带被剥" case — the whole pass vetoes
//!   on one, never strips per-section).
//! - The horizontal paddings must actually differ: a group that already
//!   agrees has no drift to prove.
//! - The REAL jian layout must prove at least one section's text (or narrow
//!   in-flow content) sits < 24px from the canvas edge. Text always counts;
//!   a non-text node spanning the full root width is a band (full-bleed
//!   intent or a layout row — no evidence either way), an authored x/y
//!   overlay is absolute decoration, and both are skipped.
//!
//! A phone screen is never touched: edge-to-edge content is ITS legal
//! contract, so flush content there proves nothing.

use super::*;

use jian_ops_schema::node::container::ContainerProps;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use jian_scene::layout_scene::SceneNode;

use crate::cleanup::cleanup_equalize_siblings::{padding_edges, padding_value};
use crate::design_type::DesignForm;

/// Minimum count of fill_container frame sections under the root.
const MIN_SECTIONS: usize = 3;
/// A content edge closer than this to the canvas edge proves the margin is
/// missing rather than merely small (same band as the deck/card floor).
const SECTION_EDGE_GAP: f64 = 24.0;

/// Resolved geometry of one node (absolute scene coordinates).
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    w: f64,
    h: f64,
}

/// Per-root cleanup pass: raise the horizontal padding of transparent
/// sibling sections to the group maximum, per side, when drift is proven.
/// Returns `true` iff at least one section was patched.
pub(super) fn unify_transparent_section_margins(sink: &mut dyn DocSink, root_id: &str) -> bool {
    // Gate 0 — the mobile edge-to-edge contract is legal content, never a
    // missing margin, so a phone screen has no proof to offer this pass.
    if crate::geometry_validation::root_design_form(sink.state(), root_id)
        == DesignForm::MobileScreen
    {
        return false;
    }
    let Some(root) = find_root(sink.state(), root_id) else {
        return false;
    };
    let Some(root_props) = container_props(root) else {
        return false;
    };
    // Gate 1 — the root must not already own a margin. Missing padding reads
    // as all-zero; a variable-bound padding cannot be proven zero, so it
    // blocks the pass too.
    let Some(root_edges) = padding_edges(root_props.padding.as_ref()) else {
        return false;
    };
    if root_edges.iter().any(|edge| *edge != 0.0) {
        return false;
    }
    let Some(children) = root.children() else {
        return false;
    };

    // Gate 2 — at least three direct-child Frames authored `fill_container`
    // wide, each with a provable (numeric) padding.
    let mut candidates: Vec<(&PenNode, [f64; 4])> = Vec::new();
    for child in children {
        let PenNode::Frame(frame) = child else {
            continue;
        };
        if !frame.container.is_fill_container_width() {
            continue;
        }
        let Some(edges) = padding_edges(frame.container.padding.as_ref()) else {
            continue;
        };
        candidates.push((child, edges));
    }
    if candidates.len() < MIN_SECTIONS {
        return false;
    }

    // Gate 3 — a visible own fill on ANY candidate vetoes the whole pass
    // (the coloured-band red line from the 0808 strip accident).
    if candidates
        .iter()
        .any(|(node, _)| has_visible_own_fill(node))
    {
        return false;
    }

    // Gate 4 — the horizontal paddings must differ somewhere; an agreed
    // group has no drift to prove.
    let max_left = candidates
        .iter()
        .map(|(_, edges)| edges[3])
        .fold(0.0, f64::max);
    let max_right = candidates
        .iter()
        .map(|(_, edges)| edges[1])
        .fold(0.0, f64::max);
    let drifts = candidates
        .iter()
        .any(|(_, edges)| edges[1] != max_right || edges[3] != max_left);
    if !drifts {
        return false;
    }

    // Gate 5 — the REAL layout must prove content flush against the canvas
    // edge. No geometry proof, no repair.
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let Some(page) = scene.active_page() else {
        return false;
    };
    let Some(root_scene) = find_scene_node(&page.children, root_id) else {
        return false;
    };
    let root_bounds = scene_rect(root_scene);
    let mut flush_proven = false;
    for (section, _) in &candidates {
        let Some(section_scene) = find_scene_node(&root_scene.children, section.id_str()) else {
            continue;
        };
        if section_has_flush_content(section, &section_scene.children, &root_bounds) {
            flush_proven = true;
            break;
        }
    }
    if !flush_proven {
        return false;
    }

    // Repair — per-side max-raise, vertical component untouched. A section
    // already at the group max is not patched (nothing to raise). Targets
    // are collected as owned ids first so the state borrows end before any
    // command is applied.
    let mut targets: Vec<(NodeId, [f64; 4])> = Vec::new();
    for (section, edges) in &candidates {
        let new_right = edges[1].max(max_right);
        let new_left = edges[3].max(max_left);
        if new_right == edges[1] && new_left == edges[3] {
            continue;
        }
        targets.push((
            NodeId::new(section.id_str()),
            [edges[0], new_right, edges[2], new_left],
        ));
    }
    if targets.is_empty() {
        return false;
    }
    for (node_id, new_edges) in targets {
        let patch = serde_json::to_string(&padding_value(new_edges)).unwrap_or_default();
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json: format!(r#"{{"padding":{patch}}}"#),
            page_id: None,
        });
    }
    true
}

/// True when any descendant of `section_scene`'s children is CONTENT whose
/// left or right edge sits < [`SECTION_EDGE_GAP`] from the canvas edge.
/// Text always counts; a non-text node counts only when it is narrower than
/// the root (a full-width node is a band, an authored x/y overlay is
/// absolute decoration — neither is evidence of margin drift).
fn section_has_flush_content(
    section: &PenNode,
    children: &[SceneNode],
    root_bounds: &Rect,
) -> bool {
    for child_scene in children {
        let Some(child) = find_pen_descendant(section, &child_scene.id) else {
            continue;
        };
        let rect = scene_rect(child_scene);
        if !is_flush_evidence(child, &rect, root_bounds) {
            continue;
        }
        let left_gap = rect.x - root_bounds.x;
        let right_gap = (root_bounds.x + root_bounds.w) - (rect.x + rect.w);
        if left_gap < SECTION_EDGE_GAP || right_gap < SECTION_EDGE_GAP {
            return true;
        }
        if section_has_flush_content(child, &child_scene.children, root_bounds) {
            return true;
        }
    }
    false
}

/// A node is flush evidence when it is text, or a non-text in-flow node
/// narrower than the root. Authored x/y overlays and full-bleed bands
/// (resolved size == root size) are decoration; full-width bands are layout
/// rows — none of them prove a missing margin.
fn is_flush_evidence(node: &PenNode, rect: &Rect, root_bounds: &Rect) -> bool {
    if matches!(node, PenNode::Text(_)) {
        return true;
    }
    if node.base().x.is_some() || node.base().y.is_some() {
        return false;
    }
    if (rect.w - root_bounds.w).abs() <= 0.5 && (rect.h - root_bounds.h).abs() <= 0.5 {
        return false;
    }
    rect.w < root_bounds.w - 0.5
}

/// True when the node carries a fill that is visibly painted: any fill
/// entry whose opacity is not 0, and for solids whose colour alpha is not 0.
/// A fill colour that does not parse as hex (a `$variable` ref, say) is
/// treated as visible — transparency cannot be proven there.
fn has_visible_own_fill(node: &PenNode) -> bool {
    let Some(fills) = op_editor_core::fills::node_fills(node) else {
        return false;
    };
    fills.iter().any(fill_is_visible)
}

fn fill_is_visible(fill: &PenFill) -> bool {
    let opacity = match fill {
        PenFill::Solid(body) => body.opacity,
        PenFill::LinearGradient(body) => body.opacity,
        PenFill::RadialGradient(body) => body.opacity,
        PenFill::MeshGradient(body) => body.opacity,
        PenFill::Shader(body) => body.opacity,
        PenFill::Image(body) => body.opacity,
    };
    if opacity == Some(0.0) {
        return false;
    }
    match fill {
        PenFill::Solid(body) => op_util::hex_color::parse_hex_rgba8(
            &body.color,
            op_util::hex_color::HexOptions::LENIENT,
        )
        .map(|rgba| rgba[3] != 0)
        .unwrap_or(true),
        _ => true,
    }
}

fn container_props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(frame) => Some(&frame.container),
        PenNode::Group(group) => Some(&group.container),
        PenNode::Rectangle(rect) => Some(&rect.container),
        _ => None,
    }
}

fn find_scene_node<'a>(nodes: &'a [SceneNode], node_id: &str) -> Option<&'a SceneNode> {
    for node in nodes {
        if node.id == node_id {
            return Some(node);
        }
        if let Some(found) = find_scene_node(&node.children, node_id) {
            return Some(found);
        }
    }
    None
}

fn find_pen_descendant<'a>(node: &'a PenNode, node_id: &str) -> Option<&'a PenNode> {
    if node.id_str() == node_id {
        return Some(node);
    }
    let children = node.children()?;
    for child in children {
        if let Some(found) = find_pen_descendant(child, node_id) {
            return Some(found);
        }
    }
    None
}

fn scene_rect(node: &SceneNode) -> Rect {
    let bounds = node.aggregate_bounds();
    Rect {
        x: f64::from(bounds.origin.x),
        w: f64::from(bounds.size.x),
        h: f64::from(bounds.size.y),
    }
}

/// Container width authored as `fill_container`.
trait FillContainerWidth {
    fn is_fill_container_width(&self) -> bool;
}

impl FillContainerWidth for ContainerProps {
    fn is_fill_container_width(&self) -> bool {
        matches!(
            self.width,
            Some(jian_ops_schema::sizing::SizingBehavior::Keyword(
                jian_ops_schema::sizing::SizingKeyword::FillContainer
            ))
        )
    }
}

#[cfg(test)]
#[path = "cleanup_section_margins_tests.rs"]
mod tests;
