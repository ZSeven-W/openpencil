//! Sibling style-drift repair for grids and rows of same-shaped tiles.
//!
//! Evidence (real Opus 5 output, 2026-09-05, food-delivery home): a 3x3
//! category grid where every tile is `frame[frame[icon_font],text]` — an
//! icon chip ("X 图标底", 56x56) over a label. The first column's three
//! tiles carried NO stroke and a `$--primary` (orange) icon while the other
//! six had a 1px `$--border` stroke and `$--secondary-foreground` icons.
//! Rendered, the first column reads as broken/misaligned. Sibling tiles
//! that share one structure must share one style; the model drifted.
//!
//! ## Family
//!
//! A family is a group of >= 4 children of one parent (any layout) that
//! are all frames sharing one STRUCTURAL SIGNATURE — the tree of node kinds
//! and child counts, ignoring names, text content and icon names (e.g.
//! `frame[frame[icon_font],text]`). The trivial signature `frame` (no
//! descendants) proves nothing and never forms a family. A 3x3 grid planned
//! as three row frames under one grid parent counts too: when every row
//! shares one signature, the rows are flattened one level and the
//! grandchildren vote as one family.
//!
//! ## Facts and the majority rule
//!
//! The signature is walked in lockstep across the family. At every frame
//! position the pass votes on stroke presence + thickness, cornerRadius and
//! the primary solid fill token (`$variable` reference or literal hex); at
//! every icon_font position it votes on the primary fill token. A value is
//! the family norm only when MORE THAN HALF the members AND AT LEAST 4
//! members hold it — "fewer than 4 members agree" means there is no
//! provable majority and the position is left alone. Two provability rules
//! mirror `cleanup_equalize_siblings`: a non-solid primary fill (gradient /
//! image / ...) on any member makes the whole position unprovable, and a
//! position mixing `$ref`s with literal hex is never touched (the two value
//! systems cannot be proven synonymous). A majority of "no fill" or "no
//! stroke" is never enforced — removing paint is a destructive move this
//! pass does not make, and a stroked minority can be load-bearing
//! (clip-stroke safety padding keys off child strokes). Text content, icon
//! names, sizes and positions are never modified.
//!
//! ## The selected-tile carve-out
//!
//! A deliberately highlighted "selected" tile is exactly one member, so a
//! family with exactly ONE outlier is left unrepaired when that outlier
//! differs on paint facts only (fills and stroke); two or more outliers
//! are model drift and are all repaired. A member that carries a `state`
//! schema or a "selected" name marker is never edited. The status bar and
//! the bottom tab bar subtrees are skipped outright — chrome is not
//! content.

use crate::types::DocSink;

use std::collections::BTreeMap;

use jian_ops_schema::node::container::CornerRadius;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use op_design_lint::node_util::{children as node_children, is_node_visible, node_kind_str};
use op_editor_core::fills::{first_solid_stroke_hex, node_stroke_width};
use op_editor_core::variables_resolve::is_variable_ref;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::Value;

/// Minimum family size: two tiles agree by accident, three can drift
/// together; four same-shaped tiles are a grid.
const MIN_MEMBERS: usize = 4;
/// A majority must be held by at least this many members — fewer than 4
/// members agreeing on a fact means there is no provable norm at all.
const MIN_AGREE: usize = 4;

/// Per-root cleanup pass: unify drifted style facts inside every detected
/// twin family under `root_id`. Returns how many commands were applied.
pub(crate) fn repair_sibling_style_drift(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    ) else {
        return 0;
    };
    let mut commands = Vec::new();
    collect_family_commands(root, &mut commands);
    let applied = commands.len();
    for command in commands {
        sink.apply(command);
    }
    applied
}

/// Depth-first walk that plans every family's repairs. Chrome subtrees are
/// never entered: the status bar and the bottom tab bar are styled apart
/// from the content grid on purpose.
fn collect_family_commands(node: &PenNode, commands: &mut Vec<EditorCommand>) {
    if super::is_status_bar(node) || super::is_bottom_nav_section(node) {
        return;
    }
    let Some(children) = node.children() else {
        return;
    };
    for family in twin_families(children) {
        plan_family_commands(&family, commands);
    }
    for child in children {
        collect_family_commands(child, commands);
    }
}

/// The twin families among `children`: every group of >= [`MIN_MEMBERS`]
/// same-signature frames votes independently. When no direct family exists
/// and every child is a frame sharing ONE signature, the children are rows
/// of a grid — flatten one level and group the grandchildren instead.
fn twin_families(children: &[PenNode]) -> Vec<Vec<Member<'_>>> {
    let direct = sibling_groups(children.iter().collect());
    if !direct.is_empty() {
        return direct;
    }
    let rows: Vec<&PenNode> = children
        .iter()
        .filter(|child| is_node_visible(child) && matches!(child, PenNode::Frame(_)))
        .collect();
    if rows.len() < 2 {
        return Vec::new();
    }
    let row_signature = structural_signature(rows[0]);
    if !rows
        .iter()
        .all(|row| structural_signature(row) == row_signature)
    {
        return Vec::new();
    }
    sibling_groups(
        rows.iter()
            .flat_map(|row| node_children(row).iter())
            .collect(),
    )
}

/// Group candidate siblings by structural signature; every group with at
/// least [`MIN_MEMBERS`] members is a family. The trivial signature `frame`
/// (a childless frame) is shared by EVERY childless frame and proves
/// nothing, so it never groups.
fn sibling_groups<'a>(candidates: Vec<&'a PenNode>) -> Vec<Vec<Member<'a>>> {
    let mut by_signature: BTreeMap<String, Vec<&PenNode>> = BTreeMap::new();
    for node in candidates {
        if !is_node_visible(node) || !matches!(node, PenNode::Frame(_)) {
            continue;
        }
        let signature = structural_signature(node);
        if !signature.contains('[') {
            continue;
        }
        by_signature.entry(signature).or_default().push(node);
    }
    by_signature
        .into_values()
        .filter(|group| group.len() >= MIN_MEMBERS)
        .map(|group| group.into_iter().map(Member::of).collect())
        .collect()
}

/// The structural signature: the tree of node kinds and child counts,
/// ignoring names, text content and icon names. Two tiles with the same
/// signature are structural twins, e.g. `frame[frame[icon_font],text]`.
fn structural_signature(node: &PenNode) -> String {
    let mut out = String::new();
    push_signature(node, &mut out);
    out
}

fn push_signature(node: &PenNode, out: &mut String) {
    out.push_str(node_kind_str(node));
    let children = node_children(node);
    if children.is_empty() {
        return;
    }
    out.push('[');
    for (index, child) in children.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_signature(child, out);
    }
    out.push(']');
}

/// One family member: its fact positions in DFS pre-order. Equal
/// signatures guarantee every member has the same position count, so
/// index `k` names the same subtree slot across the whole family.
struct Member<'a> {
    /// Frame positions, the tile root included.
    frames: Vec<&'a PenNode>,
    /// icon_font positions.
    icons: Vec<&'a PenNode>,
    /// A `state` schema or a "selected" name marks deliberate highlighting.
    selected_marked: bool,
}

impl<'a> Member<'a> {
    fn of(node: &'a PenNode) -> Self {
        let mut frames = Vec::new();
        let mut icons = Vec::new();
        collect_fact_positions(node, &mut frames, &mut icons);
        Self {
            frames,
            icons,
            selected_marked: carries_selected_marker(node),
        }
    }
}

fn collect_fact_positions<'a>(
    node: &'a PenNode,
    frames: &mut Vec<&'a PenNode>,
    icons: &mut Vec<&'a PenNode>,
) {
    match node {
        PenNode::Frame(_) => frames.push(node),
        PenNode::IconFont(_) => icons.push(node),
        _ => {}
    }
    for child in node_children(node) {
        collect_fact_positions(child, frames, icons);
    }
}

/// A tile is deliberately highlighted when it carries a component `state`
/// schema (e.g. a `selected` entry) or its name says so.
fn carries_selected_marker(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    if frame.state.as_ref().is_some_and(|state| !state.is_empty()) {
        return true;
    }
    let name = frame
        .base
        .name
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    name.contains("selected")
}

/// Stroke vote: presence + thickness (the spec's fact). The majority
/// camp's colour is adopted only when a stroke has to be (re)written.
#[derive(Debug, Clone, PartialEq)]
enum StrokeVote {
    Absent,
    Present(f64),
}

fn stroke_vote(node: &PenNode) -> StrokeVote {
    match node_stroke_width(node) {
        None => StrokeVote::Absent,
        Some(width) => StrokeVote::Present(width),
    }
}

/// cornerRadius fact. `f64` bit patterns keep the value exactly comparable
/// without a float-`Eq` newtype.
#[derive(Debug, Clone, PartialEq)]
enum RadiusFact {
    Absent,
    Uniform(u64),
    PerCorner([u64; 4]),
}

fn radius_fact(node: &PenNode) -> RadiusFact {
    let PenNode::Frame(frame) = node else {
        return RadiusFact::Absent;
    };
    match &frame.container.corner_radius {
        None => RadiusFact::Absent,
        Some(CornerRadius::Uniform(radius)) => RadiusFact::Uniform(radius.to_bits()),
        Some(CornerRadius::PerCorner(corners)) => RadiusFact::PerCorner(corners.map(f64::to_bits)),
    }
}

/// Serialize the radius norm back to the schema's `cornerRadius` shape;
/// `null` clears the field (the patch merge maps `null` onto `None`).
fn radius_patch(fact: &RadiusFact) -> Value {
    match fact {
        RadiusFact::Absent => Value::Null,
        RadiusFact::Uniform(bits) => Value::from(f64::from_bits(*bits)),
        RadiusFact::PerCorner(bits) => Value::from(bits.map(f64::from_bits).to_vec()),
    }
}

/// What one position's primary fill holds, for the vote.
enum FillFact {
    /// No fill, an empty fill list, or an empty colour — a votable missing
    /// value that can join a concrete majority but never define one.
    Missing,
    /// The primary fill is NOT solid (gradient / image / ...) — the
    /// position is not provably a colour slot.
    NonSolid,
    /// A solid literal hex or `$variable` reference, compared as-is.
    Solid(String),
}

fn fill_fact(node: &PenNode) -> FillFact {
    let Some(fills) = op_editor_core::fills::node_fills(node) else {
        return FillFact::Missing;
    };
    match fills.first() {
        None => FillFact::Missing,
        Some(PenFill::Solid(body)) if !body.color.is_empty() => FillFact::Solid(body.color.clone()),
        Some(PenFill::Solid(_)) => FillFact::Missing,
        Some(_) => FillFact::NonSolid,
    }
}

/// The proven fill norm for one position plus every member's vote, or
/// `None` when the position is unprovable or has no CONCRETE majority.
fn fill_norm(nodes: &[&PenNode]) -> Option<(String, Vec<Option<String>>)> {
    let facts: Vec<FillFact> = nodes.iter().map(|node| fill_fact(node)).collect();
    if facts.iter().any(|fact| matches!(fact, FillFact::NonSolid)) {
        return None;
    }
    let votes: Vec<Option<String>> = facts
        .iter()
        .map(|fact| match fact {
            FillFact::Solid(token) => Some(token.clone()),
            FillFact::Missing | FillFact::NonSolid => None,
        })
        .collect();
    // Reference strings and literals cannot be proven synonymous — a
    // position mixing them is never touched.
    let has_ref = votes.iter().flatten().any(|token| is_variable_ref(token));
    let has_literal = votes.iter().flatten().any(|token| !is_variable_ref(token));
    if has_ref && has_literal {
        return None;
    }
    // A majority of "no fill" is never enforced: removing a fill is a
    // destructive move this pass does not make.
    let Some(Some(norm)) = majority_of(&votes) else {
        return None;
    };
    Some((norm, votes))
}

/// The value held by MORE THAN HALF the family AND at least [`MIN_AGREE`]
/// members, when such a value exists. A tie has no norm.
fn majority_of<T: PartialEq + Clone>(values: &[T]) -> Option<T> {
    let n = values.len();
    values
        .iter()
        .find(|candidate| {
            let count = values.iter().filter(|value| value == candidate).count();
            count * 2 > n && count >= MIN_AGREE
        })
        .cloned()
}

/// One pending fact alignment on one member.
enum Edit {
    /// Stroke presence/thickness alignment (a paint fact).
    Stroke {
        node_id: String,
        current: StrokeVote,
        target: StrokeTarget,
    },
    /// cornerRadius alignment (a SHAPE fact — the selected-tile carve-out
    /// does not cover it).
    Radius { node_id: String, patch: Value },
    /// Frame / icon fill token alignment (a paint fact).
    Fill { node_id: String, token: String },
}

/// Where a drifted stroke must end up; the colour is the majority camp's
/// representative solid stroke colour (adopted, never voted on). There is
/// no `Absent` target: a majority of "no stroke" is never enforced, since
/// removing paint is destructive.
#[derive(Debug, Clone)]
struct StrokeTarget {
    color: Option<String>,
    width: f64,
}

impl Edit {
    /// Fills and strokes are paint; cornerRadius is shape.
    fn is_paint(&self) -> bool {
        !matches!(self, Edit::Radius { .. })
    }

    fn push_commands(&self, commands: &mut Vec<EditorCommand>) {
        match self {
            Edit::Stroke {
                node_id,
                current,
                target,
            } => {
                // `SetNodeStrokeHex` attaches a fresh stroke when the node
                // has none, so the colour lands first and the thickness
                // corrects it afterwards.
                if matches!(current, StrokeVote::Absent) {
                    if let Some(color) = &target.color {
                        commands.push(EditorCommand::SetNodeStrokeHex {
                            node_id: NodeId::new(node_id.clone()),
                            hex: color.clone(),
                        });
                    }
                }
                commands.push(EditorCommand::SetNodeStrokeWidth {
                    node_id: NodeId::new(node_id.clone()),
                    width: target.width as f32,
                });
            }
            Edit::Radius { node_id, patch } => commands.push(EditorCommand::PatchNodeData {
                node_id: NodeId::new(node_id.clone()),
                patch_json: serde_json::json!({ "cornerRadius": patch }).to_string(),
                page_id: None,
            }),
            Edit::Fill { node_id, token } => commands.push(EditorCommand::SetNodeFillHex {
                node_id: NodeId::new(node_id.clone()),
                hex: token.clone(),
            }),
        }
    }
}

/// Plan one family's repairs. Votes run per fact position; the outlier
/// filter then decides whether the family is drift at all.
fn plan_family_commands(family: &[Member], commands: &mut Vec<EditorCommand>) {
    let n = family.len();
    let mut member_edits: Vec<Vec<Edit>> = (0..n).map(|_| Vec::new()).collect();

    let frame_slots = family.first().map_or(0, |member| member.frames.len());
    for slot in 0..frame_slots {
        let nodes: Vec<&PenNode> = family.iter().map(|member| member.frames[slot]).collect();

        let votes: Vec<StrokeVote> = nodes.iter().map(|node| stroke_vote(node)).collect();
        // Only a CONCRETE stroke norm is enforced: a majority of "no
        // stroke" never strips the minority's strokes (see the header).
        if let Some(StrokeVote::Present(width)) = majority_of(&votes) {
            let norm = StrokeVote::Present(width);
            let target = StrokeTarget {
                color: majority_stroke_color(&nodes, &votes, width),
                width,
            };
            for (index, vote) in votes.iter().enumerate() {
                if vote != &norm {
                    member_edits[index].push(Edit::Stroke {
                        node_id: nodes[index].id_str().to_string(),
                        current: vote.clone(),
                        target: target.clone(),
                    });
                }
            }
        }

        let radii: Vec<RadiusFact> = nodes.iter().map(|node| radius_fact(node)).collect();
        if let Some(norm) = majority_of(&radii) {
            for (index, fact) in radii.iter().enumerate() {
                if fact != &norm {
                    member_edits[index].push(Edit::Radius {
                        node_id: nodes[index].id_str().to_string(),
                        patch: radius_patch(&norm),
                    });
                }
            }
        }

        plan_fill_votes(&nodes, &mut member_edits);
    }

    let icon_slots = family.first().map_or(0, |member| member.icons.len());
    for slot in 0..icon_slots {
        let nodes: Vec<&PenNode> = family.iter().map(|member| member.icons[slot]).collect();
        plan_fill_votes(&nodes, &mut member_edits);
    }

    // The selected-tile carve-out: exactly one outlier differing on paint
    // facts only is deliberate highlighting, not drift.
    let outliers: Vec<usize> = (0..n)
        .filter(|index| !member_edits[*index].is_empty())
        .collect();
    if let [only] = outliers.as_slice() {
        if member_edits[*only].iter().all(Edit::is_paint) {
            return;
        }
    }
    for index in outliers {
        // A tile carrying a `state` / selected marker is never edited.
        if family[index].selected_marked {
            continue;
        }
        for edit in &member_edits[index] {
            edit.push_commands(commands);
        }
    }
}

fn plan_fill_votes(nodes: &[&PenNode], member_edits: &mut [Vec<Edit>]) {
    let Some((norm, votes)) = fill_norm(nodes) else {
        return;
    };
    for (index, vote) in votes.iter().enumerate() {
        if vote.as_ref() != Some(&norm) {
            member_edits[index].push(Edit::Fill {
                node_id: nodes[index].id_str().to_string(),
                token: norm.clone(),
            });
        }
    }
}

/// The solid stroke colour of the first majority-camp member that has one.
fn majority_stroke_color(nodes: &[&PenNode], votes: &[StrokeVote], width: f64) -> Option<String> {
    nodes
        .iter()
        .zip(votes)
        .find(|(_, vote)| **vote == StrokeVote::Present(width))
        .and_then(|(node, _)| first_solid_stroke_hex(node).map(str::to_string))
}

#[cfg(test)]
#[path = "sibling_style_drift_tests.rs"]
mod tests;
