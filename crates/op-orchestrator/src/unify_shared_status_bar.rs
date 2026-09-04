//! Deterministic cross-screen status-bar chrome unification — [C from the
//! 0718-1-k3-1 postmortem] fixes screens missing the pinned status bar
//! entirely (measured: two of three screens had no status-bar subtree at
//! all while the third carried one) by cloning a reference screen's status
//! bar onto every OTHER mobile screen-shaped root that lacks one.
//!
//! Sibling pass to [`crate::unify_shared_nav`] — same "reuse, don't redraw"
//! shape — run immediately after it in `cleanup::run_cleanup_passes`, so
//! both the classic and loop-finalize paths pick it up through the one
//! shared choke point. Status bars and bottom-nav are both shared chrome
//! every screen should carry identically; splitting them into two passes
//! (rather than one pass unifying "all chrome") keeps each pass's
//! reference-detection + injection logic independently simple and
//! testable — status bars key off [`crate::cleanup::is_status_bar`]
//! (role/name/id aliases), bottom-nav keys off `collect_nav_containers`
//! (role + tab-item structure), and the two shapes never overlap.
//!
//! ## Screen-shaped root gate
//!
//! Reuses [`crate::unfilled_screens::list_screen_candidates`]'s exact
//! mobile-artboard shape check for numeric-height roots. This pass adds one
//! deliberately local extension: a 300–480px-wide top-level Frame whose
//! authored height is `fit_content` also qualifies when the real jian layout
//! resolves it to at least 480px tall. Generated mobile roots are intentionally
//! finalized as `fit_content`, so requiring a literal height here skipped the
//! exact screens whose shared chrome this pass owns. The global unfilled-screen
//! contract remains unchanged.
//!
//! ## Never fabricates chrome from nothing
//!
//! If NO screen in the document carries a status bar, the whole pass is a
//! no-op — there's no reference to clone, and this pass never invents a
//! status bar that no screen in the document ever had.
//!
//! ## Injected clones must carry `role: "status-bar"` (0718-1-k3-1 review fix)
//!
//! Reference detection here uses role/name/id aliases ([`is_status_bar`]), but
//! `unfilled_screens.rs`'s chrome exclusion (`CHROME_ROLES`) is role-based —
//! an authored status bar frequently has no `role` at all. Left unstamped,
//! its own text children (time/battery) would read as "real content" to the
//! promise-delivery check. Both injected clones and existing direct status
//! bars are therefore stamped when their role is not canonical. Existing bars are
//! otherwise preserved and, when appended after content, moved in-place to
//! child index 0 rather than redrawn.
//!
//! ## Roadmap: geometry-based reference detection (recorded, not this batch)
//!
//! Reference-screen detection here is a structural/name check
//! ([`is_status_bar`]), not the resolved-layout geometric judgment
//! `op-host-native/src/preview/present.rs::pinned_status_bar_candidate`
//! uses (flush-to-top, ≥90% root width, ≤60px tall, against REAL post-
//! layout `SceneNode` bounds). The clone comes from an already-authored
//! sibling screen's real status bar, so it almost certainly satisfies that
//! same shape too — a mismatch would need the target root's own padding to
//! push the clone's resolved top offset outside the pin tolerance. Revisit
//! with a `op_pen_loader::editor_state_to_layout_scene`-based check (see
//! `wire_screen_navigation.rs`'s `resolved_y_offsets` for the same
//! infrastructure already proven inside this crate) if a real case turns
//! up where an injected status bar doesn't actually pin in preview.

use std::collections::HashSet;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};

use crate::cleanup::is_status_bar;
use crate::types::DocSink;
use crate::unfilled_screens::list_screen_candidates;

const MIN_SCREEN_WIDTH: f64 = 300.0;
const MAX_SCREEN_WIDTH: f64 = 480.0;
const MIN_SCREEN_HEIGHT: f64 = 480.0;

/// Entry point. No-ops when fewer than 2 mobile screen-shaped roots exist,
/// or when no screen carries a status bar at all.
pub fn unify_shared_status_bar(sink: &mut dyn DocSink) {
    let screens = list_status_bar_screen_candidates(sink.state());
    if screens.len() < 2 {
        return;
    }

    let Some(reference_bar) = find_reference_status_bar(sink, &screens) else {
        return;
    };

    for screen_id in &screens {
        let existing = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(screen_id.clone()),
        )
        .and_then(direct_status_bar_child)
        .map(|(index, bar)| ExistingStatusBar {
            node_id: bar.id_str().to_string(),
            index,
            role_needs_status_bar: bar.base().role.as_deref() != Some("status-bar"),
        });

        if let Some(existing) = existing {
            if existing.role_needs_status_bar {
                sink.apply(EditorCommand::PatchNodeData {
                    node_id: NodeId::new(existing.node_id.clone()),
                    patch_json: r#"{"role":"status-bar"}"#.to_string(),
                    page_id: None,
                });
            }
            if existing.index > 0 {
                sink.apply(EditorCommand::MoveNode {
                    node_id: NodeId::new(existing.node_id),
                    target_parent: NodeId::new(screen_id.clone()),
                    page_id: None,
                    index: Some(0),
                });
            }
            continue;
        }

        let mut clone = reference_bar.clone();
        stamp_chrome_role(&mut clone);

        // `InsertSubtree` APPENDS to the target parent's children, so the
        // clone lands as the LAST child first — reposition it to the FIRST
        // slot afterward so it pins to the top like every authored status
        // bar does. Mirrors `cleanup_mobile_chrome::anchor_bottom_nav_last`'s
        // own "move within the same parent" idiom for the opposite end.
        sink.apply(EditorCommand::InsertSubtree {
            nodes: vec![clone],
            parent_id: NodeId::new(screen_id.clone()),
            page_id: None,
        });
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(screen_id.clone()),
        ) else {
            continue;
        };
        let Some(new_child) = root.children().and_then(|c| c.last()) else {
            continue;
        };
        let new_id = NodeId::new(new_child.id_str().to_string());
        sink.apply(EditorCommand::MoveNode {
            node_id: new_id,
            target_parent: NodeId::new(screen_id.clone()),
            page_id: None,
            index: Some(0),
        });
    }
}

#[derive(Debug)]
struct ExistingStatusBar {
    node_id: String,
    index: usize,
    role_needs_status_bar: bool,
}

/// Status-bar-specific screen candidates. Numeric-height roots keep using the
/// shared unfilled-screen gate. Only `fit_content` Frame roots get the local
/// resolved-height fallback needed by generated mobile screens.
fn list_status_bar_screen_candidates(state: &EditorState) -> Vec<String> {
    let numeric_ids: HashSet<String> = list_screen_candidates(state)
        .into_iter()
        .map(|screen| screen.node_id)
        .collect();
    let fit_content_ids: HashSet<&str> = state
        .active_children()
        .iter()
        .filter(|node| looks_like_fit_content_mobile_root(node))
        .map(PenNodeExt::id_str)
        .collect();

    if fit_content_ids.is_empty() {
        return state
            .active_children()
            .iter()
            .filter(|node| numeric_ids.contains(node.id_str()))
            .map(|node| node.id_str().to_string())
            .collect();
    }

    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let resolved_fit_content_ids: HashSet<&str> = scene
        .active_page()
        .map(|page| {
            page.children
                .iter()
                .filter(|node| fit_content_ids.contains(node.id.as_str()))
                .filter(|node| {
                    let height = f64::from(node.bounds.size.y);
                    height.is_finite() && height >= MIN_SCREEN_HEIGHT
                })
                .map(|node| node.id.as_str())
                .collect()
        })
        .unwrap_or_default();

    state
        .active_children()
        .iter()
        .filter(|node| {
            numeric_ids.contains(node.id_str()) || resolved_fit_content_ids.contains(node.id_str())
        })
        .map(|node| node.id_str().to_string())
        .collect()
}

pub(crate) fn looks_like_fit_content_mobile_root(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    let Some(width) = node.width_px() else {
        return false;
    };
    (MIN_SCREEN_WIDTH..=MAX_SCREEN_WIDTH).contains(&width)
        && matches!(
            frame.container.height.as_ref(),
            Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
        )
}

/// Find the document-order first screen with a direct status-bar child,
/// returning a CLONE of that status bar (owned, so the caller can drop the
/// `sink.state()` borrow before mutating).
fn find_reference_status_bar(sink: &dyn DocSink, screens: &[String]) -> Option<PenNode> {
    for screen_id in screens {
        let root = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(screen_id.clone()),
        )?;
        if let Some((_, bar)) = direct_status_bar_child(root) {
            return Some(bar.clone());
        }
    }
    None
}

/// The first DIRECT child of `root` that reads as a status bar (name/id
/// substring match — see [`is_status_bar`]). Direct-child only, matching
/// `cleanup::remove_duplicate_status_bars`'s own scan depth: a status bar
/// is authored as a top-level sibling of the screen's content, not nested.
fn direct_status_bar_child(root: &PenNode) -> Option<(usize, &PenNode)> {
    root.children()?
        .iter()
        .enumerate()
        .find(|(_, child)| is_status_bar(child))
}

/// Force `role: "status-bar"` onto `node` if it doesn't already carry that
/// canonical role — see the module doc's role-stamping section. Existing nodes are
/// patched through `EditorCommand`; this helper only handles owned clones.
fn stamp_chrome_role(node: &mut PenNode) {
    if node.base().role.as_deref() != Some("status-bar") {
        node.base_mut().role = Some("status-bar".to_string());
    }
}

#[cfg(test)]
#[path = "unify_shared_status_bar_tests.rs"]
mod tests;
