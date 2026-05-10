//! `Toolbar` — top-strip tool selector.
//!
//! Step 2 scope: paint a horizontal row of fixed-width tool
//! buttons (Select / Rect / Text / Pen). The active tool's button
//! is filled blue; others are white with a black stroke. Click /
//! keyboard tool-switching wires up in Step 3 alongside pointer
//! handling.

use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

/// Width of one tool button in pixels. Toolbar layout reports
/// `tools.len() * BUTTON_WIDTH + (tools.len() - 1) * GAP +
/// 2 * PAD` as its total width contribution; the host typically
/// constrains the toolbar to the full window width and lets the
/// button strip pin to the left.
const BUTTON_WIDTH: f32 = 64.0;
const BUTTON_HEIGHT: f32 = 28.0;
const GAP: f32 = 6.0;
const PAD: f32 = 8.0;

pub struct Toolbar {
    pub id: WidgetId,
    pub tools: Vec<&'static str>,
    pub active: usize,
}

impl Toolbar {
    /// Default Step 2 tool set — Select / Rect / Text / Pen.
    /// Reserved id range 3000-3999 (toolbar = 3000, per-button
    /// ids are 3001..). `active` defaults to 0 = Select.
    pub fn default_set() -> Self {
        Self {
            id: WidgetId::new(3000),
            tools: vec!["Select", "Rect", "Text", "Pen"],
            active: 0,
        }
    }
}

impl Widget for Toolbar {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Toolbar always reports the host's available width — it
        // pins to the top strip and the button row aligns to the
        // left within that width.
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, BUTTON_HEIGHT + 2.0 * PAD),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, Color::WHITE);
        let mut x = rect.origin.x + PAD;
        let y = rect.origin.y + PAD;
        for (idx, label) in self.tools.iter().enumerate() {
            let button = Rect {
                origin: Point2D::new(x, y),
                size: Point2D::new(BUTTON_WIDTH, BUTTON_HEIGHT),
            };
            // **Overflow handling, Step 2 scope (codex Step 2 R1
            // CONCERN-4)**: Buttons that would overflow the
            // toolbar's reported width are silently dropped. This
            // keeps rendering predictable on narrow / mobile
            // viewports but is a UX cliff — the user can't reach
            // hidden tools. Step 3+ replaces this with one of:
            //   - horizontal scroll inside the toolbar rect
            //     (drag / wheel)
            //   - "More tools" overflow dropdown anchored to the
            //     right edge
            //   - icon-only mode at narrower widths
            // Phase D pointer/wheel routing has to land before
            // any of those is wirable.
            if x + BUTTON_WIDTH > rect.origin.x + rect.size.x - PAD {
                break;
            }
            if idx == self.active {
                cx.backend.fill_rect(button, Color::BLUE);
            } else {
                cx.backend.fill_rect(button, Color::WHITE);
                cx.backend.stroke_rect(button, Color::BLACK, 1.0);
            }
            let text = TextLayout::single_run(
                label,
                "system-ui",
                12.0,
                jian_core::scene::Color::rgb(20, 20, 20),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &text,
                Point2D::new(button.origin.x + 8.0, button.origin.y + 19.0),
            );
            x += BUTTON_WIDTH + GAP;
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Toolbar);
        node.set_label("Toolbar");
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_set_has_four_tools_and_active_zero() {
        let toolbar = Toolbar::default_set();
        assert_eq!(toolbar.tools.len(), 4);
        assert_eq!(toolbar.tools[0], "Select");
        assert_eq!(toolbar.tools[3], "Pen");
        assert_eq!(toolbar.active, 0);
    }

    #[test]
    fn layout_reports_full_width_and_44px_height() {
        let toolbar = Toolbar::default_set();
        let cx = LayoutCx {
            available_width: 800.0,
            dpi: 1.0,
        };
        let lb = toolbar.layout(&cx);
        assert_eq!(lb.rect.size.x, 800.0);
        // BUTTON_HEIGHT(28) + 2 * PAD(8) = 44
        assert_eq!(lb.rect.size.y, 44.0);
    }

    #[test]
    fn access_node_advertises_toolbar_role() {
        let toolbar = Toolbar::default_set();
        let node = toolbar.access_node();
        assert_eq!(node.role(), accesskit::Role::Toolbar);
        assert_eq!(node.label(), Some("Toolbar"));
    }

    #[test]
    fn active_index_round_trips() {
        let mut toolbar = Toolbar::default_set();
        toolbar.active = 2;
        assert_eq!(toolbar.tools[toolbar.active], "Text");
    }
}
