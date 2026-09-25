//! The small `{ }` badge the canvas paints in the top-right corner of every
//! generator frame, so a frame whose children are program-owned reads as
//! different from a hand-built one before it is even selected.
//!
//! Screen-space and fixed-size (chrome, not content): it is an editor
//! affordance, never part of the document, so viewers and exports built
//! with `CanvasViewport::from_scene` paint none.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, PenNodeExt, Viewport};

use crate::layout_scene::SceneNode;
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

const BADGE_SIZE: f32 = 18.0;
const BADGE_INSET: f32 = 4.0;
const ICON_SIZE: f32 = 12.0;

/// Ids of every generator node on the active page, in tree order.
pub(super) fn collect_generator_badges(state: &EditorState) -> Vec<String> {
    fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
        for node in nodes {
            if op_editor_core::generator::is_generator(node) {
                out.push(node.id_str().to_string());
            }
            if let Some(children) = node.children() {
                walk(children, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(state.active_children(), &mut out);
    out
}

/// Screen rect of the badge for a node whose doc-space bounds are
/// `bounds`, or `None` when the node is too small on screen to carry one.
pub(super) fn badge_rect(bounds: Rect, viewport_origin: Point2D, zoom: f32) -> Option<Rect> {
    let width = bounds.size.x * zoom;
    let height = bounds.size.y * zoom;
    if width < BADGE_SIZE * 2.0 || height < BADGE_SIZE + BADGE_INSET * 2.0 {
        return None;
    }
    let right = viewport_origin.x + (bounds.origin.x * zoom) + width;
    let top = viewport_origin.y + bounds.origin.y * zoom;
    Some(Rect::xywh(
        right - BADGE_SIZE - BADGE_INSET,
        top + BADGE_INSET,
        BADGE_SIZE,
        BADGE_SIZE,
    ))
}

pub(super) fn paint_generator_badges(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    ids: &[String],
    theme: &Theme,
    viewport_origin: Point2D,
    viewport: &Viewport,
    clip: Rect,
) {
    for id in ids {
        let Some(node) = roots.iter().find_map(|root| find(root, id)) else {
            continue;
        };
        let Some(rect) = badge_rect(node.aggregate_bounds(), viewport_origin, viewport.zoom) else {
            continue;
        };
        if !intersects(rect, clip) {
            continue;
        }
        cx.backend.fill_round_rect(rect, 5.0, theme.primary);
        draw_icon(
            cx.backend,
            Icon::Braces,
            Point2D::new(
                rect.origin.x + (BADGE_SIZE - ICON_SIZE) / 2.0,
                rect.origin.y + (BADGE_SIZE - ICON_SIZE) / 2.0,
            ),
            ICON_SIZE,
            Color::WHITE,
            1.6,
        );
    }
}

fn find<'a>(node: &'a SceneNode, id: &str) -> Option<&'a SceneNode> {
    if node.id == id {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, id))
}

fn intersects(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.x
        && b.origin.x < a.origin.x + a.size.x
        && a.origin.y < b.origin.y + b.size.y
        && b.origin.y < a.origin.y + a.size.y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_sits_inside_the_top_right_corner_and_scales_with_zoom() {
        let bounds = Rect::xywh(100.0, 50.0, 200.0, 100.0);
        let rect = badge_rect(bounds, Point2D::new(10.0, 20.0), 2.0).expect("badge");
        assert_eq!(
            rect.origin.x,
            10.0 + 200.0 + 400.0 - BADGE_SIZE - BADGE_INSET
        );
        assert_eq!(rect.origin.y, 20.0 + 100.0 + BADGE_INSET);
        assert_eq!(rect.size.x, BADGE_SIZE);
    }

    #[test]
    fn tiny_frames_get_no_badge() {
        assert!(badge_rect(Rect::xywh(0.0, 0.0, 30.0, 30.0), Point2D::ZERO, 0.5).is_none());
    }

    #[test]
    fn only_generator_nodes_are_collected() {
        let mut state = EditorState::new();
        fn one(
            _: &op_editor_core::generator::GeneratorRequest<'_>,
        ) -> Result<Vec<PenNode>, op_editor_core::generator::GeneratorError> {
            Ok(vec![serde_json::from_value(serde_json::json!({
                "type": "frame", "id": "x", "width": 10, "height": 10
            }))
            .unwrap()])
        }
        let id = state
            .insert_generator_starter(&op_editor_core::generator::GENERATOR_STARTERS[1], Some(one))
            .unwrap();
        assert_eq!(collect_generator_badges(&state), vec![id.to_string()]);
    }
}
