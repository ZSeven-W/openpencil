//! Floating align / distribute toolbar — appears above the canvas
//! whenever the active selection has 2+ nodes (distribute also
//! requires 3+; the 2-node case still shows the buttons, but the
//! distribute presses no-op gracefully via `align_selected`).
//!
//! Layout: three button groups separated by ~12 px gutters:
//!   - Align L / Center-H / Right
//!   - Align Top / Center-V / Bottom
//!   - Distribute H / Distribute V
//!
//! Anchored to the horizontal center of the canvas region, ~16 px
//! below the canvas top edge.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::{Color, Point2D, Rect, RenderBackend};
use jian_ops_schema::node::PenNode;
use op_editor_core::walkers::find_node;
use op_editor_core::{AlignAction, BooleanOp, EditorState};

pub const ALIGN_TOOLBAR_HEIGHT: f32 = 36.0;
const BUTTON_SIZE: f32 = 28.0;
const ICON_SIZE: f32 = 16.0;
const INNER_GAP: f32 = 2.0;
const GROUP_GAP: f32 = 10.0;
const SIDE_PAD: f32 = 6.0;
const CORNER_RADIUS: f32 = 8.0;

/// (8 buttons × 28) + (6 inner gaps × 2) + (2 group gaps × 10) + (2 sides × 6).
pub const ALIGN_TOOLBAR_WIDTH: f32 =
    BUTTON_SIZE * 8.0 + INNER_GAP * 6.0 + GROUP_GAP * 2.0 + SIDE_PAD * 2.0;

/// Pixels reserved on the canvas-left edge for the vertical Toolbar
/// column. Mirrors paint-side geometry: `TOOLBAR_INSET_X` (12 in
/// shell-native `widget_host/helpers.rs` and shell-web `widget_host.rs`)
/// + `TOOLBAR_WIDTH` (44) = 56 — leaves the tool column unobscured.
///
/// Has to be a shell-core local because shell-core can't depend on
/// either host crate's helpers. Keep in sync if either constant moves.
const VERTICAL_TOOLBAR_RESERVE: f32 = 56.0;

const ITEMS: &[(AlignAction, Icon)] = &[
    (AlignAction::Left, Icon::AlignLeft),
    (AlignAction::CenterH, Icon::AlignCenterH),
    (AlignAction::Right, Icon::AlignRight),
    (AlignAction::Top, Icon::AlignTop),
    (AlignAction::CenterV, Icon::AlignCenterV),
    (AlignAction::Bottom, Icon::AlignBottom),
    (AlignAction::DistributeH, Icon::DistributeH),
    (AlignAction::DistributeV, Icon::DistributeV),
];

const BOOLEAN_ITEMS: &[BooleanOp] = &[
    BooleanOp::Union,
    BooleanOp::Subtract,
    BooleanOp::Intersect,
    BooleanOp::Exclude,
];

/// Group divider indices (after these positions, insert a `GROUP_GAP`
/// instead of the default `INNER_GAP`). Two dividers split the 8
/// buttons into [3, 3, 2].
const GROUP_BREAKS: &[usize] = &[3, 6];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignToolbarHit {
    Align(AlignAction),
    Boolean(BooleanOp),
}

pub struct AlignToolbar {
    rect: Rect,
    boolean_ops: bool,
}

impl AlignToolbar {
    /// Build a toolbar centered horizontally inside `canvas_region`
    /// when the editor selection has 2+ nodes. Returns `None`
    /// otherwise so the host can skip paint + hit-test entirely.
    pub fn for_canvas_region(canvas_region: Rect, state: &EditorState) -> Option<Self> {
        if state.selection.len() < 2 {
            return None;
        }
        // Center horizontally, then clamp into
        // [canvas_left + VERTICAL_TOOLBAR_RESERVE, canvas_right - W].
        // The min_x reserve keeps the floating toolbar from overlapping
        // the vertical Toolbar's column on the canvas-left edge; the
        // max_x clamp keeps it inside the right rail. When the canvas
        // can't fit both, hide entirely — never render a clipped or
        // out-of-region pill with stale hit-test geometry.
        let boolean_ops = can_boolean_op(state);
        let toolbar_width = toolbar_width(boolean_ops);
        let min_x = canvas_region.origin.x + VERTICAL_TOOLBAR_RESERVE;
        let max_x = canvas_region.origin.x + canvas_region.size.x - toolbar_width;
        if max_x < min_x {
            return None;
        }
        let cx = canvas_region.origin.x + canvas_region.size.x / 2.0;
        let mut x = cx - toolbar_width / 2.0;
        if x < min_x {
            x = min_x;
        }
        if x > max_x {
            x = max_x;
        }
        let y = canvas_region.origin.y + 16.0;
        Some(Self {
            rect: Rect::xywh(x, y, toolbar_width, ALIGN_TOOLBAR_HEIGHT),
            boolean_ops,
        })
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Paint the toolbar background + 8 buttons. `hovered` tints the
    /// matching button with `theme.muted`.
    pub fn paint(
        &self,
        backend: &mut dyn RenderBackend,
        theme: &Theme,
        hovered: Option<AlignAction>,
    ) {
        backend.fill_round_rect(self.rect, CORNER_RADIUS, theme.popover);
        backend.stroke_round_rect(self.rect, CORNER_RADIUS, theme.border, 1.0);
        for (i, (action, icon)) in ITEMS.iter().enumerate() {
            let r = self.button_rect(i);
            if hovered == Some(*action) {
                backend.fill_round_rect(r, 5.0, theme.muted);
            }
            let icon_x = r.origin.x + (r.size.x - ICON_SIZE) / 2.0;
            let icon_y = r.origin.y + (r.size.y - ICON_SIZE) / 2.0;
            draw_icon(
                backend,
                *icon,
                Point2D::new(icon_x, icon_y),
                ICON_SIZE,
                theme.foreground,
                1.5,
            );
        }
        if self.boolean_ops {
            for (i, op) in BOOLEAN_ITEMS.iter().enumerate() {
                let r = self.button_rect(ITEMS.len() + i);
                let icon_x = r.origin.x + (r.size.x - ICON_SIZE) / 2.0;
                let icon_y = r.origin.y + (r.size.y - ICON_SIZE) / 2.0;
                paint_boolean_icon(
                    backend,
                    *op,
                    Point2D::new(icon_x, icon_y),
                    ICON_SIZE,
                    theme.foreground,
                );
            }
        }
    }

    /// Map a screen point to an `AlignAction`. None when the point
    /// lands outside the toolbar or in a gutter.
    pub fn hit_test(&self, point: Point2D) -> Option<AlignAction> {
        match self.hit_test_action(point) {
            Some(AlignToolbarHit::Align(action)) => Some(action),
            _ => None,
        }
    }

    /// Map a screen point to either an align/distribute action or a
    /// boolean operation. None when the point lands outside the
    /// toolbar or in a gutter.
    pub fn hit_test_action(&self, point: Point2D) -> Option<AlignToolbarHit> {
        if !rect_contains(self.rect, point) {
            return None;
        }
        for (i, (action, _)) in ITEMS.iter().enumerate() {
            if rect_contains(self.button_rect(i), point) {
                return Some(AlignToolbarHit::Align(*action));
            }
        }
        if self.boolean_ops {
            for (i, op) in BOOLEAN_ITEMS.iter().enumerate() {
                if rect_contains(self.button_rect(ITEMS.len() + i), point) {
                    return Some(AlignToolbarHit::Boolean(*op));
                }
            }
        }
        None
    }

    fn button_rect(&self, index: usize) -> Rect {
        let mut x = self.rect.origin.x + SIDE_PAD;
        for i in 0..index {
            x += BUTTON_SIZE;
            x += if is_group_break_after(i, self.boolean_ops) {
                GROUP_GAP
            } else {
                INNER_GAP
            };
        }
        let y = self.rect.origin.y + (self.rect.size.y - BUTTON_SIZE) / 2.0;
        Rect::xywh(x, y, BUTTON_SIZE, BUTTON_SIZE)
    }
}

fn toolbar_width(boolean_ops: bool) -> f32 {
    if !boolean_ops {
        return ALIGN_TOOLBAR_WIDTH;
    }
    ALIGN_TOOLBAR_WIDTH
        + GROUP_GAP
        + BUTTON_SIZE * BOOLEAN_ITEMS.len() as f32
        + INNER_GAP * BOOLEAN_ITEMS.len().saturating_sub(1) as f32
}

fn paint_boolean_icon(
    backend: &mut dyn RenderBackend,
    op: BooleanOp,
    top_left: Point2D,
    size: f32,
    color: Color,
) {
    let a = Rect::xywh(
        top_left.x + size * 0.08,
        top_left.y + size * 0.08,
        size * 0.56,
        size * 0.56,
    );
    let b = Rect::xywh(
        top_left.x + size * 0.36,
        top_left.y + size * 0.36,
        size * 0.56,
        size * 0.56,
    );
    let overlap = Rect::xywh(
        top_left.x + size * 0.36,
        top_left.y + size * 0.36,
        size * 0.28,
        size * 0.28,
    );
    let faint = alpha(color, 0.14);
    let muted = alpha(color, 0.42);
    let strong = alpha(color, 0.28);
    match op {
        BooleanOp::Union => {
            backend.fill_round_rect(a, 2.0, faint);
            backend.fill_round_rect(b, 2.0, faint);
            backend.fill_round_rect(overlap, 1.0, strong);
            backend.stroke_round_rect(a, 2.0, color, 1.4);
            backend.stroke_round_rect(b, 2.0, color, 1.4);
        }
        BooleanOp::Subtract => {
            backend.fill_round_rect(a, 2.0, faint);
            backend.stroke_round_rect(a, 2.0, color, 1.4);
            paint_dashed_rect(backend, b, muted, 1.4);
            backend.stroke_line(
                Point2D::new(
                    overlap.origin.x - 1.0,
                    overlap.origin.y + overlap.size.y + 1.0,
                ),
                Point2D::new(
                    overlap.origin.x + overlap.size.x + 1.0,
                    overlap.origin.y - 1.0,
                ),
                color,
                1.4,
            );
        }
        BooleanOp::Intersect => {
            backend.stroke_round_rect(a, 2.0, muted, 1.2);
            backend.stroke_round_rect(b, 2.0, muted, 1.2);
            backend.fill_round_rect(overlap, 1.0, strong);
            backend.stroke_round_rect(overlap, 1.0, color, 1.4);
        }
        BooleanOp::Exclude => {
            backend.stroke_round_rect(a, 2.0, color, 1.4);
            backend.stroke_round_rect(b, 2.0, color, 1.4);
            backend.stroke_line(
                Point2D::new(overlap.origin.x - 0.5, overlap.origin.y - 0.5),
                Point2D::new(
                    overlap.origin.x + overlap.size.x + 0.5,
                    overlap.origin.y + overlap.size.y + 0.5,
                ),
                color,
                1.4,
            );
            backend.stroke_line(
                Point2D::new(
                    overlap.origin.x + overlap.size.x + 0.5,
                    overlap.origin.y - 0.5,
                ),
                Point2D::new(
                    overlap.origin.x - 0.5,
                    overlap.origin.y + overlap.size.y + 0.5,
                ),
                color,
                1.4,
            );
        }
    }
}

fn paint_dashed_rect(backend: &mut dyn RenderBackend, rect: Rect, color: Color, width: f32) {
    let x0 = rect.origin.x;
    let y0 = rect.origin.y;
    let x1 = rect.origin.x + rect.size.x;
    let y1 = rect.origin.y + rect.size.y;
    let d = rect.size.x * 0.32;
    backend.stroke_line(Point2D::new(x0, y0), Point2D::new(x0 + d, y0), color, width);
    backend.stroke_line(Point2D::new(x1 - d, y0), Point2D::new(x1, y0), color, width);
    backend.stroke_line(Point2D::new(x1, y0), Point2D::new(x1, y0 + d), color, width);
    backend.stroke_line(Point2D::new(x1, y1 - d), Point2D::new(x1, y1), color, width);
    backend.stroke_line(Point2D::new(x1, y1), Point2D::new(x1 - d, y1), color, width);
    backend.stroke_line(Point2D::new(x0 + d, y1), Point2D::new(x0, y1), color, width);
    backend.stroke_line(Point2D::new(x0, y1), Point2D::new(x0, y1 - d), color, width);
    backend.stroke_line(Point2D::new(x0, y0 + d), Point2D::new(x0, y0), color, width);
}

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

fn is_group_break_after(index: usize, boolean_ops: bool) -> bool {
    let after = index + 1;
    GROUP_BREAKS.contains(&after) || (boolean_ops && after == ITEMS.len())
}

fn can_boolean_op(state: &EditorState) -> bool {
    if state.selection.len() < 2 {
        return false;
    }
    let children = state.active_children();
    state.selection.set.iter().all(|id| {
        find_node(children, id)
            .map(boolean_supported_node)
            .unwrap_or(false)
    })
}

fn boolean_supported_node(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_)
            | PenNode::Rectangle(_)
            | PenNode::Ellipse(_)
            | PenNode::Polygon(_)
            | PenNode::Path(_)
            | PenNode::Line(_)
    )
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x < r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y < r.origin.y + r.size.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::{ContainerProps, PenNode, PenNodeBase, RectangleNode};
    use jian_ops_schema::sizing::SizingBehavior;
    use op_editor_core::node_id::NodeId as OpNodeId;

    fn doc_with_n_selected(n: usize) -> EditorState {
        let mut state = EditorState::new();
        let ids: Vec<OpNodeId> = (0..n)
            .map(|i| OpNodeId::new(format!("missing-align-test-{i}")))
            .collect();
        state.selection.anchor = ids.last().cloned().unwrap_or_default();
        state.selection.set = ids;
        state
    }

    fn rect_node(id: &str, x: f64, y: f64, w: f64, h: f64) -> PenNode {
        PenNode::Rectangle(RectangleNode {
            base: PenNodeBase {
                id: id.to_string(),
                name: Some(id.to_string()),
                x: Some(x),
                y: Some(y),
                ..Default::default()
            },
            container: ContainerProps {
                width: Some(SizingBehavior::Number(w)),
                height: Some(SizingBehavior::Number(h)),
                ..Default::default()
            },
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }

    fn doc_with_two_rects_selected() -> EditorState {
        let mut state = EditorState::new();
        state
            .active_children_mut()
            .push(rect_node("n10", 0.0, 0.0, 100.0, 80.0));
        state
            .active_children_mut()
            .push(rect_node("n11", 40.0, 0.0, 100.0, 80.0));
        state.selection.set = vec![OpNodeId::new("n10"), OpNodeId::new("n11")];
        state.selection.anchor = OpNodeId::new("n11");
        state
    }

    fn canvas() -> Rect {
        Rect::xywh(0.0, 0.0, 1000.0, 600.0)
    }

    #[test]
    fn single_select_hides_toolbar() {
        let doc = doc_with_n_selected(1);
        assert!(AlignToolbar::for_canvas_region(canvas(), &doc).is_none());
    }

    #[test]
    fn empty_selection_hides_toolbar() {
        let mut doc = doc_with_n_selected(2);
        doc.selection.set.clear();
        doc.selection.anchor = OpNodeId::default();
        assert!(AlignToolbar::for_canvas_region(canvas(), &doc).is_none());
    }

    #[test]
    fn two_selected_shows_toolbar_centered() {
        let doc = doc_with_n_selected(2);
        let tb = AlignToolbar::for_canvas_region(canvas(), &doc).unwrap();
        // Horizontally centered in canvas (cx = 500).
        let expected_x = 500.0 - ALIGN_TOOLBAR_WIDTH / 2.0;
        assert!((tb.rect.origin.x - expected_x).abs() < 0.5);
        // Sits 16 px below canvas top.
        assert_eq!(tb.rect.origin.y, 16.0);
        assert_eq!(tb.rect.size.x, ALIGN_TOOLBAR_WIDTH);
        assert_eq!(tb.rect.size.y, ALIGN_TOOLBAR_HEIGHT);
    }

    #[test]
    fn boolean_compatible_selection_appends_boolean_buttons() {
        let doc = doc_with_two_rects_selected();
        let tb = AlignToolbar::for_canvas_region(canvas(), &doc).unwrap();
        assert!(tb.rect.size.x > ALIGN_TOOLBAR_WIDTH);
        let expected_width = ALIGN_TOOLBAR_WIDTH + GROUP_GAP + BUTTON_SIZE * 4.0 + INNER_GAP * 3.0;
        assert_eq!(tb.rect.size.x, expected_width);
        let first_boolean = tb.button_rect(ITEMS.len());
        let center = Point2D::new(
            first_boolean.origin.x + first_boolean.size.x / 2.0,
            first_boolean.origin.y + first_boolean.size.y / 2.0,
        );
        assert_eq!(
            tb.hit_test_action(center),
            Some(AlignToolbarHit::Boolean(BooleanOp::Union))
        );
        assert_eq!(tb.hit_test(center), None);
        let intersect = tb.button_rect(ITEMS.len() + 2);
        let center = Point2D::new(
            intersect.origin.x + intersect.size.x / 2.0,
            intersect.origin.y + intersect.size.y / 2.0,
        );
        assert_eq!(
            tb.hit_test_action(center),
            Some(AlignToolbarHit::Boolean(BooleanOp::Intersect))
        );
        let exclude = tb.button_rect(ITEMS.len() + 3);
        let center = Point2D::new(
            exclude.origin.x + exclude.size.x / 2.0,
            exclude.origin.y + exclude.size.y / 2.0,
        );
        assert_eq!(
            tb.hit_test_action(center),
            Some(AlignToolbarHit::Boolean(BooleanOp::Exclude))
        );
    }

    #[test]
    fn hit_test_maps_buttons_to_actions() {
        let doc = doc_with_n_selected(3);
        let tb = AlignToolbar::for_canvas_region(canvas(), &doc).unwrap();
        for (i, (action, _)) in ITEMS.iter().enumerate() {
            let r = tb.button_rect(i);
            let center = Point2D::new(r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0);
            assert_eq!(tb.hit_test(center), Some(*action), "button {i}");
        }
    }

    #[test]
    fn hit_test_misses_outside_toolbar() {
        let doc = doc_with_n_selected(2);
        let tb = AlignToolbar::for_canvas_region(canvas(), &doc).unwrap();
        assert_eq!(tb.hit_test(Point2D::new(-10.0, -10.0)), None);
        assert_eq!(tb.hit_test(Point2D::new(500.0, 500.0)), None);
    }

    #[test]
    fn narrow_canvas_hides_toolbar() {
        // Codex CONCERN-3: hide when canvas can't fit both the
        // align toolbar AND the vertical Toolbar reserve on the
        // left. Threshold = ALIGN_TOOLBAR_WIDTH + 56.
        let doc = doc_with_n_selected(2);
        let too_narrow = Rect::xywh(
            100.0,
            0.0,
            ALIGN_TOOLBAR_WIDTH + VERTICAL_TOOLBAR_RESERVE - 1.0,
            600.0,
        );
        assert!(AlignToolbar::for_canvas_region(too_narrow, &doc).is_none());
        // At exactly the threshold the toolbar appears, pinned to
        // canvas_left + reserve.
        let exact = Rect::xywh(
            100.0,
            0.0,
            ALIGN_TOOLBAR_WIDTH + VERTICAL_TOOLBAR_RESERVE,
            600.0,
        );
        let tb = AlignToolbar::for_canvas_region(exact, &doc).unwrap();
        assert_eq!(tb.rect.origin.x, 100.0 + VERTICAL_TOOLBAR_RESERVE);
    }

    #[test]
    fn toolbar_clamp_reserves_vertical_toolbar_column() {
        // Codex stop-gate CONCERN: the centered candidate would
        // push past the vertical Toolbar (occupying ~56 px on the
        // canvas-left edge). Clamp pins min_x to canvas_left + 56
        // so the two widgets never visually overlap. Otherwise a
        // user-visible align button could be eaten by the
        // already-painted vertical Toolbar's hit-test region.
        let doc = doc_with_n_selected(2);
        // Just enough width to require clamping. Centered candidate
        // = canvas_left + 50 (overlaps with the Toolbar column).
        let canvas = Rect::xywh(0.0, 0.0, ALIGN_TOOLBAR_WIDTH + 100.0, 600.0);
        let tb = AlignToolbar::for_canvas_region(canvas, &doc).unwrap();
        // Centered would be x=50; clamp pushes to 56.
        assert_eq!(tb.rect.origin.x, VERTICAL_TOOLBAR_RESERVE);
    }

    #[test]
    fn buttons_grouped_with_wider_gap_at_breaks() {
        let doc = doc_with_n_selected(2);
        let tb = AlignToolbar::for_canvas_region(canvas(), &doc).unwrap();
        // Distance between button 2 (Right, last in group 1) and
        // button 3 (Top, first in group 2) is GROUP_GAP, not INNER_GAP.
        let b2 = tb.button_rect(2);
        let b3 = tb.button_rect(3);
        let gap = b3.origin.x - (b2.origin.x + b2.size.x);
        assert!((gap - GROUP_GAP).abs() < 0.01);
        // Distance between button 0 (Left) and 1 (CenterH) is INNER_GAP.
        let b0 = tb.button_rect(0);
        let b1 = tb.button_rect(1);
        let gap = b1.origin.x - (b0.origin.x + b0.size.x);
        assert!((gap - INNER_GAP).abs() < 0.01);
    }
}
