//! `Dropdown` — Step 1b inspector blend-mode style picker.
//!
//! Phase B static slice: shows the currently-selected option only;
//! `state.open == true` does not yet pop a menu (Phase C lands click +
//! keyboard handling that toggles `open` and renders the menu).

use super::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

#[derive(Debug, Clone)]
pub struct DropdownState {
    pub selected: usize,
    pub open: bool,
}

pub struct Dropdown {
    pub id: WidgetId,
    pub label: String,
    pub options: Vec<String>,
    pub state: DropdownState,
}

impl Dropdown {
    /// Sample blend-mode dropdown. WidgetId range 300-399 is reserved
    /// for dropdowns by Step 1b convention.
    pub fn sample() -> Self {
        Self {
            id: WidgetId::new(300),
            label: "Blend".to_string(),
            options: vec![
                "Normal".to_string(),
                "Multiply".to_string(),
                "Screen".to_string(),
            ],
            state: DropdownState {
                selected: 0,
                open: false,
            },
        }
    }
}

impl Widget for Dropdown {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, 34.0),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, Color::WHITE);
        cx.backend.stroke_rect(rect, Color::BLACK, 1.0);
        let selected = self
            .options
            .get(self.state.selected)
            .map(String::as_str)
            .unwrap_or("");
        let text = TextLayout::single_run(
            selected,
            "system-ui",
            13.0,
            jian_core::scene::Color::rgb(20, 20, 20),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&text, rect.origin + Point2D::new(8.0, 21.0));
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::ComboBox);
        node.set_label(self.label.clone());
        node
    }
}
