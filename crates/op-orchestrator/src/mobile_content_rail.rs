//! Deterministic mobile content-rail ownership.
//!
//! Generated mobile screens commonly mix full-width chrome with individually
//! padded content sections. A root-level gutter cannot represent that shape:
//! it would inset status/navigation chrome and destroy intentional horizontal
//! scrollers. This pass repairs transparent root-direct content sections,
//! wraps unambiguously edge-spanning rounded/stroked cards in a transparent
//! rail owner, and gives clipped horizontal scrollers a leading rail while
//! keeping their trailing edge flush.

use crate::types::DocSink;
use jian_ops_schema::node::{container::LayoutMode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorCommand, LayoutPropValue, NodeId, PenNodeExt};
use std::collections::HashSet;

const DEFAULT_MOBILE_RAIL: f64 = 24.0;
const MIN_MOBILE_WIDTH: f64 = 320.0;
const MAX_MOBILE_WIDTH: f64 = 480.0;
const MIN_CONTENT_RAIL: f64 = 16.0;
const MAX_CONTENT_RAIL: f64 = 28.0;
const MAX_HEADER_NEIGHBOR_SIBLINGS: usize = 4;

pub(crate) fn repair_mobile_content_rails_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        repair_mobile_content_rails(sink, &root_id);
    }
}

pub(crate) fn repair_mobile_content_rails(sink: &mut dyn DocSink, root_id: &str) {
    // A two-script fresh generation can append a second, byte-equivalent
    // page-content shell under the same mobile page. Merge that narrow shape
    // before rail/header collection so every later repair sees one coherent
    // ownership tree. The merge itself is one atomic editor command.
    merge_duplicate_page_shells(sink, root_id);

    // `root_id` is a top-level active root in the common case (fresh-document
    // generation), but the classic selected-frame append path nests the
    // inserted subtree root under an existing top-level mobile screen instead
    // of placing it at the top level (the same "Component 11c" shape
    // `cleanup.rs::find_root` already works around). The appended fragment
    // itself is usually just a section — it will not pass
    // `looks_like_mobile_screen` on its own — so on a miss we walk up to the
    // enclosing top-level screen to use as detection CONTEXT, but keep
    // mutations scoped to `root_id`'s own subtree so pre-existing content
    // elsewhere in that screen is never touched.
    let (repairs, apply_root_id) = {
        let roots = sink.state().active_children();
        if let Some(root) = roots.iter().find(|node| node.id_str() == root_id) {
            (collect_repairs(root), root_id.to_string())
        } else if let Some((screen, scope)) = find_enclosing_top_level(roots, root_id) {
            let repairs = collect_repairs(screen)
                .into_iter()
                .filter(|repair| repair_touches_scope(repair, &scope))
                .collect();
            (repairs, screen.id_str().to_string())
        } else {
            return;
        }
    };
    // Scope filtering above is per-repair, but some repairs are only correct
    // as a PAIR (a scroller's own leading-rail add + its inner lane's
    // matching leading-rail clear): if the append's scope contains only the
    // inner lane, filtering can keep the "clear" half and drop the "add"
    // half, net-erasing the rail entirely. Drop any repair whose declared
    // partner did not survive scoping.
    let repairs = drop_unpaired_repairs(repairs);

    for repair in repairs {
        match repair {
            RailRepair::SetPadding {
                node_id, padding, ..
            } => {
                sink.apply(EditorCommand::SetNodeLayoutProp {
                    node_id: NodeId::new(node_id),
                    property: "padding".to_string(),
                    value: LayoutPropValue::NumberArray(padding),
                });
            }
            RailRepair::WrapSurface {
                surface_id,
                wrapper,
                original_index,
            } => {
                let wrapper_id = NodeId::new(wrapper.id_str().to_string());
                if !sink.apply(EditorCommand::InsertAuthoredSubtree {
                    nodes: vec![*wrapper],
                    parent_id: NodeId::new(apply_root_id.clone()),
                    page_id: None,
                }) {
                    continue;
                }
                if !sink.apply(EditorCommand::MoveNode {
                    node_id: NodeId::new(surface_id),
                    target_parent: wrapper_id.clone(),
                    page_id: None,
                    index: None,
                }) {
                    sink.apply(EditorCommand::DeleteNode {
                        node_id: wrapper_id,
                        page_id: None,
                    });
                    continue;
                }
                sink.apply(EditorCommand::MoveNode {
                    node_id: wrapper_id,
                    target_parent: NodeId::new(apply_root_id.clone()),
                    page_id: None,
                    index: Some(original_index),
                });
            }
            RailRepair::WrapLooseNode {
                node_id,
                wrapper,
                original_index,
            } => {
                let wrapper_id = NodeId::new(wrapper.id_str().to_string());
                if !sink.apply(EditorCommand::InsertAuthoredSubtree {
                    nodes: vec![*wrapper],
                    parent_id: NodeId::new(apply_root_id.clone()),
                    page_id: None,
                }) {
                    continue;
                }
                if !sink.apply(EditorCommand::MoveNode {
                    node_id: NodeId::new(node_id),
                    target_parent: wrapper_id.clone(),
                    page_id: None,
                    index: None,
                }) {
                    sink.apply(EditorCommand::DeleteNode {
                        node_id: wrapper_id,
                        page_id: None,
                    });
                    continue;
                }
                sink.apply(EditorCommand::MoveNode {
                    node_id: wrapper_id,
                    target_parent: NodeId::new(apply_root_id.clone()),
                    page_id: None,
                    index: Some(original_index),
                });
            }
            RailRepair::MoveIntoHeader {
                header_id,
                children,
            } => {
                let mut moved = Vec::new();
                for (node_id, original_index) in children {
                    if sink.apply(EditorCommand::MoveNode {
                        node_id: NodeId::new(node_id.clone()),
                        target_parent: NodeId::new(header_id.clone()),
                        page_id: None,
                        index: None,
                    }) {
                        moved.push((node_id, original_index));
                        continue;
                    }

                    // The repair is an atomic ownership decision: if one of
                    // the validated siblings cannot move, put the earlier
                    // siblings back instead of leaving a half-filled header.
                    for (moved_id, moved_index) in moved.into_iter().rev() {
                        sink.apply(EditorCommand::MoveNode {
                            node_id: NodeId::new(moved_id),
                            target_parent: NodeId::new(apply_root_id.clone()),
                            page_id: None,
                            index: Some(moved_index),
                        });
                    }
                    break;
                }
            }
        }
    }
}

// Sibling modules: this file keeps the public repair surface
// (`repair_mobile_content_rails*`), the `RailRepair` plan and its collection
// walk; the duplicate page-shell merge, the header-adoption heuristics, and
// the shared node predicates live in their own files and are re-imported here
// so the walk sees the same flat namespace as before.
#[path = "mobile_content_rail_detect.rs"]
mod mobile_content_rail_detect;
#[path = "mobile_content_rail_header.rs"]
mod mobile_content_rail_header;
#[path = "mobile_content_rail_shell_merge.rs"]
mod mobile_content_rail_shell_merge;

use mobile_content_rail_detect::*;
use mobile_content_rail_header::*;
use mobile_content_rail_shell_merge::*;

/// Finds the top-level active root whose subtree contains `target_id`,
/// returning that root together with the full id set of `target_id`'s own
/// subtree (the mutation-scope allowlist).
fn find_enclosing_top_level<'a>(
    roots: &'a [PenNode],
    target_id: &str,
) -> Option<(&'a PenNode, HashSet<String>)> {
    roots
        .iter()
        .find_map(|root| subtree_ids_rooted_at(root, target_id).map(|scope| (root, scope)))
}

/// Returns the id set of `target_id`'s own subtree (including itself) if
/// `target_id` appears anywhere under `node`, else `None`.
fn subtree_ids_rooted_at(node: &PenNode, target_id: &str) -> Option<HashSet<String>> {
    if node.id_str() == target_id {
        let mut ids = HashSet::new();
        collect_node_ids(node, &mut ids);
        return Some(ids);
    }
    node.children()
        .into_iter()
        .flatten()
        .find_map(|child| subtree_ids_rooted_at(child, target_id))
}

/// True when a repair's mutation target lies inside the scoped subtree.
fn repair_touches_scope(repair: &RailRepair, scope: &HashSet<String>) -> bool {
    match repair {
        RailRepair::SetPadding { node_id, .. } => scope.contains(node_id),
        RailRepair::WrapSurface { surface_id, .. } => scope.contains(surface_id),
        RailRepair::WrapLooseNode { node_id, .. } => scope.contains(node_id),
        RailRepair::MoveIntoHeader {
            header_id,
            children,
        } => {
            scope.contains(header_id) && children.iter().all(|(node_id, _)| scope.contains(node_id))
        }
    }
}

/// Drops any `SetPadding` repair whose `requires` partner (the other half of
/// an atomic outer-add/inner-clear pair, see `collect_scroller_padding_repairs`)
/// is not itself present in `repairs`. A no-op when `repairs` is unfiltered
/// (both halves of every pair are always present together), so this is safe
/// to run unconditionally.
fn drop_unpaired_repairs(repairs: Vec<RailRepair>) -> Vec<RailRepair> {
    let present: HashSet<String> = repairs
        .iter()
        .filter_map(|repair| match repair {
            RailRepair::SetPadding { node_id, .. } => Some(node_id.clone()),
            RailRepair::WrapSurface { .. }
            | RailRepair::WrapLooseNode { .. }
            | RailRepair::MoveIntoHeader { .. } => None,
        })
        .collect();
    repairs
        .into_iter()
        .filter(|repair| match repair {
            RailRepair::SetPadding {
                requires: Some(req),
                ..
            } => present.contains(req),
            _ => true,
        })
        .collect()
}

#[derive(Debug)]
enum RailRepair {
    SetPadding {
        node_id: String,
        padding: Vec<f64>,
        /// When `Some(id)`, this repair is only meaningful if the repair
        /// targeting `id` is ALSO present in the final (post-scope-filter)
        /// list — see `collect_scroller_padding_repairs`'s outer-add /
        /// inner-clear pair.
        requires: Option<String>,
    },
    WrapSurface {
        surface_id: String,
        wrapper: Box<PenNode>,
        original_index: usize,
    },
    WrapLooseNode {
        node_id: String,
        wrapper: Box<PenNode>,
        original_index: usize,
    },
    MoveIntoHeader {
        header_id: String,
        children: Vec<(String, usize)>,
    },
}

fn collect_repairs(root: &PenNode) -> Vec<RailRepair> {
    if !looks_like_mobile_screen(root)
        || has_expression_padding(root)
        || horizontal_padding(root).is_some_and(nonzero_pair)
    {
        return Vec::new();
    }
    let Some(sections) = root.children() else {
        return Vec::new();
    };
    let root_width = root.width_px().unwrap_or_default();
    let rail = infer_content_rail(sections);
    let mut repairs = Vec::new();
    let mut known_ids = HashSet::new();
    collect_node_ids(root, &mut known_ids);
    let header_adoption = collect_header_adoption(sections);
    let adopted_ids: HashSet<&str> = header_adoption
        .as_ref()
        .into_iter()
        .flat_map(|adoption| {
            adoption
                .children
                .iter()
                .map(|(node_id, _)| node_id.as_str())
        })
        .collect();

    for (index, section) in sections.iter().enumerate() {
        if adopted_ids.contains(section.id_str()) {
            continue;
        }

        if is_ordinary_root_leaf(section) {
            let wrapper_id = unique_wrapper_id(section.id_str(), &mut known_ids);
            repairs.push(RailRepair::WrapLooseNode {
                node_id: section.id_str().to_string(),
                wrapper: Box::new(content_rail_wrapper(
                    &wrapper_id,
                    section,
                    DEFAULT_MOBILE_RAIL,
                )),
                original_index: index,
            });
            continue;
        }

        if !section.is_container()
            || is_mobile_chrome(section)
            || is_intentional_full_bleed_role(section)
            || is_empty_header_shell(section)
            || is_compact_header_action(section)
            || has_expression_padding(section)
        {
            continue;
        }

        // A root-direct horizontal viewport owns its own asymmetric rail,
        // including image-only carousels that have no text/icon descendants.
        if is_clipped_horizontal_scroller(section) {
            collect_scroller_padding_repairs(section, rail, &mut repairs);
            continue;
        }

        // A root-direct surfaced card needs an OUTER rail owner; padding the
        // card itself would inset only its contents while leaving the border
        // glued to the viewport. Prefer this structural repair even when the
        // card contains a full-width cover image. Authored absolute-positioned
        // surfaces are deliberately excluded by the predicate below.
        if !is_transparent_surface(section) {
            if has_surface_content_descendant(section)
                && is_edge_spanning_insettable_surface(section, root_width)
            {
                let wrapper_id = unique_wrapper_id(section.id_str(), &mut known_ids);
                repairs.push(RailRepair::WrapSurface {
                    surface_id: section.id_str().to_string(),
                    wrapper: Box::new(content_rail_wrapper(&wrapper_id, section, rail)),
                    original_index: index,
                });
            }
            continue;
        }

        let scrollers: Vec<&PenNode> = section
            .children()
            .into_iter()
            .flatten()
            .filter(|child| is_clipped_horizontal_scroller(child))
            .collect();
        if !scrollers.is_empty() {
            // A horizontal rail needs a flush trailing edge so the last card
            // can scroll offscreen. Keep the section full-width, inset its
            // short header siblings, and add only a leading inset to each
            // viewport.
            for child in section.children().into_iter().flatten() {
                if is_clipped_horizontal_scroller(child) {
                    collect_scroller_padding_repairs(child, rail, &mut repairs);
                } else if is_scroller_header(child)
                    && !has_expression_padding(child)
                    && horizontal_padding(child).is_none_or(|pair| !nonzero_pair(pair))
                {
                    repairs.push(RailRepair::SetPadding {
                        node_id: child.id_str().to_string(),
                        padding: padding_with_horizontal_rail(child, rail),
                        requires: None,
                    });
                }
            }
            continue;
        }

        // A deeper scroller has no unambiguous direct header/viewport owner.
        // Leave it untouched rather than putting a symmetric rail on an
        // ancestor and silently closing its intentional trailing edge.
        if contains_clipped_horizontal_scroller(section) {
            continue;
        }

        // Transparent media-overlay owners are intentional full bleed. This
        // must stay aligned with the design-lint detector so pre-validation
        // cannot undo the cleanup decision on the next pipeline stage.
        if has_full_bleed_media_child(section, root_width) || !has_text_or_icon_descendant(section)
        {
            continue;
        }

        if horizontal_padding(section).is_none_or(|pair| !nonzero_pair(pair)) {
            repairs.push(RailRepair::SetPadding {
                node_id: section.id_str().to_string(),
                padding: padding_with_horizontal_rail(section, rail),
                requires: None,
            });
        }
    }

    // Structural wrapping above replaces nodes one-for-one at their original
    // indices. Reparent header children last so those precomputed indices stay
    // stable while wrappers are inserted and moved into place.
    if let Some(adoption) = header_adoption {
        repairs.push(RailRepair::MoveIntoHeader {
            header_id: adoption.header_id,
            children: adoption.children,
        });
    }

    repairs
}

fn collect_scroller_padding_repairs(scroller: &PenNode, rail: f64, repairs: &mut Vec<RailRepair>) {
    if has_expression_padding(scroller) {
        return;
    }

    let current_padding = horizontal_padding(scroller);
    let needs_leading_rail = current_padding.is_none_or(|(left, _)| left <= 0.0);

    if needs_leading_rail {
        repairs.push(RailRepair::SetPadding {
            node_id: scroller.id_str().to_string(),
            padding: padding_with_leading_rail(scroller, rail),
            requires: None,
        });
    }

    if let Some((node_id, padding, inner_left)) = redundant_inner_scroller_rail_repair(scroller) {
        let duplicates_existing_rail =
            current_padding.is_some_and(|(outer_left, _)| (outer_left - inner_left).abs() <= 0.5);
        if needs_leading_rail || duplicates_existing_rail {
            // The inner lane's rail is only redundant once the OUTER
            // scroller actually owns a leading rail. When this push is
            // triggered by `needs_leading_rail`, the outer's own add above
            // is what establishes that ownership — if scope filtering drops
            // the outer add (e.g. an append whose inserted root is this
            // inner lane itself, not the outer viewport), clearing the inner
            // lane alone would erase the rail entirely. `duplicates_existing_rail`
            // needs no such pairing: the outer already owns a rail we are not
            // touching, so the inner clear stands on its own.
            let requires = needs_leading_rail.then(|| scroller.id_str().to_string());
            repairs.push(RailRepair::SetPadding {
                node_id,
                padding,
                requires,
            });
        }
    }
}

fn redundant_inner_scroller_rail_repair(scroller: &PenNode) -> Option<(String, Vec<f64>, f64)> {
    let children = scroller.children()?;
    let [inner] = children.as_slice() else {
        return None;
    };
    let props = container_props(inner)?;
    if props.layout != Some(LayoutMode::Horizontal)
        || props.clip_content == Some(true)
        || !matches!(
            props.width.as_ref(),
            Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
        )
        || !is_transparent_surface(inner)
        || has_expression_padding(inner)
        || inner.children().map_or(0, |children| children.len()) < 2
    {
        return None;
    }

    let (left, right) = horizontal_padding(inner)?;
    if !(MIN_CONTENT_RAIL..=MAX_CONTENT_RAIL).contains(&left) {
        return None;
    }
    let (top, bottom) = vertical_padding(inner);
    Some((
        inner.id_str().to_string(),
        vec![top, right, bottom, 0.0],
        left,
    ))
}

// Single-sourced in op-editor-core (same walk as the reveal bookkeeping).
use op_editor_core::agent_reveals::collect_node_ids;

fn unique_wrapper_id(surface_id: &str, known_ids: &mut HashSet<String>) -> String {
    let base = format!("{surface_id}__content_rail");
    let mut candidate = base.clone();
    let mut suffix = 2usize;
    while known_ids.contains(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    known_ids.insert(candidate.clone());
    candidate
}

fn content_rail_wrapper(id: &str, surface: &PenNode, rail: f64) -> PenNode {
    let name = surface.base().name.as_deref().unwrap_or("Content");
    serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": id,
        "name": format!("{name} Content Rail"),
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "padding": [0, rail, 0, rail],
        "children": []
    }))
    .expect("content rail wrapper is valid PenNode")
}
