//! `TreeWidget` — Step 1b inspector "Layers" panel.
//!
//! Phase B static slice: items are pre-baked via `TreeWidget::sample()`;
//! Phase C event handling adds selection state mutation. Each `TreeItem`
//! carries its own `WidgetId` so accesskit can address rows individually
//! when the DOM mirror lands in Phase D.

use super::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

#[derive(Debug, Clone)]
pub struct TreeItem {
    pub id: WidgetId,
    pub label: String,
    pub depth: u8,
    pub selected: bool,
}

pub struct TreeWidget {
    pub id: WidgetId,
    pub items: Vec<TreeItem>,
}

impl TreeWidget {
    /// Sample tree mirroring the inspector's "Layers" panel. WidgetId
    /// range 100-199 is reserved for tree nodes by Step 1b convention so
    /// id collisions across widget kinds (PropertyRow=200s, Dropdown=300s,
    /// TextInput=400s) cannot happen by accident.
    pub fn sample() -> Self {
        Self {
            id: WidgetId::new(100),
            items: vec![
                TreeItem {
                    id: WidgetId::new(101),
                    label: "Frame".to_string(),
                    depth: 0,
                    selected: true,
                },
                TreeItem {
                    id: WidgetId::new(102),
                    label: "Title".to_string(),
                    depth: 1,
                    selected: false,
                },
                TreeItem {
                    id: WidgetId::new(103),
                    label: "Button".to_string(),
                    depth: 1,
                    selected: false,
                },
            ],
        }
    }
}

impl Widget for TreeWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, self.items.len() as f32 * 28.0 + 8.0),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, Color::WHITE);
        for (index, item) in self.items.iter().enumerate() {
            let y = rect.origin.y + 4.0 + index as f32 * 28.0;
            let row = Rect {
                origin: Point2D::new(rect.origin.x + 4.0, y),
                size: Point2D::new(rect.size.x - 8.0, 24.0),
            };
            if item.selected {
                cx.backend.fill_rect(row, Color::BLUE);
            }
            let text = TextLayout::single_run(
                &item.label,
                "system-ui",
                13.0,
                jian_core::scene::Color::rgb(0, 0, 0),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &text,
                Point2D::new(
                    row.origin.x + 8.0 + f32::from(item.depth) * 16.0,
                    row.origin.y + 17.0,
                ),
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Tree);
        node.set_label("Layers");
        node
    }
}
