//! Comment pins — the canvas markers for comment threads.
//!
//! A pin is a speech-bubble disc whose bottom-left tip sits on the
//! thread's anchor: a node anchor follows the node through the layout
//! scene, a point anchor stays at its document coordinates. Pins paint
//! above the canvas content (outside the pan cache, so they track the
//! live viewport) and below every floating panel; their press tier is
//! the first thing the canvas tier checks, ahead of node selection.
//!
//! Archived threads have no pin (they are history), and a thread whose
//! node was deleted is left in the document but not drawn.

use op_editor_core::comment_ui::CommentPopover;
use op_editor_core::comments::{CanvasComment, CommentLookup, CommentStatus};
use op_editor_core::EditorState;

use crate::layout_scene::LayoutScene;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::host_canvas_geometry::{canvas_origin, canvas_rect};
use crate::widgets::{text_metrics, PaintCx};
use crate::{Color, Point2D, Rect, TextLayout};

/// Pin disc diameter in screen px.
pub const PIN_SIZE: f32 = 24.0;
const PIN_FONT_SIZE: f32 = 11.0;

/// One pin resolved to screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct CommentPin {
    pub id: String,
    /// 1-based position among the document's threads — the pin label.
    pub number: usize,
    pub status: CommentStatus,
    /// The anchor point the tip sits on.
    pub tip: Point2D,
    /// The disc's bounds (also the hit rect).
    pub rect: Rect,
}

/// Document-space anchor point of a thread on the active page, or `None`
/// when it is archived, on another page, or its node is gone.
pub fn comment_doc_point(
    state: &EditorState,
    scene: &LayoutScene,
    comment: &CanvasComment,
) -> Option<Point2D> {
    if comment.status == CommentStatus::Archived {
        return None;
    }
    if state.comment_page_index(comment)? != state.ui.active_page_index {
        return None;
    }
    let anchor = &comment.anchor;
    let x = anchor.x.map(|v| v as f32);
    let y = anchor.y.map(|v| v as f32);
    match anchor.node_id.as_deref() {
        Some(node_id) => {
            let node = scene.active_page()?.find(node_id)?;
            if node.hidden {
                return None;
            }
            let b = node.aggregate_bounds();
            Some(match (x, y) {
                (Some(dx), Some(dy)) => Point2D::new(b.origin.x + dx, b.origin.y + dy),
                _ => Point2D::new(b.origin.x + b.size.x, b.origin.y),
            })
        }
        None => Some(Point2D::new(x?, y?)),
    }
}

/// Every visible pin, in paint order (later pins paint on top).
pub fn comment_pins(
    state: &EditorState,
    scene: &LayoutScene,
    viewport_width: f32,
    viewport_height: f32,
) -> Vec<CommentPin> {
    if state.editor_ui.preview.mode {
        return Vec::new();
    }
    let canvas = canvas_rect(state, viewport_width, viewport_height);
    state
        .comments()
        .iter()
        .enumerate()
        .filter_map(|(index, comment)| {
            let doc = comment_doc_point(state, scene, comment)?;
            let tip = doc_to_screen(state, doc);
            let rect = Rect::xywh(tip.x, tip.y - PIN_SIZE, PIN_SIZE, PIN_SIZE);
            canvas.contains(tip).then(|| CommentPin {
                id: comment.id.clone(),
                number: index + 1,
                status: comment.status,
                tip,
                rect,
            })
        })
        .collect()
}

/// Screen tip of the open popover's anchor — the open thread's pin, or a
/// draft's anchor point.
pub fn open_popover_tip(
    state: &EditorState,
    scene: &LayoutScene,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<Point2D> {
    let open = state.editor_ui.comments.open.as_ref()?;
    // Kept for signature symmetry with `comment_pins`: the popover tip is
    // not clipped to the canvas, the popover clamps itself instead.
    let _ = (viewport_width, viewport_height);
    let doc = match open {
        CommentPopover::Thread { id } => {
            let comment = state.comment(id)?;
            // An archived thread reopened from history has no pin; anchor
            // its popover where the pin would be.
            let mut live = comment.clone();
            live.status = CommentStatus::Pending;
            comment_doc_point(state, scene, &live)?
        }
        CommentPopover::Draft { anchor } => {
            let probe = CanvasComment {
                id: String::new(),
                status: CommentStatus::Pending,
                anchor: anchor.clone(),
                messages: Vec::new(),
                updated_at: None,
            };
            comment_doc_point(state, scene, &probe)?
        }
    };
    Some(doc_to_screen(state, doc))
}

/// Document → screen, the inverse of `canvas_doc_point_unclamped`.
pub fn doc_to_screen(state: &EditorState, doc: Point2D) -> Point2D {
    let (ox, oy) = canvas_origin(state);
    let vp = &state.viewport;
    Point2D::new(
        ox + vp.pan_x + doc.x * vp.zoom,
        oy + vp.pan_y + doc.y * vp.zoom,
    )
}

/// The top-most pin under `point`.
pub fn pin_at(pins: &[CommentPin], point: Point2D) -> Option<&CommentPin> {
    pins.iter().rev().find(|pin| pin.rect.contains(point))
}

/// Paint every pin plus the draft marker. Clipped to the canvas.
pub fn paint_comment_pins(
    cx: &mut PaintCx<'_>,
    state: &EditorState,
    scene: &LayoutScene,
    viewport_width: f32,
    viewport_height: f32,
) {
    let pins = comment_pins(state, scene, viewport_width, viewport_height);
    let draft_tip = match state.editor_ui.comments.open {
        Some(CommentPopover::Draft { .. }) => {
            open_popover_tip(state, scene, viewport_width, viewport_height)
        }
        _ => None,
    };
    if pins.is_empty() && draft_tip.is_none() {
        return;
    }
    let theme = theme_for(&state.editor_ui);
    let open_id = match &state.editor_ui.comments.open {
        Some(CommentPopover::Thread { id }) => Some(id.as_str()),
        _ => None,
    };
    cx.backend.save();
    cx.backend
        .clip_rect(canvas_rect(state, viewport_width, viewport_height));
    for pin in &pins {
        let fill = match pin.status {
            CommentStatus::Addressed => theme.status_success,
            _ => theme.primary,
        };
        paint_pin(
            cx,
            pin.rect,
            fill,
            theme.primary_foreground,
            &pin.number.to_string(),
        );
        if open_id == Some(pin.id.as_str()) {
            let ring = Rect::xywh(
                pin.rect.origin.x - 3.0,
                pin.rect.origin.y - 3.0,
                PIN_SIZE + 6.0,
                PIN_SIZE + 6.0,
            );
            cx.backend.stroke_oval(ring, theme.ring, 2.0);
        }
    }
    if let Some(tip) = draft_tip {
        let rect = Rect::xywh(tip.x, tip.y - PIN_SIZE, PIN_SIZE, PIN_SIZE);
        paint_pin(cx, rect, theme.muted_foreground, theme.background, "+");
    }
    cx.backend.restore();
}

fn paint_pin(cx: &mut PaintCx<'_>, rect: Rect, fill: Color, text_color: Color, label: &str) {
    // Round on three corners, square at the bottom-left tip.
    let r = PIN_SIZE / 2.0;
    cx.backend
        .fill_round_rect_per_corner(rect, [r, r, r, 0.0], fill);
    cx.backend.stroke_round_rect_per_corner(
        rect,
        [r, r, r, 0.0],
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.9,
        },
        1.5,
    );
    let x = text_metrics::centered_text_x(cx.backend, label, PIN_FONT_SIZE, rect);
    let baseline = jian_widgets::centered_text_baseline_y(rect, PIN_FONT_SIZE);
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        PIN_FONT_SIZE,
        text_color.to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(600);
    cx.backend.draw_text(&layout, Point2D::new(x, baseline));
}
