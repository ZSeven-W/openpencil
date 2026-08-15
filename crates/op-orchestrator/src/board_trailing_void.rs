//! Resolved trailing-void measurement for fixed boards, shared by the card
//! centring repair (DS P2-b item B) and the `finalize_design` advisory scan
//! (DS P2-b item C, riding the P2-a advisories channel).
//!
//! "Trailing void" is the empty strip between the lowest resolved CONTENT
//! bottom edge and the board's bottom edge, as a fraction of the board
//! height. It is read from the REAL jian layout, re-parsed from the current
//! state at call time (the convention every geometric pass follows), with
//! the P1.5 content discipline:
//!
//! - Text always counts as content; an in-flow non-text node counts too.
//! - A bounded node occupies its full authored box even when its children
//!   hug inside it; an unbounded (hug) container owns no box of its own, so
//!   its in-flow children decide — recursing also keeps overlay children OUT
//!   of an unbounded container's aggregate rect.
//! - An authored-position overlay (explicit x/y) and a full-bleed layer
//!   (resolved size == board size) are decoration: ignored, so a badge
//!   pinned to the bottom cannot mask the void and neither can a background.

use jian_ops_schema::node::PenNode;
use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorState, NodeId, PenNodeExt};

/// Trailing-void fraction of the board height at/above which the centre
/// repair (item B) fires: the board is provably top-stacked.
pub(crate) const CARD_VOID_CENTRE_FLOOR: f64 = 0.20;
/// Trailing-void fraction that still reads as sparse AFTER centring — which
/// can at best halve the void — and therefore triggers the advisory (item C).
pub(crate) const BOARD_VOID_ADVISORY_FLOOR: f64 = 0.25;

/// Resolved geometry of one node (absolute scene coordinates).
struct Rect {
    y: f64,
    w: f64,
    h: f64,
}

/// The resolved trailing-void fraction of `root_id`'s board, or `None` when
/// unmeasurable: root missing, a non-numeric (hug) board size, no scene
/// entry, or no content at all. Range `0.0` (content reaches the bottom
/// edge) ..= `1.0` (nothing but decoration).
pub(crate) fn root_trailing_void_ratio(state: &EditorState, root_id: &str) -> Option<f64> {
    let root = op_editor_core::walkers::find_node(
        state.active_children(),
        &NodeId::new(root_id.to_string()),
    )?;
    // Fixed board only: a hug-height root resolves to its content, where
    // there is no void to measure. The AUTHORED size is the board by
    // contract — the resolved scene root can grow past it when content
    // overflows, and measuring against the grown rect would read an
    // overflowing board as "full".
    let authored_w = root.width_px().filter(|width| *width > 0.0)?;
    let authored_h = root.height_px().filter(|height| *height > 0.0)?;
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let page = scene.active_page()?;
    let root_scene = find_scene_node(&page.children, root_id)?;
    let board = Rect {
        y: f64::from(root_scene.aggregate_bounds().origin.y),
        w: authored_w,
        h: authored_h,
    };
    let content_bottom = content_bottom_under(root, &root_scene.children, &board)?;
    let void = (board.y + board.h - content_bottom).max(0.0);
    Some((void / board.h).clamp(0.0, 1.0))
}

/// Lowest bottom edge (absolute scene y) of the in-flow content under
/// `root`, ignoring authored overlays and full-bleed decoration. `None`
/// when the subtree holds no content evidence at all.
fn content_bottom_under(root: &PenNode, scene_children: &[SceneNode], board: &Rect) -> Option<f64> {
    let mut bottom: Option<f64> = None;
    for child_scene in scene_children {
        let Some(child) = find_pen_descendant(root, &child_scene.id) else {
            continue;
        };
        // Authored x/y overlay: decoration that ignores layout — its subtree
        // proves nothing about where the content ends (P1.5 discipline).
        if child.base().x.is_some() || child.base().y.is_some() {
            continue;
        }
        let rect = scene_rect(child_scene);
        // Whole-board decoration (resolved size == board size): ignored.
        let full_bleed = (rect.w - board.w).abs() <= 0.5 && (rect.h - board.h).abs() <= 0.5;
        if full_bleed {
            continue;
        }
        // A node whose own bounds have VERTICAL extent occupies its own
        // resolved box (aggregate == own bounds) — an empty hug section that
        // resolved to zero height owns no space and proves nothing. An
        // unbounded container likewise counts only through its children.
        if child_scene.bounds.size.y > 0.0 {
            let own_bottom = rect.y + rect.h;
            bottom = Some(bottom.map_or(own_bottom, |b| b.max(own_bottom)));
        }
        if child
            .children()
            .is_some_and(|children| !children.is_empty())
        {
            if let Some(child_bottom) = content_bottom_under(child, &child_scene.children, board) {
                bottom = Some(bottom.map_or(child_bottom, |b| b.max(child_bottom)));
            }
        }
    }
    bottom
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

fn scene_rect(node: &SceneNode) -> Rect {
    let bounds = node.aggregate_bounds();
    Rect {
        y: f64::from(bounds.origin.y),
        w: f64::from(bounds.size.x),
        h: f64::from(bounds.size.y),
    }
}

/// One board-trailing-void advisory for the `finalize_design` summary
/// (DS P2-b item C, riding the P2-a advisories channel): the sparse board's
/// root id plus a message naming the void percentage and the fix direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardTrailingVoidAdvisory {
    pub code: &'static str,
    pub node_ids: Vec<String>,
    pub message: String,
}

/// Scan every top-level root of the active page for fixed Card/Deck boards
/// whose trailing void is still >= [`BOARD_VOID_ADVISORY_FLOOR`] of the
/// board height after the cleanup passes — content too sparse for the
/// centre repair to rescue. Read-only: the document is never modified here;
/// the caller surfaces hits as advisories so the model-in-the-loop adds
/// density itself (add content or scale up type/spacing).
pub fn collect_board_trailing_void(state: &EditorState) -> Vec<BoardTrailingVoidAdvisory> {
    state
        .active_children()
        .iter()
        .filter_map(|root| {
            let root_id = root.id_str();
            let form = crate::geometry_validation::root_design_form(state, root_id);
            if !form.is_card_board() && !form.is_deck_board() {
                return None;
            }
            let void = root_trailing_void_ratio(state, root_id)?;
            if void < BOARD_VOID_ADVISORY_FLOOR {
                return None;
            }
            Some(BoardTrailingVoidAdvisory {
                code: "board-trailing-void",
                node_ids: vec![root_id.to_string()],
                message: format!(
                    "{}% of the board height sits empty below the content — \
                     add content or scale up type/spacing",
                    (void * 100.0).round() as i64
                ),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "board_trailing_void_tests.rs"]
mod tests;
