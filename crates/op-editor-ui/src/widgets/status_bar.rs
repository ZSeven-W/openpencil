//! `StatusBar` — floating zoom widget pinned to the bottom-right
//! corner of the canvas (Step 4 visual lift).
//!
//! Mirrors the TS app's `StatusBar` (apps/web/src/components/editor/
//! status-bar.tsx): a pill-shape with Search / Minus / "100%" / Plus.
//! Step 4 paints only — the controls don't actually adjust zoom yet.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

pub const STATUS_BAR_WIDTH: f32 = 134.0;
pub const STATUS_BAR_HEIGHT: f32 = 32.0;
const ICON_SIZE: f32 = 14.0;
const SIDE_PAD: f32 = 14.0;
/// Gap between the search icon and the `[- N% +]` cluster — the
/// section divider, wider than within-cluster spacing so the eye
/// reads them as separate groups.
const SECTION_GAP: f32 = 18.0;
/// Gap inside the zoom cluster.
const CLUSTER_GAP: f32 = 8.0;

pub struct StatusBar {
    pub id: WidgetId,
    pub zoom_percent: u32,
    pub theme: Theme,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(6000),
            zoom_percent: 100,
            theme: Theme::dark(),
        }
    }

    pub fn with_zoom(zoom_percent: u32) -> Self {
        Self {
            zoom_percent,
            ..Self::new()
        }
    }

    /// Build the bar with theme + zoom from the editor state.
    pub fn for_editor(state: &op_editor_core::EditorState) -> Self {
        let zoom = (state.viewport.zoom * 100.0).round() as u32;
        Self {
            id: WidgetId::new(6000),
            zoom_percent: zoom.max(1),
            theme: crate::widgets::editor_state_ext::theme_for(&state.editor_ui),
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for StatusBar {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(STATUS_BAR_WIDTH, STATUS_BAR_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        // Pill background.
        cx.backend
            .fill_round_rect(rect, STATUS_BAR_HEIGHT / 2.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(rect, STATUS_BAR_HEIGHT / 2.0, self.theme.border, 1.0);

        let center_y = rect.origin.y + rect.size.y / 2.0;
        let icon_y = center_y - ICON_SIZE / 2.0;
        // Single tight row left→right: Search · - · 100% · +.
        // No giant gap between the search icon and the zoom
        // cluster (matches TS app's StatusBar density).
        let mut x = rect.origin.x + SIDE_PAD;
        draw_icon(
            cx.backend,
            Icon::Search,
            Point2D::new(x, icon_y),
            ICON_SIZE,
            self.theme.muted_foreground,
            1.4,
        );
        x += ICON_SIZE + SECTION_GAP;
        draw_icon(
            cx.backend,
            Icon::Minus,
            Point2D::new(x, icon_y),
            ICON_SIZE,
            self.theme.muted_foreground,
            1.4,
        );
        x += ICON_SIZE + CLUSTER_GAP;
        let zoom_text = format!("{}%", self.zoom_percent);
        let label = TextLayout::single_run(
            &zoom_text,
            "system-ui",
            12.0,
            to_jian_color(self.theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&label, Point2D::new(x, center_y + 4.5));
        x += zoom_text.chars().count() as f32 * 7.5 + CLUSTER_GAP;
        draw_icon(
            cx.backend,
            Icon::Plus,
            Point2D::new(x, icon_y),
            ICON_SIZE,
            self.theme.muted_foreground,
            1.4,
        );
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label("Zoom controls");
        node
    }
}

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_zoom_is_100() {
        let bar = StatusBar::new();
        assert_eq!(bar.zoom_percent, 100);
    }

    #[test]
    fn with_zoom_overrides() {
        let bar = StatusBar::with_zoom(150);
        assert_eq!(bar.zoom_percent, 150);
    }

    #[test]
    fn layout_reports_pill_size() {
        let cx = LayoutCx {
            available_width: 9999.0,
            dpi: 1.0,
        };
        let lb = StatusBar::new().layout(&cx);
        assert_eq!(lb.rect.size.x, STATUS_BAR_WIDTH);
        assert_eq!(lb.rect.size.y, STATUS_BAR_HEIGHT);
    }
}
