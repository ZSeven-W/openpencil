//! `StatusBar` — floating zoom widget pinned to the bottom-right
//! corner of the canvas (Step 4 visual lift).
//!
//! Mirrors the TS app's `StatusBar` (apps/web/src/components/editor/
//! status-bar.tsx): a pill-shape with Search / Minus / "100%" / Plus.
//! Search frames the page content; the `[-]` / `[+]` glyphs step the
//! zoom. [`StatusBar::control_at`] resolves a pointer to a control for
//! both the click dispatch and the per-control `theme.button_hover`
//! hover wash.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::StatusBarButton;

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
/// Width of the left "search" section, used as its click + hover
/// target. Generous so an edge click on the pill's left still frames
/// content (mirrors the host's historical 38 px search target).
const SEARCH_SECTION_W: f32 = 38.0;
/// Half-side of the square click + hover target around the zoom
/// minus / plus glyphs.
const ZOOM_HIT_HALF: f32 = 12.0;

pub struct StatusBar {
    pub id: WidgetId,
    pub zoom_percent: u32,
    pub theme: Theme,
    /// Which control the cursor is over — drives the per-control
    /// `theme.button_hover` wash. `None` = no hover.
    pub hover: Option<StatusBarButton>,
    pub pressed: Option<StatusBarButton>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(6000),
            zoom_percent: 100,
            theme: Theme::dark(),
            hover: None,
            pressed: None,
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
            hover: state.editor_ui.statusbar_hover,
            pressed: match state.editor_ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::StatusBar(button)) => Some(button),
                _ => None,
            },
        }
    }

    /// Top-left x of the search / minus / plus glyphs for a pill at
    /// `rect`. Shared by paint + [`control_at`] so the hover wash and
    /// the click target can never drift from the painted glyphs. The
    /// plus x trails the zoom label, whose width tracks `zoom_percent`.
    fn glyph_x(&self, rect: Rect) -> (f32, f32, f32) {
        let search_x = rect.origin.x + SIDE_PAD;
        let minus_x = search_x + ICON_SIZE + SECTION_GAP;
        let text_x = minus_x + ICON_SIZE + CLUSTER_GAP;
        let zoom_text = format!("{}%", self.zoom_percent);
        let plus_x = text_x + zoom_text.chars().count() as f32 * 7.5 + CLUSTER_GAP;
        (search_x, minus_x, plus_x)
    }

    /// Resolve a pointer at `point` to a control. The search section
    /// is the generous left band; the zoom minus / plus are square
    /// targets around their glyphs.
    pub fn control_at(&self, rect: Rect, point: Point2D) -> Option<StatusBarButton> {
        if point.x < rect.origin.x
            || point.x > rect.origin.x + rect.size.x
            || point.y < rect.origin.y
            || point.y > rect.origin.y + rect.size.y
        {
            return None;
        }
        if point.x <= rect.origin.x + SEARCH_SECTION_W {
            return Some(StatusBarButton::Search);
        }
        let cy = rect.origin.y + rect.size.y / 2.0;
        let (_search_x, minus_x, plus_x) = self.glyph_x(rect);
        if zoom_hit(minus_x + ICON_SIZE / 2.0, cy, point) {
            return Some(StatusBarButton::ZoomOut);
        }
        if zoom_hit(plus_x + ICON_SIZE / 2.0, cy, point) {
            return Some(StatusBarButton::ZoomIn);
        }
        None
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
        let (search_x, minus_x, plus_x) = self.glyph_x(rect);
        let search_color = paint_control_bg(
            cx,
            &self.theme,
            search_x,
            center_y,
            StatusBarButton::Search,
            self.hover,
            self.pressed,
        );
        draw_icon(
            cx.backend,
            Icon::Search,
            Point2D::new(search_x, icon_y),
            ICON_SIZE,
            search_color,
            1.4,
        );
        let minus_color = paint_control_bg(
            cx,
            &self.theme,
            minus_x,
            center_y,
            StatusBarButton::ZoomOut,
            self.hover,
            self.pressed,
        );
        draw_icon(
            cx.backend,
            Icon::Minus,
            Point2D::new(minus_x, icon_y),
            ICON_SIZE,
            minus_color,
            1.4,
        );
        let text_x = minus_x + ICON_SIZE + CLUSTER_GAP;
        let zoom_text = format!("{}%", self.zoom_percent);
        let label = TextLayout::single_run(
            &zoom_text,
            "system-ui",
            12.0,
            (self.theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&label, Point2D::new(text_x, center_y + 4.5));
        let plus_color = paint_control_bg(
            cx,
            &self.theme,
            plus_x,
            center_y,
            StatusBarButton::ZoomIn,
            self.hover,
            self.pressed,
        );
        draw_icon(
            cx.backend,
            Icon::Plus,
            Point2D::new(plus_x, icon_y),
            ICON_SIZE,
            plus_color,
            1.4,
        );
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label("Zoom controls");
        node
    }
}

/// True when `point` lands inside the square zoom target centred on
/// `(cx, cy)`. Mirrors the `ZOOM_HIT_HALF` slop used for the hover wash.
fn zoom_hit(cx: f32, cy: f32, point: Point2D) -> bool {
    (point.x - cx).abs() <= ZOOM_HIT_HALF && (point.y - cy).abs() <= ZOOM_HIT_HALF
}

/// Paint the `theme.button_hover` wash behind a status-bar control
/// when the cursor rests on it, returning the glyph color: foreground
/// while hovered, muted otherwise. The wash is a square centred on the
/// glyph (the search section's larger click band still triggers it).
fn paint_control_bg(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    glyph_x: f32,
    center_y: f32,
    control: StatusBarButton,
    hover: Option<StatusBarButton>,
    pressed: Option<StatusBarButton>,
) -> Color {
    let cx_center = glyph_x + ICON_SIZE / 2.0;
    let r = Rect {
        origin: Point2D::new(cx_center - ZOOM_HIT_HALF, center_y - ZOOM_HIT_HALF),
        size: Point2D::new(ZOOM_HIT_HALF * 2.0, ZOOM_HIT_HALF * 2.0),
    };
    crate::widgets::button::paint_ghost_button_feedback(
        cx.backend,
        theme,
        r,
        hover == Some(control),
        pressed == Some(control),
    )
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
    fn control_at_resolves_search_minus_plus() {
        let bar = StatusBar::with_zoom(100);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(STATUS_BAR_WIDTH, STATUS_BAR_HEIGHT),
        };
        let cy = STATUS_BAR_HEIGHT / 2.0;
        // Generous left band → Search (mirrors the host's old 38 px target).
        assert_eq!(
            bar.control_at(rect, Point2D::new(5.0, cy)),
            Some(StatusBarButton::Search)
        );
        let (_search_x, minus_x, plus_x) = bar.glyph_x(rect);
        assert_eq!(
            bar.control_at(rect, Point2D::new(minus_x + ICON_SIZE / 2.0, cy)),
            Some(StatusBarButton::ZoomOut)
        );
        assert_eq!(
            bar.control_at(rect, Point2D::new(plus_x + ICON_SIZE / 2.0, cy)),
            Some(StatusBarButton::ZoomIn)
        );
    }

    #[test]
    fn for_editor_picks_up_hover() {
        let mut state = op_editor_core::EditorState::new();
        state.editor_ui.statusbar_hover = Some(StatusBarButton::ZoomIn);
        let bar = StatusBar::for_editor(&state);
        assert_eq!(bar.hover, Some(StatusBarButton::ZoomIn));
    }

    #[test]
    fn for_editor_picks_up_pressed_button() {
        let mut state = op_editor_core::EditorState::new();
        state.editor_ui.pressed_button = Some(op_editor_core::ButtonPressTarget::StatusBar(
            StatusBarButton::ZoomIn,
        ));
        let bar = StatusBar::for_editor(&state);
        assert_eq!(bar.pressed, Some(StatusBarButton::ZoomIn));
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
