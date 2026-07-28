//! Collaboration cursor and remote-selection paint.
//!
//! Presence is deliberately lossy, bounded in `op-editor-core`, and painted
//! below local selection chrome. Unknown/deleted ids are skipped each frame.

use crate::layout_scene::SceneNode;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::{CollabUiState, Viewport};

const CURSOR_SIZE: f32 = 12.0;
const LABEL_FONT_SIZE: f32 = 10.0;
const LABEL_HEIGHT: f32 = 19.0;
const LABEL_PAD_X: f32 = 6.0;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CollabPresencePaint {
    participant_key: String,
    display_name: String,
    color: Color,
    cursor: Option<Point2D>,
    selection: Vec<String>,
}

pub(super) fn snapshot(state: &CollabUiState) -> Vec<CollabPresencePaint> {
    state
        .presence()
        .iter()
        .filter_map(|presence| {
            let participant = state
                .participants()
                .iter()
                .find(|participant| participant.participant_key == presence.participant_key)?;
            if participant.is_self {
                return None;
            }
            Some(CollabPresencePaint {
                participant_key: participant.participant_key.clone(),
                display_name: participant.display_name.clone(),
                color: rgba_u32(participant.color_rgba),
                cursor: presence
                    .cursor
                    .map(|point| Point2D::new(point.x as f32, point.y as f32)),
                selection: presence.selection.as_ref().clone(),
            })
        })
        .collect()
}

pub(super) fn paint(
    cx: &mut PaintCx<'_>,
    roots: &[SceneNode],
    items: &[CollabPresencePaint],
    canvas_rect: Rect,
    viewport: &Viewport,
) {
    for item in items {
        let outline = item.color.with_alpha(0.9);
        for id in &item.selection {
            let Some(node) = find_node(roots, id) else {
                continue;
            };
            if node.hidden {
                continue;
            }
            let bounds = node.aggregate_bounds();
            if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
                continue;
            }
            let rect = doc_rect_to_screen(bounds, canvas_rect, viewport);
            stroke_outline(cx, rect, outline);
        }
        if let Some(cursor) = item.cursor {
            paint_cursor(cx, item, doc_point_to_screen(cursor, canvas_rect, viewport));
        }
    }
}

fn find_node<'a>(roots: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
    for root in roots {
        if root.id == id {
            return Some(root);
        }
        if let Some(node) = find_node(&root.children, id) {
            return Some(node);
        }
    }
    None
}

fn doc_rect_to_screen(rect: Rect, canvas_rect: Rect, viewport: &Viewport) -> Rect {
    Rect::xywh(
        canvas_rect.origin.x + viewport.pan_x + rect.origin.x * viewport.zoom,
        canvas_rect.origin.y + viewport.pan_y + rect.origin.y * viewport.zoom,
        rect.size.x * viewport.zoom,
        rect.size.y * viewport.zoom,
    )
}

fn doc_point_to_screen(point: Point2D, canvas_rect: Rect, viewport: &Viewport) -> Point2D {
    Point2D::new(
        canvas_rect.origin.x + viewport.pan_x + point.x * viewport.zoom,
        canvas_rect.origin.y + viewport.pan_y + point.y * viewport.zoom,
    )
}

fn stroke_outline(cx: &mut PaintCx<'_>, rect: Rect, color: Color) {
    let left = rect.origin.x;
    let top = rect.origin.y;
    let right = rect.origin.x + rect.size.x;
    let bottom = rect.origin.y + rect.size.y;
    cx.backend.stroke_line(
        Point2D::new(left, top),
        Point2D::new(right, top),
        color,
        1.5,
    );
    cx.backend.stroke_line(
        Point2D::new(right, top),
        Point2D::new(right, bottom),
        color,
        1.5,
    );
    cx.backend.stroke_line(
        Point2D::new(right, bottom),
        Point2D::new(left, bottom),
        color,
        1.5,
    );
    cx.backend.stroke_line(
        Point2D::new(left, bottom),
        Point2D::new(left, top),
        color,
        1.5,
    );
}

fn paint_cursor(cx: &mut PaintCx<'_>, item: &CollabPresencePaint, point: Point2D) {
    let cursor = [
        point,
        Point2D::new(point.x + CURSOR_SIZE * 0.78, point.y + CURSOR_SIZE),
        Point2D::new(point.x + CURSOR_SIZE * 0.35, point.y + CURSOR_SIZE * 0.82),
    ];
    cx.backend.fill_polygon(&cursor, item.color);
    cx.backend
        .stroke_line(cursor[0], cursor[1], Color::WHITE.with_alpha(0.8), 0.75);

    let text_width = cx.backend.measure_text(&item.display_name, LABEL_FONT_SIZE);
    let label = Rect::xywh(
        point.x + CURSOR_SIZE * 0.6,
        point.y + CURSOR_SIZE,
        text_width + LABEL_PAD_X * 2.0,
        LABEL_HEIGHT,
    );
    cx.backend.fill_round_rect(label, 5.0, item.color);
    let layout = TextLayout::single_run(
        &item.display_name,
        "system-ui",
        LABEL_FONT_SIZE,
        Color::WHITE.to_jian(),
        Point2D::ZERO,
    )
    .with_font_weight(500);
    cx.backend.draw_text(
        &layout,
        Point2D::new(label.origin.x + LABEL_PAD_X, label.origin.y + 13.0),
    );
}

fn rgba_u32(value: u32) -> Color {
    Color {
        r: ((value >> 24) & 0xff) as f32 / 255.0,
        g: ((value >> 16) & 0xff) as f32 / 255.0,
        b: ((value >> 8) & 0xff) as f32 / 255.0,
        a: (value & 0xff) as f32 / 255.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::{
        AuthenticatedCollabSession, CollabCanvasPoint, CollabConnectionPhase, CollabParticipantUi,
        CollabUiRole, RemotePresenceUi,
    };

    #[test]
    fn snapshot_excludes_self_and_keeps_epoch_local_remote_identity() {
        let mut state = CollabUiState::default();
        state.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Design".into(),
                role: CollabUiRole::Owner,
                share_endpoint: None,
            },
            vec![
                CollabParticipantUi::new("self", "Ada", 0x112233ff, CollabUiRole::Owner, true),
                CollabParticipantUi::new(
                    "remote",
                    "Grace",
                    0x445566ff,
                    CollabUiRole::Editor,
                    false,
                ),
            ],
        );
        state.queue_presence_snapshot(vec![
            RemotePresenceUi::bounded(
                "self",
                Some(CollabCanvasPoint { x: 1.0, y: 2.0 }),
                ["n1".into()],
                None,
                1,
            ),
            RemotePresenceUi::bounded(
                "remote",
                Some(CollabCanvasPoint { x: 3.0, y: 4.0 }),
                ["n2".into()],
                None,
                1,
            ),
        ]);
        assert!(state.flush_presence(100));

        let items = snapshot(&state);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].participant_key, "remote");
        assert_eq!(items[0].display_name, "Grace");
        assert_eq!(items[0].selection, ["n2"]);
    }
}
