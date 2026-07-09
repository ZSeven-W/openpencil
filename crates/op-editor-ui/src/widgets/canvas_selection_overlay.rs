use crate::layout_scene::{NodeKind, SceneNode};
use crate::theme::Theme;
use crate::widgets::canvas_overlay_transform::OverlayTransform;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_indicators::AgentIndicators;
use op_editor_core::Viewport;
use std::collections::HashSet;

const LABEL_FONT_SIZE: f32 = 12.0;
const LABEL_HEIGHT: f32 = 22.0;
const LABEL_PAD_X: f32 = 8.0;
const LABEL_GAP: f32 = 6.0;
const LABEL_MARGIN: f32 = 4.0;

pub(super) struct SelectionPaintInput<'a> {
    pub(super) theme: &'a Theme,
    pub(super) indicators: Option<&'a AgentIndicators>,
    pub(super) now_ms: u64,
    pub(super) canvas_rect: Rect,
    pub(super) viewport: &'a Viewport,
    pub(super) selection_label: Option<&'a str>,
}

pub(super) fn paint_selected_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    input: &SelectionPaintInput<'_>,
    show_handles: bool,
    transforms: &[OverlayTransform],
) {
    let Some(world_rect) = selection_world_rect(node, input) else {
        return;
    };
    let is_container = matches!(
        node.kind,
        NodeKind::Frame | NodeKind::Group | NodeKind::Other(_)
    );
    let transformed = super::canvas_overlay_transform::replay_on_backend(cx, transforms)
        || replay_legacy_node_rotation(cx, node, world_rect, transforms);
    super::canvas_viewport_overlay::paint_selection_overlay(
        cx,
        world_rect,
        input.theme,
        is_container,
        show_handles,
    );
    if transformed {
        cx.backend.restore();
    }
    if let (true, Some(label)) = (show_handles, input.selection_label) {
        paint_selection_label(cx, input, world_rect, label);
    }
}

fn replay_legacy_node_rotation(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    transforms: &[OverlayTransform],
) -> bool {
    if !transforms.is_empty() || node.rotation.abs() <= f32::EPSILON {
        return false;
    }
    {
        let pivot = Point2D::new(
            world_rect.origin.x + world_rect.size.x / 2.0,
            world_rect.origin.y + world_rect.size.y / 2.0,
        );
        cx.backend.save();
        cx.backend.rotate(node.rotation, pivot);
    }
    true
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
    let mut union_rect = None;
    for root in roots {
        let mut transforms = Vec::new();
        union_rect = union_optional_rects(
            union_rect,
            paint_selected_subtree(cx, root, &selected, input, &mut transforms),
        );
    }
    if let (Some(world_rect), Some(label)) = (union_rect, input.selection_label) {
        paint_selection_label(cx, input, world_rect, label);
    }
}

fn paint_selected_subtree(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    selected: &HashSet<&str>,
    input: &SelectionPaintInput<'_>,
    transforms: &mut Vec<OverlayTransform>,
) -> Option<Rect> {
    let pushed = push_node_transform(node, input, transforms);
    let mut union_rect = None;
    if selected.contains(node.id.as_str()) {
        union_rect = selection_world_rect(node, input);
        paint_selected_node(cx, node, input, false, transforms);
    }
    for child in &node.children {
        union_rect = union_optional_rects(
            union_rect,
            paint_selected_subtree(cx, child, selected, input, transforms),
        );
    }
    if pushed {
        transforms.pop();
    }
    union_rect
}

fn push_node_transform(
    node: &SceneNode,
    input: &SelectionPaintInput<'_>,
    transforms: &mut Vec<OverlayTransform>,
) -> bool {
    if !node.flip_x && !node.flip_y && node.rotation.abs() <= f32::EPSILON {
        return false;
    }
    let bounds = node.aggregate_bounds();
    let pivot = Point2D::new(
        input.canvas_rect.origin.x
            + input.viewport.pan_x
            + (bounds.origin.x + bounds.size.x / 2.0) * input.viewport.zoom,
        input.canvas_rect.origin.y
            + input.viewport.pan_y
            + (bounds.origin.y + bounds.size.y / 2.0) * input.viewport.zoom,
    );
    transforms.push(OverlayTransform {
        rotation: node.rotation,
        flip_x: node.flip_x,
        flip_y: node.flip_y,
        pivot,
    });
    true
}

fn selection_world_rect(node: &SceneNode, input: &SelectionPaintInput<'_>) -> Option<Rect> {
    if node.hidden
        || input.indicators.is_some_and(|indicators| {
            indicators
                .reveals
                .get(&node.id)
                .is_some_and(|started_at| input.now_ms < *started_at)
        })
    {
        return None;
    }
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    Some(Rect {
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
    })
}

fn union_optional_rects(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(union_rects(a, b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn union_rects(a: Rect, b: Rect) -> Rect {
    let min_x = a.origin.x.min(b.origin.x);
    let min_y = a.origin.y.min(b.origin.y);
    let max_x = (a.origin.x + a.size.x).max(b.origin.x + b.size.x);
    let max_y = (a.origin.y + a.size.y).max(b.origin.y + b.size.y);
    Rect::xywh(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn paint_selection_label(
    cx: &mut PaintCx<'_>,
    input: &SelectionPaintInput<'_>,
    world_rect: Rect,
    label: &str,
) {
    let max_label_width = (input.canvas_rect.size.x - LABEL_MARGIN * 2.0).max(32.0);
    let max_text_width = (max_label_width - LABEL_PAD_X * 2.0).max(0.0);
    let text = truncate_label_to_width(cx, label, max_text_width);
    let text_width = cx.backend.measure_text(&text, LABEL_FONT_SIZE);
    let label_width = (text_width + LABEL_PAD_X * 2.0).min(max_label_width);
    let canvas_left = input.canvas_rect.origin.x + LABEL_MARGIN;
    let canvas_top = input.canvas_rect.origin.y + LABEL_MARGIN;
    let canvas_right = input.canvas_rect.origin.x + input.canvas_rect.size.x - LABEL_MARGIN;
    let canvas_bottom = input.canvas_rect.origin.y + input.canvas_rect.size.y - LABEL_MARGIN;
    let x = clamp_to_range(
        world_rect.origin.x + world_rect.size.x / 2.0 - label_width / 2.0,
        canvas_left,
        canvas_right - label_width,
    );
    let y = clamp_to_range(
        world_rect.origin.y + world_rect.size.y + LABEL_GAP,
        canvas_top,
        canvas_bottom - LABEL_HEIGHT,
    );
    let pill = Rect::xywh(x, y, label_width, LABEL_HEIGHT);
    let bg = Color {
        a: 0.95,
        ..input.theme.card
    };
    let stroke = Color {
        a: 0.85,
        ..input.theme.primary
    };
    cx.backend.fill_round_rect(pill, 7.0, bg);
    cx.backend.stroke_round_rect(pill, 7.0, stroke, 1.0);
    let layout = TextLayout::single_run(
        &text,
        "system-ui",
        LABEL_FONT_SIZE,
        input.theme.primary.to_jian(),
        Point2D::ZERO,
    )
    .with_font_weight(500);
    cx.backend.draw_text(
        &layout,
        Point2D::new(pill.origin.x + LABEL_PAD_X, pill.origin.y + 15.0),
    );
}

fn truncate_label_to_width(cx: &mut PaintCx<'_>, label: &str, max_width: f32) -> String {
    if cx.backend.measure_text(label, LABEL_FONT_SIZE) <= max_width {
        return label.to_string();
    }
    let ellipsis = "…";
    let ellipsis_width = cx.backend.measure_text(ellipsis, LABEL_FONT_SIZE);
    if ellipsis_width >= max_width {
        return ellipsis.to_string();
    }
    let mut out = String::new();
    for ch in label.chars() {
        let mut probe = out.clone();
        probe.push(ch);
        probe.push_str(ellipsis);
        if cx.backend.measure_text(&probe, LABEL_FONT_SIZE) > max_width {
            break;
        }
        out.push(ch);
    }
    format!("{out}{ellipsis}")
}

fn clamp_to_range(value: f32, min: f32, max: f32) -> f32 {
    if max <= min {
        min
    } else {
        value.clamp(min, max)
    }
}
