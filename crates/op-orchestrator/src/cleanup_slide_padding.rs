//! Enforce the deck safe-margin floor on slide roots (DS P1-a, pass 3),
//! the card margin floor on portrait card boards (DS P1.5 + P2-b/c A), and
//! the board text-overflow wrap (DS P2-c B, pass `board-text-wrap`).
//!
//! Measured 0814-08-14: a deck page's title sat flush against the canvas left
//! edge (x=0) with the root carrying no horizontal padding at all. Decks have
//! a fixed safe margin by contract (the deck contract locks "safe margins"
//! across pages), so a content edge closer than 24px to the root's edge on a
//! root with less than 64px of horizontal padding is provably not authored
//! intent — it is a missing margin, and the repair is to raise the ROOT's
//! horizontal padding to the floor. Only raising, never lowering.
//!
//! Cards joined the gate in DS P1.5 (0815): a portrait card board is the same
//! "fixed independent board" shape as a deck in a portrait aspect, and the
//! 0815 card corpus carries the same margin-ownership contract at a 48px
//! floor (1080-wide cards — the corpus rule "1080-wide card roots >= 48px").
//! DS P2-b item A adds the card's VERTICAL component: the 0815 card measured
//! its masthead < 40px from the board's top edge, so when a text / content
//! descendant's top edge provably sits < 24px from the board top, the root's
//! top padding is raised to the 48px floor. DS P2-c item A mirrors the same
//! evidence on the BOTTOM edge: a footer section carrying `[0, 80]` padding
//! and a padding-less root leaves the footer text flush against the board's
//! bottom edge — proof the bottom margin is missing, so only the bottom
//! padding rises. A deck's vertical composition stays untouched here — the
//! centre pass owns it. The floor is per-form: Deck keeps 64 horizontal,
//! Card uses 48 both axes. A phone screen is NEVER gated — edge-to-edge
//! content is its legal contract.
//!
//! ## Why the vertical floor lifts PER EDGE while the horizontal one lifts
//! ## both sides
//!
//! The horizontal precedent raises BOTH left and right when either side is
//! flush: margins are symmetric by nature, a flush left edge proves the whole
//! horizontal margin missing, and a deck/card body is expected to sit on a
//! symmetric gutter. The vertical axis is different: a card's head is
//! routinely composed with a large authored top gap (the P2-c card carries a
//! 96px masthead gap) while its tail routinely sits flush against the bottom
//! edge — that asymmetry is normal card composition, not a defect pattern.
//! Symmetric lifting would therefore turn a correct 96px head composition
//! into a 144px one and destroy it. So each vertical edge rises on ITS OWN
//! flush evidence, `max(current, 48)` — only up, never down, never both.
//!
//! ## Why the detection is geometric and the predicate narrow
//!
//! - The gate is the single form judge (`geometry_validation::root_design_form`
//!   → `DesignForm::Deck` / `DesignForm::Card`), never a name heuristic.
//! - Detection runs on the REAL jian layout, re-parsed from the CURRENT
//!   document state at pass time (the same convention every geometric pass
//!   follows — each one builds its own scene via
//!   `editor_state_to_active_page_layout_scene` instead of trusting a scene
//!   captured before the earlier passes repaired the tree). That is what lets
//!   the floor see repairs that ran earlier in the same round — in
//!   particular the section-margin pass raising every section to the group
//!   margin, after which nothing is flush any more and the floor must stand
//!   down.
//! - The flush evidence is CONTENT, not frames: a full-width in-flow frame
//!   (`fill_container` section) touches the root edge BY CONSTRUCTION, so its
//!   own frame box proves nothing — only its content can. The same text /
//!   narrow-content discipline as the section-margin pass applies: text
//!   always counts; a non-text node counts only when it is an in-flow node
//!   narrower than the root; a full-bleed fill layer (resolved size == root
//!   size) and an authored-position overlay (explicit x/y) are decoration and
//!   neither trigger nor block the repair.
//! - Content that is AUTHORED too wide to fit inside the padded inner width
//!   (fixed numeric width > root - 2×floor) blocks the repair: raising the
//!   floor would only make it overflow, which is the geometry loop's
//!   problem, not this pass's.

use super::*;

use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::text::{TextContent, TextGrowth};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_scene::layout_scene::SceneNode;

use crate::cleanup::cleanup_equalize_siblings::{padding_edges, padding_value};
use crate::design_type::DesignForm;

/// The deck horizontal safe-margin floor, per side.
const SLIDE_PADDING_FLOOR: f64 = 64.0;
/// The card horizontal safe-margin floor, per side (0815 corpus: 1080-wide
/// card roots carry >= 48px).
const CARD_PADDING_FLOOR: f64 = 48.0;
/// A content edge closer than this to the root edge proves the margin is
/// missing rather than merely small.
const SLIDE_EDGE_GAP: f64 = 24.0;

/// Resolved geometry of one node (absolute scene coordinates).
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Per-root cleanup pass: raise a deck root's horizontal padding to
/// [`SLIDE_PADDING_FLOOR`] (a card's horizontal padding to
/// [`CARD_PADDING_FLOOR`]) when content provably sits against a horizontal
/// root edge, and — DS P2-b A + P2-c A — raise a card's TOP or BOTTOM
/// padding to the same floor, each edge on its own flush evidence. A flush
/// top lifts only `top`, a flush bottom lifts only `bottom` (see the module
/// doc for why the vertical axis is per-edge while the horizontal one is
/// symmetric). Returns `true` iff the root was patched.
pub(super) fn enforce_slide_padding_floor(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let form = crate::geometry_validation::root_design_form(sink.state(), root_id);
    let floor = match form {
        DesignForm::Deck => SLIDE_PADDING_FLOOR,
        DesignForm::Card => CARD_PADDING_FLOOR,
        _ => return false,
    };
    let Some(root) = find_root(sink.state(), root_id) else {
        return false;
    };
    let Some(props) = container_props(root) else {
        return false;
    };
    let Some([top, right, bottom, left]) = padding_edges(props.padding.as_ref()) else {
        // A variable-bound padding cannot be proven wrong; leave it alone.
        return false;
    };
    // Horizontal floor: deck + card. Vertical floor: card only — a deck's
    // vertical composition belongs to the centre pass. Each vertical edge is
    // gated and lifted independently (per-edge semantics, DS P2-c A).
    let horizontal_below = right.min(left) < floor;
    let top_below = form.is_card_board() && top < CARD_PADDING_FLOOR;
    let bottom_below = form.is_card_board() && bottom < CARD_PADDING_FLOOR;
    if !horizontal_below && !top_below && !bottom_below {
        return false;
    }

    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let Some(page) = scene.active_page() else {
        return false;
    };
    let Some(root_scene) = find_scene_node(&page.children, root_id) else {
        return false;
    };
    let root_bounds = scene_rect(root_scene);

    let mut horizontal_violation = false;
    if horizontal_below {
        for child_scene in &root_scene.children {
            let Some(child) = root
                .children()
                .and_then(|children| children.iter().find(|c| c.id_str() == child_scene.id))
            else {
                continue;
            };
            let rect = scene_rect(child_scene);
            // A full-width in-flow frame (a `fill_container` section) spans the
            // board by construction — its own flush frame edge is not evidence.
            // Only the CONTENT inside it can prove a missing margin, so descend
            // with the same text / narrow-content discipline the section-margin
            // pass uses. After that pass raised every section to the group
            // margin, this re-parsed scene shows the content pulled off the edge
            // and the floor correctly stands down.
            if is_full_width_layout_wrapper(child, &rect, &root_bounds) {
                match wrapper_content_edge(child, &child_scene.children, &root_bounds, floor) {
                    ContentEdge::Flush => horizontal_violation = true,
                    ContentEdge::Blocked => return false,
                    ContentEdge::Clear => {}
                }
                continue;
            }
            if !is_content_child(child, &rect, &root_bounds) {
                continue;
            }
            // A fixed-width child that cannot fit inside the padded inner width
            // would only overflow more after the repair — not this pass's fix.
            if child
                .width_px()
                .is_some_and(|width| width > root_bounds.w - 2.0 * floor)
            {
                return false;
            }
            let left_gap = rect.x - root_bounds.x;
            let right_gap = (root_bounds.x + root_bounds.w) - (rect.x + rect.w);
            if left_gap < SLIDE_EDGE_GAP || right_gap < SLIDE_EDGE_GAP {
                horizontal_violation = true;
            }
        }
    }
    // Vertical evidence (card only), per edge: a text / content descendant
    // whose TOP edge sits closer than [`SLIDE_EDGE_GAP`] to the board top
    // lifts only `top`; the mirror evidence on the BOTTOM edge (the P2-c
    // flush footer) lifts only `bottom`. Same content discipline (drill-down
    // semantics, P1.5) on both edges. `max(current, floor)` — only up.
    let top_violation = top_below
        && vertical_flush_descendant(root, &root_scene.children, &root_bounds, VerticalEdge::Top);
    let bottom_violation = bottom_below
        && vertical_flush_descendant(
            root,
            &root_scene.children,
            &root_bounds,
            VerticalEdge::Bottom,
        );
    if !horizontal_violation && !top_violation && !bottom_violation {
        return false;
    }

    let new_right = if horizontal_violation {
        right.max(floor)
    } else {
        right
    };
    let new_left = if horizontal_violation {
        left.max(floor)
    } else {
        left
    };
    let new_top = if top_violation {
        top.max(CARD_PADDING_FLOOR)
    } else {
        top
    };
    let new_bottom = if bottom_violation {
        bottom.max(CARD_PADDING_FLOOR)
    } else {
        bottom
    };
    let patch = serde_json::to_string(&padding_value([new_top, new_right, new_bottom, new_left]))
        .unwrap_or_default();
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new(root_id),
        patch_json: format!(r#"{{"padding":{patch}}}"#),
        page_id: None,
    });
    true
}

/// Which vertical board edge a flush-evidence walk is measuring.
#[derive(Clone, Copy)]
enum VerticalEdge {
    Top,
    Bottom,
}

/// Vertical flush evidence (card only): does any text / in-flow content
/// descendant sit closer than [`SLIDE_EDGE_GAP`] to the measured board edge
/// (top OR bottom)? The same content discipline as the horizontal walk: text
/// counts; an authored x/y overlay and a full-bleed layer are decoration; a
/// full-width or full-height frame with children is layout structure whose
/// own edge is structural, so descend into it — a `fill_container` section
/// spans the board by construction, its masthead text is what proves the
/// top margin, its footer text the bottom one.
fn vertical_flush_descendant(
    root: &PenNode,
    scene_children: &[SceneNode],
    root_bounds: &Rect,
    edge: VerticalEdge,
) -> bool {
    for child_scene in scene_children {
        let Some(child) = find_pen_descendant(root, &child_scene.id) else {
            continue;
        };
        let rect = scene_rect(child_scene);
        if child.base().x.is_some() || child.base().y.is_some() {
            continue;
        }
        let full_bleed =
            (rect.w - root_bounds.w).abs() <= 0.5 && (rect.h - root_bounds.h).abs() <= 0.5;
        if full_bleed {
            continue;
        }
        let is_layout_structure =
            (rect.w - root_bounds.w).abs() <= 0.5 || (rect.h - root_bounds.h).abs() <= 0.5;
        if is_layout_structure
            && child
                .children()
                .is_some_and(|children| !children.is_empty())
        {
            if vertical_flush_descendant(child, &child_scene.children, root_bounds, edge) {
                return true;
            }
            continue;
        }
        // Text or narrow in-flow content: its own edge is the evidence.
        let gap = match edge {
            VerticalEdge::Top => rect.y - root_bounds.y,
            VerticalEdge::Bottom => (root_bounds.y + root_bounds.h) - (rect.y + rect.h),
        };
        if gap < SLIDE_EDGE_GAP {
            return true;
        }
    }
    false
}

/// Content counts when it is text or an in-flow non-decorative child.
/// Decoration — a full-bleed fill layer (resolved size == root size) or an
/// authored-position overlay (explicit x/y) — neither triggers the floor nor
/// blocks other content from triggering it.
fn is_content_child(child: &PenNode, rect: &Rect, root_bounds: &Rect) -> bool {
    if matches!(child, PenNode::Text(_)) {
        return true;
    }
    // Authored x/y: an absolutely-positioned overlay (the "layout:none
    // 覆盖层" the root padding cannot move anyway).
    if child.base().x.is_some() || child.base().y.is_some() {
        return false;
    }
    // Full-bleed fill layer: the node resolves to the whole board. Only a
    // NON-text child can be one — text is always content above.
    if (rect.w - root_bounds.w).abs() <= 0.5 && (rect.h - root_bounds.h).abs() <= 0.5 {
        return false;
    }
    true
}

/// A full-width in-flow frame WITH children — a `fill_container` section —
/// whose own flush frame edge is structural rather than evidence. A full-width
/// frame without children (an unpadded banner, say) stays plain content and
/// falls through to [`is_content_child`], exactly as before.
fn is_full_width_layout_wrapper(child: &PenNode, rect: &Rect, root_bounds: &Rect) -> bool {
    if matches!(child, PenNode::Text(_)) {
        return false;
    }
    if child.base().x.is_some() || child.base().y.is_some() {
        return false;
    }
    if (rect.w - root_bounds.w).abs() > 0.5 {
        return false;
    }
    // A full-bleed layer is decoration — `is_content_child` handles it.
    if (rect.h - root_bounds.h).abs() <= 0.5 {
        return false;
    }
    child
        .children()
        .is_some_and(|children| !children.is_empty())
}

/// What the content edge under a full-width wrapper proves.
enum ContentEdge {
    /// A descendant proved content flush against the canvas edge.
    Flush,
    /// Nothing flush to prove — the wrapper's content respects a margin.
    Clear,
    /// A fixed-width descendant cannot fit inside the padded inner width;
    /// the floor would only make it overflow, so the repair is blocked.
    Blocked,
}

/// Mirror of the section-margin pass's content discipline, applied to the
/// content INSIDE a full-width wrapper: text always counts; a non-text node
/// counts only when it is in-flow and narrower than the root; a full-bleed
/// layer and an authored x/y overlay are decoration; a nested full-width
/// frame is further layout structure whose own content decides.
fn wrapper_content_edge(
    wrapper: &PenNode,
    scene_children: &[SceneNode],
    root_bounds: &Rect,
    floor: f64,
) -> ContentEdge {
    for child_scene in scene_children {
        let Some(child) = find_pen_descendant(wrapper, &child_scene.id) else {
            continue;
        };
        let rect = scene_rect(child_scene);
        if child.base().x.is_some() || child.base().y.is_some() {
            continue;
        }
        let full_bleed =
            (rect.w - root_bounds.w).abs() <= 0.5 && (rect.h - root_bounds.h).abs() <= 0.5;
        if full_bleed {
            continue;
        }
        if (rect.w - root_bounds.w).abs() <= 0.5
            && child
                .children()
                .is_some_and(|children| !children.is_empty())
        {
            match wrapper_content_edge(child, &child_scene.children, root_bounds, floor) {
                ContentEdge::Flush => return ContentEdge::Flush,
                ContentEdge::Blocked => return ContentEdge::Blocked,
                ContentEdge::Clear => {}
            }
            continue;
        }
        // A fixed-width node that cannot fit inside the padded inner width
        // would only overflow more after the repair — not this pass's fix.
        if child
            .width_px()
            .is_some_and(|width| width > root_bounds.w - 2.0 * floor)
        {
            return ContentEdge::Blocked;
        }
        let left_gap = rect.x - root_bounds.x;
        let right_gap = (root_bounds.x + root_bounds.w) - (rect.x + rect.w);
        if left_gap < SLIDE_EDGE_GAP || right_gap < SLIDE_EDGE_GAP {
            return ContentEdge::Flush;
        }
    }
    ContentEdge::Clear
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

fn container_props(node: &PenNode) -> Option<&jian_ops_schema::node::container::ContainerProps> {
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

fn scene_rect(node: &SceneNode) -> Rect {
    let bounds = node.aggregate_bounds();
    Rect {
        x: f64::from(bounds.origin.x),
        y: f64::from(bounds.origin.y),
        w: f64::from(bounds.size.x),
        h: f64::from(bounds.size.y),
    }
}

// ── Board text horizontal-overflow wrap (DS P2-c B, `board-text-wrap`) ───────

/// Slack: a text right edge up to this far past the board's right inner edge
/// is rounding noise, not clipped glyphs; beyond it the canvas provably cuts
/// the text off.
const BOARD_TEXT_OVERFLOW_EPS: f64 = 2.0;

/// One duplicate-copy group: every text under `parent_id` whose content and
/// `fontSize` equal `content` / `font_size` is the same modelled shape (the
/// shadow/outline double-copy hack) and must wrap consistently.
struct CopyGroup {
    parent_id: String,
    content: String,
    font_size: f64,
}

/// Per-root cleanup pass (Card/Deck board gate): a text descendant whose REAL
/// layout puts its right edge past the root's RIGHT INNER edge (board width −
/// resolved right padding) by more than [`BOARD_TEXT_OVERFLOW_EPS`] while its
/// `textGrowth` is not fixed-width has glyphs the canvas provably clips —
/// clipping a title can never be authored intent, so the judgement holds. The
/// repair is `width: "fill_container"` + `textGrowth: "fixed-width"`, the
/// same wrap posture the board's subtitle / entry texts already use; the next
/// layout round wraps it inside the board instead of clipping it.
///
/// Overlay discipline: a text inside a container that carries an authored x/y
/// is pinned on a floating layer (a badge, a sticker) whose bleed past the
/// board edge can be intentional, and a text pinned by its OWN authored x/y
/// directly under a flex parent is the same shape — neither triggers. But a
/// text pinned INSIDE a `layout:none` band that is itself in flow (no
/// authored x/y — the measured headline-wrap) is the band's content, and its
/// pinned x/y is just the shadow copy's offset, not a floating layer: it
/// triggers. When one text triggers, EVERY sibling under the same parent with
/// the same content + `fontSize` is patched together — two copies that wrap
/// differently destroy the double-copy effect. Returns `true` iff at least
/// one text was patched.
pub(super) fn wrap_board_overflowing_text(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let form = crate::geometry_validation::root_design_form(sink.state(), root_id);
    if !(form.is_card_board() || form.is_deck_board()) {
        return false;
    }
    let Some(root) = find_root(sink.state(), root_id) else {
        return false;
    };
    let Some(props) = container_props(root) else {
        return false;
    };
    // A variable-bound padding cannot define the inner edge; stand down.
    let Some([_, right, _, _]) = padding_edges(props.padding.as_ref()) else {
        return false;
    };

    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let Some(page) = scene.active_page() else {
        return false;
    };
    let Some(root_scene) = find_scene_node(&page.children, root_id) else {
        return false;
    };
    let root_bounds = scene_rect(root_scene);
    let right_inner = root_bounds.x + root_bounds.w - right;

    // Walk the root's CHILDREN: the root's own x/y is its canvas position,
    // not a floating-layer pin, so it must not start the overlay flag.
    let root_layout = props.layout.as_ref().cloned();
    let mut groups: Vec<CopyGroup> = Vec::new();
    if let Some(root_children) = root.children() {
        for child in root_children {
            collect_copy_group_triggers(
                child,
                root_id,
                root_layout.clone(),
                false,
                &root_scene.children,
                right_inner,
                &mut groups,
            );
        }
    }
    if groups.is_empty() {
        return false;
    }
    let mut patched = false;
    for group in &groups {
        patched |= patch_copy_group(sink, root_id, group);
    }
    patched
}

/// DFS over the pen tree, mirrored against the resolved scene, collecting the
/// duplicate-copy groups that contain a provably clipped text.
///
/// `parent_id` is the id of `node`'s pen parent (`root_id` for the root's own
/// children) and `parent_layout` that parent's layout mode; `in_floating_layer`
/// is set once an ancestor container carries an authored x/y. A container's
/// own authored x/y makes its whole subtree a floating layer; a TEXT's
/// authored x/y counts as a floating overlay only when its parent is a flex
/// container — the pinned text then floats on the board flow — while the same
/// pin inside a `layout:none` (or layout-less) in-flow band is the band's
/// content and stays eligible.
#[allow(clippy::too_many_arguments)]
fn collect_copy_group_triggers(
    node: &PenNode,
    parent_id: &str,
    parent_layout: Option<LayoutMode>,
    in_floating_layer: bool,
    scene_children: &[SceneNode],
    right_inner: f64,
    groups: &mut Vec<CopyGroup>,
) {
    let own_layout = container_props(node).and_then(|props| props.layout.as_ref().cloned());
    let floating = in_floating_layer
        || (!matches!(node, PenNode::Text(_))
            && (node.base().x.is_some() || node.base().y.is_some()));
    if let PenNode::Text(text) = node {
        if floating {
            return;
        }
        let pinned = text.base.x.is_some() || text.base.y.is_some();
        let parent_is_flex = matches!(
            parent_layout,
            Some(LayoutMode::Vertical | LayoutMode::Horizontal)
        );
        if pinned && parent_is_flex {
            return;
        }
        // Already wrapping → nothing to repair (and the shadow copy's +3px
        // offset keeps it 3px past the edge even after the fix — the growth
        // keyword is what proves the repair, so it must gate the trigger).
        if matches!(
            text.text_growth,
            Some(TextGrowth::FixedWidth | TextGrowth::FixedWidthHeight)
        ) {
            return;
        }
        let Some(scene) = find_scene_node(scene_children, node.id_str()) else {
            return;
        };
        let rect = scene_rect(scene);
        if rect.x + rect.w <= right_inner + BOARD_TEXT_OVERFLOW_EPS {
            return;
        }
        let group = CopyGroup {
            parent_id: parent_id.to_string(),
            content: text_content_string(&text.content),
            font_size: text.font_size.unwrap_or(0.0),
        };
        if !groups.iter().any(|existing| {
            existing.parent_id == group.parent_id
                && existing.content == group.content
                && existing.font_size == group.font_size
        }) {
            groups.push(group);
        }
        return;
    }
    let Some(children) = node.children() else {
        return;
    };
    for child in children {
        collect_copy_group_triggers(
            child,
            node.id_str(),
            own_layout.clone(),
            floating,
            scene_children,
            right_inner,
            groups,
        );
    }
}

/// Patch every member of `group`: each text under `parent` with the group's
/// content + `fontSize` that is not already on the wrap posture gets
/// `width: "fill_container"` + `textGrowth: "fixed-width"`. Commands are
/// planned against the immutable state first, then applied, so the state
/// borrow ends before the sink is borrowed mutably.
fn patch_copy_group(sink: &mut dyn DocSink, root_id: &str, group: &CopyGroup) -> bool {
    let commands: Vec<EditorCommand> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return false;
        };
        let parent = if group.parent_id == root_id {
            root
        } else {
            let Some(parent) = find_pen_descendant(root, &group.parent_id) else {
                return false;
            };
            parent
        };
        let Some(children) = parent.children() else {
            return false;
        };
        let mut commands: Vec<EditorCommand> = Vec::new();
        for child in children {
            let PenNode::Text(text) = child else {
                continue;
            };
            if text_content_string(&text.content) != group.content
                || text.font_size.unwrap_or(0.0) != group.font_size
            {
                continue;
            }
            let already_fill = matches!(
                text.width,
                Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
            );
            let already_wrap = matches!(
                text.text_growth,
                Some(TextGrowth::FixedWidth | TextGrowth::FixedWidthHeight)
            );
            let node_id = NodeId::new(child.id_str());
            if !already_fill {
                commands.push(EditorCommand::SetNodeLayoutProp {
                    node_id: node_id.clone(),
                    property: "width".to_string(),
                    value: LayoutPropValue::Keyword("fill_container".to_string()),
                });
            }
            if !already_wrap {
                commands.push(EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property: "textGrowth".to_string(),
                    value: LayoutPropValue::Keyword("fixed-width".to_string()),
                });
            }
        }
        commands
    };
    let applied = !commands.is_empty();
    for command in commands {
        sink.apply(command);
    }
    applied
}

/// The comparable text of a text node: the plain string, or the joined
/// segment texts of a styled run.
fn text_content_string(content: &TextContent) -> String {
    match content {
        TextContent::Plain(text) => text.clone(),
        TextContent::Styled(segments) => segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect(),
    }
}

/// Driver hook for the board-margin family: the padding floor (checkpointed
/// under Layout as `slide-padding-floor`, unchanged) followed by the board
/// text wrap (checkpointed under Overflow as `board-text-wrap`, DS P2-c B).
/// The wrap reads the floor's settled padding, so it runs after it. Both
/// checkpoint names are the pre-existing contracts; grouping the two calls
/// here keeps `cleanup.rs` under its line cap without moving a checkpoint.
pub(super) fn enforce_slide_padding_floor_and_board_text_wrap(
    sink: &mut dyn DocSink,
    root_id: &str,
    summary: &mut RepairSummary,
    counter: &mut RepairCounter,
) {
    enforce_slide_padding_floor(sink, root_id);
    counter.checkpoint(summary, CheckCategory::Layout, "slide-padding-floor");
    wrap_board_overflowing_text(sink, root_id);
    counter.checkpoint(summary, CheckCategory::Overflow, "board-text-wrap");
}

#[cfg(test)]
#[path = "cleanup_slide_padding_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cleanup_board_text_wrap_tests.rs"]
mod board_text_wrap_tests;
