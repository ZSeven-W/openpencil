use crate::layout_scene::SceneNode;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::Viewport;

const LABEL_SIDE_PADDING: f32 = 4.0;
const LABEL_FONT_SIZE: f32 = 12.0;
const GENERATING_ICON_SIZE: f32 = 14.0;
const GENERATING_ICON_GAP: f32 = 4.0;

/// `draw_text` takes a baseline while icons take a top-left corner. Keep the
/// existing frame-label baseline fixed and align the icon with the text line's
/// visual center using the same cap-height approximation as
/// `jian_widgets::centered_text_baseline_y`.
fn generating_icon_top(text_baseline_y: f32) -> f32 {
    let text_center_y = text_baseline_y - LABEL_FONT_SIZE * 0.35;
    text_center_y - GENERATING_ICON_SIZE / 2.0
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FrameLabel {
    pub(super) id: String,
    pub(super) text: String,
    pub(super) color: Color,
    pub(super) generating: bool,
}

impl FrameLabel {
    pub(super) fn new(
        id: impl Into<String>,
        text: impl Into<String>,
        color: Color,
        generating: bool,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            color,
            generating,
        }
    }
}

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

fn label_width(label: &FrameLabel) -> f32 {
    let text_width = (label.text.chars().count() as f32 * 7.0).max(1.0);
    if label.generating {
        GENERATING_ICON_SIZE + GENERATING_ICON_GAP + text_width
    } else {
        text_width
    }
}

fn label_rect(
    node: &SceneNode,
    label: &FrameLabel,
    viewport_origin: Point2D,
    viewport: &Viewport,
) -> Rect {
    let b = node.aggregate_bounds();
    let sx = viewport_origin.x + b.origin.x * viewport.zoom;
    let sy = viewport_origin.y + b.origin.y * viewport.zoom;
    Rect::xywh(
        sx - LABEL_SIDE_PADDING,
        sy - 32.0,
        label_width(label) + LABEL_SIDE_PADDING * 2.0,
        28.0,
    )
}

pub(super) fn frame_label_at_point(
    roots: &[SceneNode],
    labels: &[FrameLabel],
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
        let Some(label) = labels.peek() else {
            break;
        };
        if !label_id_matches(&node.id, &label.id) {
            continue;
        }
        let label = labels.next().expect("peeked label exists");
        if (label_rect(node, label, viewport_origin, viewport)).contains(point) {
            return Some(label.id.clone());
        }
    }
    None
}

pub(super) fn paint_frame_labels(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    labels: &[FrameLabel],
    hidden_ids: &[String],
    viewport_origin: Point2D,
    viewport: &Viewport,
    clip: Rect,
) {
    let mut labels = labels.iter().peekable();
    for node in roots {
        let Some(label) = labels.peek() else {
            break;
        };
        if !label_id_matches(&node.id, &label.id) {
            continue;
        }
        let label = labels.next().expect("peeked label exists");
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
        let hit_rect = label_rect(node, label, viewport_origin, viewport);
        let layout = TextLayout::single_run(
            &label.text,
            "system-ui",
            LABEL_FONT_SIZE,
            jian_core::scene::Color::rgba(
                (label.color.r * 255.0) as u8,
                (label.color.g * 255.0) as u8,
                (label.color.b * 255.0) as u8,
                255,
            ),
            Point2D::ZERO,
        )
        .with_font_weight(500);
        let text_baseline_y = sy - 18.0;
        let mut text_x = hit_rect.origin.x + LABEL_SIDE_PADDING;
        if label.generating {
            draw_icon(
                cx.backend,
                Icon::Sparkles,
                Point2D::new(text_x, generating_icon_top(text_baseline_y)),
                GENERATING_ICON_SIZE,
                label.color,
                1.5,
            );
            text_x += GENERATING_ICON_SIZE + GENERATING_ICON_GAP;
        }
        cx.backend
            .draw_text(&layout, Point2D::new(text_x, text_baseline_y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generating_icon_center_matches_text_line_center() {
        let baseline_y = 40.0;
        let icon_center_y = generating_icon_top(baseline_y) + GENERATING_ICON_SIZE / 2.0;
        let text_center_y = baseline_y - LABEL_FONT_SIZE * 0.35;
        assert!((icon_center_y - text_center_y).abs() < f32::EPSILON);
    }
}
