//! `TextInput` — Step 1b inspector single-line text field with CJK IME
//! preview support.
//!
//! Phase B static slice: paints `state.preedit` (in-progress IME
//! composition) when present, else `state.value` (committed). The
//! preedit underline is the only visual cue; Phase C event handling
//! lands compositionstart / compositionupdate / compositionend → state
//! mutation in shell-web.

use super::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

#[derive(Debug, Clone, Default)]
pub struct TextInputState {
    pub value: String,
    pub preedit: String,
}

pub struct TextInput {
    pub id: WidgetId,
    pub label: String,
    pub state: TextInputState,
}

impl TextInput {
    /// Sample text input. WidgetId range 400-499 is reserved for text
    /// inputs by Step 1b convention.
    pub fn sample() -> Self {
        Self {
            id: WidgetId::new(400),
            label: "Name".to_string(),
            state: TextInputState {
                value: "Frame 1".to_string(),
                preedit: String::new(),
            },
        }
    }
}

impl Widget for TextInput {
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
        let display = if self.state.preedit.is_empty() {
            self.state.value.as_str()
        } else {
            self.state.preedit.as_str()
        };
        let text = TextLayout::single_run(
            display,
            "system-ui",
            13.0,
            jian_core::scene::Color::rgb(20, 20, 20),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&text, rect.origin + Point2D::new(8.0, 21.0));
        if !self.state.preedit.is_empty() {
            cx.backend.stroke_rect(
                Rect {
                    origin: Point2D::new(rect.origin.x + 8.0, rect.origin.y + 25.0),
                    size: Point2D::new(80.0, 1.0),
                },
                Color::BLACK,
                1.0,
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::TextInput);
        node.set_label(self.label.clone());
        node.set_value(self.state.value.clone());
        node
    }
}
