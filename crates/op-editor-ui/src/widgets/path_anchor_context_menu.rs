//! Right-click context menu on a path anchor / control handle.
//!
//! Ports `apps/web/src/canvas/skia/skia-canvas.tsx:276-332`
//! (`PathAnchorContextMenu`): four rows — Corner Point / Symmetric
//! Curve / Free Curve / Reset Handles. The TS component hardcodes the
//! English labels (no i18n keys exist for them), so the Rust rows do
//! too. Opened by a Select-tool right-click that hits a path control
//! (`skia-interaction.ts:1356-1378`); a mousedown outside closes it
//! without consuming the press (TS uses a document-level listener, so
//! the underlying canvas mousedown still routes).

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
pub use jian_widgets::components::menu::MenuHit;
use op_editor_core::{EditorState, PathAnchorMenuState};

pub const PATH_ANCHOR_MENU_WIDTH: f32 = 160.0;
const ROW_HEIGHT: f32 = 26.0;
const PAD_Y: f32 = 4.0;
const ROW_FONT: f32 = 12.0;

/// One menu row's action. `Corner` / `Mirrored` / `Independent` map
/// onto the canonical `PenPathPointType`; `Reset` re-derives tangent
/// default handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathAnchorMenuAction {
    Corner,
    Mirrored,
    Independent,
    Reset,
}

/// Row order mirrors the TS `items` array (`skia-canvas.tsx:307-312`).
const ROWS: &[(PathAnchorMenuAction, &str)] = &[
    (PathAnchorMenuAction::Corner, "Corner Point"),
    (PathAnchorMenuAction::Mirrored, "Symmetric Curve"),
    (PathAnchorMenuAction::Independent, "Free Curve"),
    (PathAnchorMenuAction::Reset, "Reset Handles"),
];

pub struct PathAnchorContextMenu {
    pub theme: Theme,
    pub state: PathAnchorMenuState,
}

impl PathAnchorContextMenu {
    pub fn for_state(state: &EditorState, menu: PathAnchorMenuState) -> Self {
        Self {
            theme: theme_for(&state.editor_ui),
            state: menu,
        }
    }

    pub fn rect(&self) -> Rect {
        Rect {
            origin: Point2D::new(self.state.x, self.state.y),
            size: Point2D::new(
                PATH_ANCHOR_MENU_WIDTH,
                PAD_Y * 2.0 + ROWS.len() as f32 * ROW_HEIGHT,
            ),
        }
    }

    /// Row index the cursor is over; `None` outside the row band.
    pub fn hovered_row_at(&self, point: Point2D) -> Option<usize> {
        let rect = self.rect();
        if point.x < rect.origin.x
            || point.x > rect.origin.x + rect.size.x
            || point.y < rect.origin.y + PAD_Y
            || point.y > rect.origin.y + rect.size.y - PAD_Y
        {
            return None;
        }
        let idx = ((point.y - rect.origin.y - PAD_Y) / ROW_HEIGHT) as usize;
        (idx < ROWS.len()).then_some(idx)
    }

    /// The action under `point`; `None` when the hit is not an
    /// actionable row. Callers use `hit` to distinguish inside blank
    /// menu space from an outside dismiss.
    pub fn hit_test(&self, point: Point2D) -> Option<PathAnchorMenuAction> {
        match self.hit(point) {
            MenuHit::Row(idx) => Some(ROWS[idx].0),
            MenuHit::Inside | MenuHit::Outside => None,
        }
    }

    pub fn hit(&self, point: Point2D) -> MenuHit {
        let rect = self.rect();
        if point.x < rect.origin.x
            || point.x > rect.origin.x + rect.size.x
            || point.y < rect.origin.y
            || point.y > rect.origin.y + rect.size.y
        {
            return MenuHit::Outside;
        }
        if point.y < rect.origin.y + PAD_Y || point.y > rect.origin.y + rect.size.y - PAD_Y {
            return MenuHit::Inside;
        }
        let idx = ((point.y - rect.origin.y - PAD_Y) / ROW_HEIGHT) as usize;
        if idx < ROWS.len() {
            MenuHit::Row(idx)
        } else {
            MenuHit::Inside
        }
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>) {
        let rect = self.rect();
        cx.backend.fill_round_rect(rect, 6.0, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, 6.0, self.theme.border, 1.0);
        let mut y = rect.origin.y + PAD_Y;
        for (i, (_, label)) in ROWS.iter().enumerate() {
            if self.state.menu.hover == Some(i) {
                let hover_rect = Rect {
                    origin: Point2D::new(rect.origin.x + 4.0, y + 1.0),
                    size: Point2D::new(rect.size.x - 8.0, ROW_HEIGHT - 2.0),
                };
                cx.backend
                    .fill_round_rect(hover_rect, 4.0, self.theme.row_selected);
            }
            let fg = self.theme.foreground;
            let layout = TextLayout::single_run(
                label,
                "system-ui",
                ROW_FONT,
                jian_core::scene::Color::rgba(
                    (fg.r * 255.0) as u8,
                    (fg.g * 255.0) as u8,
                    (fg.b * 255.0) as u8,
                    255,
                ),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &layout,
                Point2D::new(rect.origin.x + 12.0, y + (ROW_HEIGHT - ROW_FONT) / 2.0),
            );
            y += ROW_HEIGHT;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::NodeId;

    fn menu() -> PathAnchorContextMenu {
        PathAnchorContextMenu {
            theme: Theme::dark(),
            state: PathAnchorMenuState {
                node_id: NodeId::new("n1"),
                anchor_index: 0,
                x: 100.0,
                y: 50.0,
                menu: Default::default(),
            },
        }
    }

    #[test]
    fn hit_test_maps_rows_in_ts_order() {
        let m = menu();
        let row_y = |i: usize| 50.0 + PAD_Y + ROW_HEIGHT * (i as f32 + 0.5);
        assert_eq!(
            m.hit_test(Point2D::new(110.0, row_y(0))),
            Some(PathAnchorMenuAction::Corner)
        );
        assert_eq!(
            m.hit_test(Point2D::new(110.0, row_y(1))),
            Some(PathAnchorMenuAction::Mirrored)
        );
        assert_eq!(
            m.hit_test(Point2D::new(110.0, row_y(2))),
            Some(PathAnchorMenuAction::Independent)
        );
        assert_eq!(
            m.hit_test(Point2D::new(110.0, row_y(3))),
            Some(PathAnchorMenuAction::Reset)
        );
    }

    #[test]
    fn outside_point_misses() {
        let m = menu();
        assert_eq!(m.hit_test(Point2D::new(50.0, 60.0)), None);
        assert_eq!(m.hit_test(Point2D::new(110.0, 300.0)), None);
    }

    #[test]
    fn hit_uses_shared_menu_state_protocol() {
        let mut m = menu();
        m.state.menu.hover = Some(2);
        assert_eq!(m.state.menu.hover, Some(2));

        let row_y = 50.0 + PAD_Y + ROW_HEIGHT * 2.5;
        assert_eq!(m.hit(Point2D::new(110.0, row_y)), MenuHit::Row(2));
        assert_eq!(m.hit(Point2D::new(110.0, 52.0)), MenuHit::Inside);
        assert_eq!(m.hit(Point2D::new(50.0, 52.0)), MenuHit::Outside);
    }
}
