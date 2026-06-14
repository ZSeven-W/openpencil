use crate::layout_scene::{NodeKind, SceneNode};
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::agent_indicators::AgentIndicators;
use op_editor_core::Viewport;
use std::collections::HashSet;

pub(super) struct SelectionPaintInput<'a> {
    pub(super) theme: &'a Theme,
    pub(super) indicators: Option<&'a AgentIndicators>,
    pub(super) now_ms: u64,
    pub(super) canvas_rect: Rect,
    pub(super) viewport: &'a Viewport,
}

pub(super) fn paint_selected_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    input: &SelectionPaintInput<'_>,
    show_handles: bool,
) {
    if node.hidden
        || input.indicators.is_some_and(|indicators| {
            indicators
                .reveals
                .get(&node.id)
                .is_some_and(|started_at| input.now_ms < *started_at)
        })
    {
        return;
    }
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return;
    }
    let world_rect = Rect {
        origin: Point2D::new(
            input.canvas_rect.origin.x
                + input.viewport.pan_x
                + bounds.origin.x * input.viewport.zoom,
            input.canvas_rect.origin.y
                + input.viewport.pan_y
                + bounds.origin.y * input.viewport.zoom,
        ),
        size: Point2D::new(
            bounds.size.x * input.viewport.zoom,
            bounds.size.y * input.viewport.zoom,
        ),
    };
    let is_container = matches!(
        node.kind,
        NodeKind::Frame | NodeKind::Group | NodeKind::Other(_)
    );
    let rotated = node.rotation.abs() > f32::EPSILON;
    if rotated {
        let pivot = Point2D::new(
            world_rect.origin.x + world_rect.size.x / 2.0,
            world_rect.origin.y + world_rect.size.y / 2.0,
        );
        cx.backend.save();
        cx.backend.rotate(node.rotation, pivot);
    }
    super::canvas_viewport_overlay::paint_selection_overlay(
        cx,
        world_rect,
        input.theme,
        is_container,
        show_handles,
    );
    if rotated {
        cx.backend.restore();
    }
}

pub(super) fn paint_multi_selection_overlays(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    selected_ids: &[String],
    input: &SelectionPaintInput<'_>,
) {
    if selected_ids.is_empty() {
        return;
    }
    let selected: HashSet<&str> = selected_ids.iter().map(String::as_str).collect();
    for root in roots {
        paint_selected_subtree(cx, root, &selected, input);
    }
}

fn paint_selected_subtree(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    selected: &HashSet<&str>,
    input: &SelectionPaintInput<'_>,
) {
    if selected.contains(node.id.as_str()) {
        paint_selected_node(cx, node, input, false);
    }
    for child in &node.children {
        paint_selected_subtree(cx, child, selected, input);
    }
}
