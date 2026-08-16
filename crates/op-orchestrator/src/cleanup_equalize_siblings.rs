//! Equalize scalar styling across a family of sibling items (DS P1-a).
//!
//! Measured 0814-08-14: a knowledge card emitted five sibling entries where
//! item 01 drifted from items 02-05 — different padding, a different title
//! font size — or the indent drifted item by item. This pass restores the
//! FAMILY norm, and only the scalar properties a majority can prove:
//! per-edge padding, gap, alignItems/justifyContent, and the fontSize /
//! fontWeight of text nodes at the same subtree position. Structure is never
//! touched: a member whose subtree kind-sequence differs from the family's
//! modal sequence (a deliberate hero first item, say) is skipped entirely —
//! restructure is an intent question, alignment is not.
//!
//! ## Why the predicate is narrow
//!
//! - The family must be >= 3 DIRECT children of one auto-layout parent, all
//!   frames. Two items agree by accident; three drift apart by accident.
//! - Entry criteria: (a) name stems equal after trailing digits are stripped
//!   ("Item 01" / "Item 02"), or (b) >= 2/3 of members share one
//!   descendant-bearing subtree kind-sequence. Only members sharing the MODAL
//!   kind-sequence are ever voted on or edited — a different-structure member
//!   can neither be aligned nor swing the vote.
//! - A value is the norm only when >= 2/3 of the editable members hold it.
//!   Three members with three paddings have no provable majority and are
//!   left alone; nothing is aligned toward a tie.
//!
//! ## Decorative-container fill alignment (DS P2-d ①)
//!
//! Measured 0815: after five sibling entries were made structurally
//! isomorphic, their number-chip FILL colours kept drifting per item
//! (black/black/red/red/pink) — same-family entries, same decorative
//! container slot, different colours, which no author intends. Beside the
//! same-position text votes, same-position Frame/Rectangle descendant nodes
//! vote on their PRIMARY fill through the same 2/3 mechanism:
//!
//! - Only a SOLID primary fill votes; a gradient / image / shader / mesh
//!   primary fill at any position makes that position unprovable and skips
//!   the WHOLE position (mirrors the gap rule: one decorative slot means the
//!   slot is not proven to be a colour slot at all).
//! - A `$variable` reference votes by the reference string — same reference
//!   is same value. A position mixing references and literals is never
//!   touched: the two systems cannot be proven synonymous.
//! - A missing fill is a votable missing value: it JOINS a concrete 2/3
//!   majority (the family norm is re-applied to the straggler), but a
//!   majority of "no fill" is never enforced — removing a fill is a
//!   destructive move this pass does not make.
//! - Text-node fills are never touched — that is the contrast pass's
//!   territory, and text colour is a legibility decision, not family drift.

use super::*;

use std::collections::BTreeMap;

use jian_ops_schema::node::container::{
    AlignItems, ContainerProps, JustifyContent, LayoutMode, Padding,
};
use jian_ops_schema::node::{FontWeight, NumberOrExpression, PenNode};
use jian_ops_schema::style::PenFill;
use op_design_lint::node_util::{children as node_children, is_node_visible, node_kind_str};
use op_editor_core::variables_resolve::is_variable_ref;
use serde_json::{json, Value};

/// Minimum sibling count for a family.
const MIN_MEMBERS: usize = 3;
/// A value must be held by at least this fraction of the members to define
/// the family norm (`count / n >= 2/3`).
const MAJORITY_NUMERATOR: usize = 2;
const MAJORITY_DENOMINATOR: usize = 3;

/// Per-root cleanup pass: align drifted scalar styling inside every detected
/// sibling family under `root_id`. Returns how many edits were applied.
pub(super) fn equalize_sibling_items(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = find_root(sink.state(), root_id) else {
        return 0;
    };
    let mut container_patches: Vec<(NodeId, String)> = Vec::new();
    let mut text_patches: Vec<(NodeId, String)> = Vec::new();
    let mut fill_patches: Vec<(NodeId, String)> = Vec::new();
    collect_family_plans(
        root,
        &mut container_patches,
        &mut text_patches,
        &mut fill_patches,
    );
    let applied = container_patches.len() + text_patches.len() + fill_patches.len();
    for (node_id, patch_json) in container_patches {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id: None,
        });
    }
    for (node_id, patch_json) in text_patches {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id: None,
        });
    }
    for (node_id, hex) in fill_patches {
        // `SetNodeFillHex` replaces the first solid in place (or prepends),
        // accepts `$variable` references, and validates literal hex — the
        // same fill writer the property panel uses.
        sink.apply(EditorCommand::SetNodeFillHex { node_id, hex });
    }
    applied
}

/// One editable family member: the frame plus the structure it shares.
struct Member<'a> {
    node: &'a PenNode,
    kind_seq: String,
    /// Text nodes in DFS pre-order; members with an equal `kind_seq` have the
    /// same count, so index `k` names "the same text position" across them.
    text_nodes: Vec<&'a PenNode>,
    /// Frame / Rectangle descendants in DFS pre-order (the member root
    /// itself excluded — the family members are the siblings, their
    /// decorative containers are the vote targets). Same alignment property
    /// as `text_nodes`: index `k` is the same container slot across members.
    fill_nodes: Vec<&'a PenNode>,
}

fn collect_family_plans(
    node: &PenNode,
    container_patches: &mut Vec<(NodeId, String)>,
    text_patches: &mut Vec<(NodeId, String)>,
    fill_patches: &mut Vec<(NodeId, String)>,
) {
    let Some(children) = node.children() else {
        return;
    };
    if let Some(family) = sibling_family(node, children) {
        plan_family_repairs(&family, container_patches, text_patches, fill_patches);
    }
    for child in children {
        collect_family_plans(child, container_patches, text_patches, fill_patches);
    }
}

/// Detect a sibling family under `parent` and return its editable members.
///
/// `None` unless EVERY gate holds: >= [`MIN_MEMBERS`] direct children, all of
/// them frames, an auto-layout parent, and either the name-stem criterion or
/// the 2/3 shared-structure criterion. The editable members are those sharing
/// the modal structure: a different-structure member (the hero) is neither
/// edited nor voted on. A name-only group of childless frames shares the
/// trivial "frame" sequence by construction, so all of its members are
/// structure-consistent.
fn sibling_family<'a>(parent: &PenNode, children: &'a [PenNode]) -> Option<Vec<Member<'a>>> {
    if children.len() < MIN_MEMBERS {
        return None;
    }
    if !children.iter().all(|c| matches!(c, PenNode::Frame(_))) {
        return None;
    }
    let layout = container_props(parent).and_then(|props| props.layout.as_ref());
    if !matches!(layout, Some(LayoutMode::Vertical | LayoutMode::Horizontal)) {
        return None;
    }
    let members: Vec<Member> = children
        .iter()
        .filter(|child| is_node_visible(child))
        .map(|child| Member {
            node: child,
            kind_seq: kind_sequence(child),
            text_nodes: collect_text_nodes(child),
            fill_nodes: collect_fill_nodes(child),
        })
        .collect();
    if members.len() < MIN_MEMBERS {
        return None;
    }

    let stems: Vec<&str> = members
        .iter()
        .map(|member| name_stem(member.node.base().name.as_deref().unwrap_or("")))
        .collect();
    let name_group = stems.iter().all(|stem| !stem.is_empty())
        && stems.windows(2).all(|pair| pair[0] == pair[1]);

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for member in &members {
        *counts.entry(member.kind_seq.as_str()).or_default() += 1;
    }
    // The modal kind-sequence. Only sequences with a descendant token count
    // as structure evidence: a childless frame's sequence is the trivial
    // "frame" shared by EVERY childless frame, so it cannot prove a family.
    let modal: Option<String> = counts
        .iter()
        .filter(|(seq, _)| seq.contains(' '))
        .max_by_key(|(_, count)| *count)
        .map(|(seq, _)| (*seq).to_string());
    let struct_group = modal.as_deref().is_some_and(|seq| {
        counts[seq] * MAJORITY_DENOMINATOR >= members.len() * MAJORITY_NUMERATOR
    });
    if !(name_group || struct_group) {
        return None;
    }

    let consistent: Vec<Member> = match &modal {
        Some(modal) => members
            .into_iter()
            .filter(|member| member.kind_seq == *modal)
            .collect(),
        // Reachable only via the name criterion: a family of childless
        // frames, which all share the same trivial sequence by construction.
        None => members,
    };
    // Two editable members can never form a 2/3 majority — nothing to align.
    if consistent.len() < MIN_MEMBERS {
        return None;
    }
    Some(consistent)
}

/// Plan the scalar repairs for one family and append `PatchNodeData` payloads.
///
/// Votes are taken ONLY over the structure-consistent members: a hero's
/// padding belongs to its own different design, so it can neither be edited
/// nor dilute the vote. Container props and text props are patched in
/// separate commands (different target nodes).
fn plan_family_repairs(
    family: &[Member],
    container_patches: &mut Vec<(NodeId, String)>,
    text_patches: &mut Vec<(NodeId, String)>,
    fill_patches: &mut Vec<(NodeId, String)>,
) {
    let n = family.len();
    // Per-member container-property changes, keyed by JSON field name.
    let mut member_patches: Vec<BTreeMap<&'static str, Value>> = vec![BTreeMap::new(); n];

    // padding, per edge.
    let edge_options: Vec<Option<[f64; 4]>> = family
        .iter()
        .map(|member| padding_edges(container_props(member.node).and_then(|p| p.padding.as_ref())))
        .collect();
    if edge_options.iter().all(Option::is_some) {
        let edges: Vec<[f64; 4]> = edge_options.iter().map(|edges| edges.unwrap()).collect();
        let mut corrected: Vec<Option<[f64; 4]>> = vec![None; n];
        for edge in 0..4 {
            let values: Vec<f64> = edges.iter().map(|edges| edges[edge]).collect();
            let Some(majority) = majority_of(&values) else {
                continue;
            };
            for (i, member_edges) in edges.iter().enumerate() {
                if member_edges[edge] != majority {
                    let target = corrected[i].get_or_insert(*member_edges);
                    target[edge] = majority;
                }
            }
        }
        for (i, edges) in corrected.iter().enumerate() {
            if let Some(edges) = edges {
                member_patches[i].insert("padding", padding_value(*edges));
            }
        }
    }

    // gap. A variable-bound gap on ANY member makes the vote unprovable and
    // skips gap alignment for the whole family.
    let gap_options: Vec<Option<Option<f64>>> = family
        .iter()
        .map(
            |member| match container_props(member.node).and_then(|p| p.gap.as_ref()) {
                None => Some(None),
                Some(NumberOrExpression::Number(v)) => Some(Some(*v)),
                Some(NumberOrExpression::Expression(_)) => None,
            },
        )
        .collect();
    if gap_options.iter().all(Option::is_some) {
        let gaps: Vec<Option<f64>> = gap_options.into_iter().map(Option::unwrap).collect();
        if let Some(majority) = majority_of(&gaps) {
            for (i, gap) in gaps.iter().enumerate() {
                if gap != &majority {
                    member_patches[i].insert("gap", majority.map_or(Value::Null, |v| json!(v)));
                }
            }
        }
    }

    // alignItems / justifyContent.
    let align_items: Vec<Option<AlignItems>> = family
        .iter()
        .map(|member| container_props(member.node).and_then(|p| p.align_items.clone()))
        .collect();
    if let Some(majority) = majority_of(&align_items) {
        for (i, value) in align_items.iter().enumerate() {
            if value != &majority {
                member_patches[i].insert(
                    "alignItems",
                    majority
                        .clone()
                        .map_or(Value::Null, |v| json!(align_items_json(v))),
                );
            }
        }
    }
    let justify_content: Vec<Option<JustifyContent>> = family
        .iter()
        .map(|member| container_props(member.node).and_then(|p| p.justify_content.clone()))
        .collect();
    if let Some(majority) = majority_of(&justify_content) {
        for (i, value) in justify_content.iter().enumerate() {
            if value != &majority {
                member_patches[i].insert(
                    "justifyContent",
                    majority
                        .clone()
                        .map_or(Value::Null, |v| json!(justify_content_json(v))),
                );
            }
        }
    }

    for (member, patch) in family.iter().zip(&member_patches) {
        if !patch.is_empty() {
            container_patches.push((
                NodeId::new(member.node.id_str()),
                serde_json::to_string(&Value::Object(
                    patch
                        .iter()
                        .map(|(k, v)| ((*k).to_string(), v.clone()))
                        .collect(),
                ))
                .unwrap_or_default(),
            ));
        }
    }

    // Same-position text nodes: fontSize / fontWeight per traversal index.
    // Equal kind-sequences guarantee every member has the same text-node
    // count, so position `k` means the same slot in every member.
    let positions = family.first().map_or(0, |m| m.text_nodes.len());
    // Per member, per text position: accumulated JSON changes.
    let mut text_changes: Vec<Vec<BTreeMap<&'static str, Value>>> =
        vec![vec![BTreeMap::new(); positions]; n];
    let Some(first_texts) = family.first().map(|member| &member.text_nodes) else {
        return;
    };
    for (position, _text) in first_texts.iter().enumerate() {
        let font_sizes: Vec<Option<f64>> = family
            .iter()
            .map(|member| match member.text_nodes.get(position) {
                Some(PenNode::Text(text)) => text.font_size,
                _ => None,
            })
            .collect();
        if let Some(majority) = majority_of(&font_sizes) {
            for (i, value) in font_sizes.iter().enumerate() {
                if value != &majority {
                    text_changes[i][position]
                        .insert("fontSize", majority.map_or(Value::Null, |v| json!(v)));
                }
            }
        }
        let font_weights: Vec<Option<FontWeight>> = family
            .iter()
            .map(|member| match member.text_nodes.get(position) {
                Some(PenNode::Text(text)) => text.font_weight.clone(),
                _ => None,
            })
            .collect();
        if let Some(majority) = majority_of(&font_weights) {
            for (i, value) in font_weights.iter().enumerate() {
                if value != &majority {
                    text_changes[i][position].insert(
                        "fontWeight",
                        majority.clone().map_or(Value::Null, font_weight_json),
                    );
                }
            }
        }
    }
    for (member, changes) in family.iter().zip(&text_changes) {
        for (position, change) in changes.iter().enumerate() {
            if change.is_empty() {
                continue;
            }
            let node = member.text_nodes[position];
            text_patches.push((
                NodeId::new(node.id_str()),
                serde_json::to_string(&Value::Object(
                    change
                        .iter()
                        .map(|(k, v)| ((*k).to_string(), v.clone()))
                        .collect(),
                ))
                .unwrap_or_default(),
            ));
        }
    }

    // Same-position Frame/Rectangle descendants: primary solid fill votes
    // (DS P2-d ①). Equal kind-sequences guarantee every member has the same
    // container-slot count, so position `k` means the same decorative slot
    // in every member — the number chip of a five-entry family, say.
    let Some(first_fills) = family.first().map(|member| &member.fill_nodes) else {
        return;
    };
    // Per member, per container-slot: the hex / `$ref` string to write.
    let mut fill_changes: Vec<Vec<Option<String>>> = vec![vec![None; first_fills.len()]; n];
    for (position, _slot_node) in first_fills.iter().enumerate() {
        let slots: Vec<FillSlot> = family
            .iter()
            .map(|member| fill_slot(member.fill_nodes[position]))
            .collect();
        // A non-solid primary fill (gradient / image / shader / mesh) on ANY
        // member proves nothing about the slot — decorative slots are not
        // provably colour slots, so the whole position is skipped (the same
        // unprovable-skip rule the gap vote uses).
        if slots.iter().any(|slot| matches!(slot, FillSlot::NonSolid)) {
            continue;
        }
        let votes: Vec<Option<FillVote>> = slots
            .iter()
            .map(|slot| match slot {
                FillSlot::Solid(vote) => Some(vote.clone()),
                FillSlot::Missing | FillSlot::NonSolid => None,
            })
            .collect();
        // Reference strings and literals cannot be proven synonymous — a
        // position mixing them is never touched.
        let has_ref = votes
            .iter()
            .any(|vote| matches!(vote, Some(FillVote::Ref(_))));
        let has_literal = votes
            .iter()
            .any(|vote| matches!(vote, Some(FillVote::Literal(_))));
        if has_ref && has_literal {
            continue;
        }
        // A CONCRETE 2/3 majority defines the norm; a majority of "no fill"
        // is never enforced (removing a fill is destructive and out of
        // scope). Members holding a different value — or none at all — join
        // the norm.
        let Some(Some(majority)) = majority_of(&votes) else {
            continue;
        };
        let hex = match &majority {
            FillVote::Literal(hex) | FillVote::Ref(hex) => hex.clone(),
        };
        for (i, vote) in votes.iter().enumerate() {
            if vote.as_ref() != Some(&majority) {
                fill_changes[i][position] = Some(hex.clone());
            }
        }
    }
    for (member, changes) in family.iter().zip(&fill_changes) {
        for (position, hex) in changes.iter().enumerate() {
            if let Some(hex) = hex {
                let node = member.fill_nodes[position];
                fill_patches.push((NodeId::new(node.id_str()), hex.clone()));
            }
        }
    }
}

/// The value held by the most members, when it clears the 2/3 bar.
fn majority_of<T: PartialEq + Clone>(values: &[T]) -> Option<T> {
    if values.is_empty() {
        return None;
    }
    let n = values.len();
    for candidate in values {
        let count = values.iter().filter(|value| *value == candidate).count();
        if count * MAJORITY_DENOMINATOR >= n * MAJORITY_NUMERATOR {
            return Some(candidate.clone());
        }
    }
    None
}

/// Container props shared by Frame / Group / Rectangle.
fn container_props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(frame) => Some(&frame.container),
        PenNode::Group(group) => Some(&group.container),
        PenNode::Rectangle(rect) => Some(&rect.container),
        _ => None,
    }
}

/// The name with trailing digits (and the whitespace before them) stripped:
/// "Item 01" → "Item", "Item02" → "Item". A bare numeric name strips to "".
fn name_stem(name: &str) -> &str {
    let trimmed = name.trim_end();
    trimmed
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .trim_end()
}

/// DFS pre-order sequence of node-kind strings ("frame text frame image").
/// Two members with the same sequence have the same node types in the same
/// traversal positions.
fn kind_sequence(node: &PenNode) -> String {
    let mut sequence = String::new();
    push_kind_sequence(node, &mut sequence);
    sequence
}

fn push_kind_sequence(node: &PenNode, sequence: &mut String) {
    if !sequence.is_empty() {
        sequence.push(' ');
    }
    sequence.push_str(node_kind_str(node));
    for child in node_children(node) {
        push_kind_sequence(child, sequence);
    }
}

/// Text nodes in DFS pre-order.
fn collect_text_nodes(node: &PenNode) -> Vec<&PenNode> {
    let mut found = Vec::new();
    push_text_nodes(node, &mut found);
    found
}

fn push_text_nodes<'a>(node: &'a PenNode, found: &mut Vec<&'a PenNode>) {
    if matches!(node, PenNode::Text(_)) {
        found.push(node);
    }
    for child in node_children(node) {
        push_text_nodes(child, found);
    }
}

/// Frame / Rectangle nodes in DFS pre-order (the root itself excluded).
fn collect_fill_nodes(node: &PenNode) -> Vec<&PenNode> {
    let mut found = Vec::new();
    for child in node_children(node) {
        push_fill_nodes(child, &mut found);
    }
    found
}

fn push_fill_nodes<'a>(node: &'a PenNode, found: &mut Vec<&'a PenNode>) {
    if matches!(node, PenNode::Frame(_) | PenNode::Rectangle(_)) {
        found.push(node);
    }
    for child in node_children(node) {
        push_fill_nodes(child, found);
    }
}

/// What a same-position Frame/Rectangle's PRIMARY fill holds, for the vote.
#[derive(Debug, Clone, PartialEq)]
enum FillVote {
    /// A `#rrggbb` literal — comparable and patchable as itself.
    Literal(String),
    /// A `$variable` reference string — votes by the reference itself (same
    /// reference is same value).
    Ref(String),
}

/// The votability of one container slot's primary fill.
#[derive(Debug)]
enum FillSlot {
    /// The primary fill is NOT solid (gradient / image / shader / mesh) —
    /// the slot is decorative, not a provable colour slot.
    NonSolid,
    /// No fill, an empty fill list, or no solid entry — a missing value that
    /// can join a concrete majority, but never define one.
    Missing,
    /// A solid literal or `$variable` reference.
    Solid(FillVote),
}

/// Classify the PRIMARY (first) fill of a Frame/Rectangle node. Only the
/// first entry decides: a solid layered under a gradient is a fallback, not
/// the slot's colour, and later entries never rescue an unprovable slot.
fn fill_slot(node: &PenNode) -> FillSlot {
    let Some(fills) = op_editor_core::fills::node_fills(node) else {
        return FillSlot::Missing;
    };
    match fills.first() {
        None => FillSlot::Missing,
        Some(PenFill::Solid(body)) => {
            let color = body.color.as_str();
            if is_variable_ref(color) {
                FillSlot::Solid(FillVote::Ref(color.to_string()))
            } else {
                FillSlot::Solid(FillVote::Literal(color.to_string()))
            }
        }
        Some(_) => FillSlot::NonSolid,
    }
}

/// `[top, right, bottom, left]` of a padding, or `None` when the padding is a
/// variable expression (unprovable). Missing padding reads as zero.
pub(super) fn padding_edges(padding: Option<&Padding>) -> Option<[f64; 4]> {
    match padding {
        None => Some([0.0; 4]),
        Some(Padding::Uniform(v)) => Some([*v; 4]),
        Some(Padding::XY([y, x])) => Some([*y, *x, *y, *x]),
        Some(Padding::LtrB(edges)) => Some(*edges),
        Some(Padding::Expression(_)) => None,
    }
}

/// Serialize `[top, right, bottom, left]` back to the compact padding shape
/// the schema accepts: uniform when all edges agree, `[y, x]` when vertical
/// and horizontal pairs agree, else `[top, right, bottom, left]`.
pub(super) fn padding_value([top, right, bottom, left]: [f64; 4]) -> Value {
    if top == right && right == bottom && bottom == left {
        json!(top)
    } else if top == bottom && right == left {
        json!([top, right])
    } else {
        json!([top, right, bottom, left])
    }
}

fn align_items_json(value: AlignItems) -> &'static str {
    match value {
        AlignItems::Start => "start",
        AlignItems::Center => "center",
        AlignItems::End => "end",
        AlignItems::Stretch => "stretch",
    }
}

fn justify_content_json(value: JustifyContent) -> &'static str {
    match value {
        JustifyContent::Start => "start",
        JustifyContent::Center => "center",
        JustifyContent::End => "end",
        JustifyContent::SpaceBetween => "space_between",
        JustifyContent::SpaceAround => "space_around",
    }
}

fn font_weight_json(value: FontWeight) -> Value {
    match value {
        FontWeight::Number(weight) => json!(weight),
        FontWeight::Keyword(keyword) => json!(keyword),
    }
}

#[cfg(test)]
#[path = "cleanup_equalize_siblings_tests.rs"]
mod tests;
