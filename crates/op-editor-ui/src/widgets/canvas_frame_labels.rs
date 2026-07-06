use crate::layout_scene::SceneNode;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::Viewport;

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static LABEL_MATCH_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_label_match_count() {
    LABEL_MATCH_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn label_match_count() -> usize {
    LABEL_MATCH_COUNT.with(Cell::get)
}

fn label_id_matches(node_id: &str, label_id: &str) -> bool {
    #[cfg(test)]
    LABEL_MATCH_COUNT.with(|count| count.set(count.get() + 1));

    node_id == label_id
}

fn label_width(name: &str) -> f32 {
    (name.chars().count() as f32 * 7.0).max(1.0)
}

fn label_rect(node: &SceneNode, name: &str, viewport_origin: Point2D, viewport: &Viewport) -> Rect {
    let b = node.aggregate_bounds();
    let sx = viewport_origin.x + b.origin.x * viewport.zoom;
    let sy = viewport_origin.y + b.origin.y * viewport.zoom;
    Rect::xywh(sx - 4.0, sy - 32.0, label_width(name) + 8.0, 28.0)
}

pub(super) fn frame_label_at_point(
    roots: &[SceneNode],
    labels: &[(String, String, Color)],
    viewport_origin: Point2D,
    viewport: &Viewport,
    clip: Rect,
    point: Point2D,
) -> Option<String> {
    if !(clip).contains(point) {
        return None;
    }
    let mut labels = labels.iter().peekable();
    for node in roots {
        let Some((id, _, _)) = labels.peek() else {
            break;
        };
        if !label_id_matches(&node.id, id) {
            continue;
        }
        let (id, name, _) = labels.next().expect("peeked label exists");
        if (label_rect(node, name, viewport_origin, viewport)).contains(point) {
            return Some(id.clone());
        }
    }
    None
}

pub(super) fn paint_frame_labels(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    labels: &[(String, String, Color)],
    hidden_ids: &[String],
    viewport_origin: Point2D,
    viewport: &Viewport,
    clip: Rect,
) {
    let mut labels = labels.iter().peekable();
    for node in roots {
        let Some((id, _, _)) = labels.peek() else {
            break;
        };
        if !label_id_matches(&node.id, id) {
            continue;
        }
        let (_, name, color) = labels.next().expect("peeked label exists");
        if hidden_ids.iter().any(|hidden| hidden == &node.id) {
            continue;
        }
        let b = node.aggregate_bounds();
        let sx = viewport_origin.x + b.origin.x * viewport.zoom;
        let sy = viewport_origin.y + b.origin.y * viewport.zoom;
        if sy < clip.origin.y || sy > clip.origin.y + clip.size.y + 18.0 {
            continue;
        }
        if sx > clip.origin.x + clip.size.x || sx + 600.0 < clip.origin.x {
            continue;
        }
        let hit_rect = label_rect(node, name, viewport_origin, viewport);
        let layout = TextLayout::single_run(
            name,
            "system-ui",
            12.0,
            jian_core::scene::Color::rgba(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
                255,
            ),
            Point2D::ZERO,
        )
        .with_font_weight(500);
        cx.backend
            .draw_text(&layout, Point2D::new(hit_rect.origin.x + 4.0, sy - 18.0));
    }
}
