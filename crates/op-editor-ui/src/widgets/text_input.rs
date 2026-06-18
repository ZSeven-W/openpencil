//! OP widget facade for Jian's reusable single-line text input view.

use super::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use jian_core::text_input::TextInputState;
use jian_widgets::components::text_input::TextInputView;
use jian_widgets::Tokens;

const DEFAULT_WIDTH: f32 = 160.0;
const DEFAULT_HEIGHT: f32 = 30.0;

pub struct TextInputWidget {
    pub id: WidgetId,
    pub state: TextInputState,
    pub placeholder: String,
    pub focused: bool,
    pub font_size: f32,
    pub now_ms: u64,
    pub pad_x: f32,
    pub tokens: Tokens,
}

impl TextInputWidget {
    pub fn new(id: u64, state: TextInputState) -> Self {
        Self {
            id: WidgetId::new(id),
            state,
            placeholder: String::new(),
            focused: false,
            font_size: 13.0,
            now_ms: 0,
            pad_x: 8.0,
            tokens: Tokens::dark(),
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }
}

impl Widget for TextInputWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width.max(DEFAULT_WIDTH), DEFAULT_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let view = TextInputView {
            state: &self.state,
            placeholder: &self.placeholder,
            focused: self.focused,
            font_size: self.font_size,
            now_ms: self.now_ms,
            pad_x: self.pad_x,
        };
        view.paint(cx.backend, rect, &self.tokens);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::TextInput);
        let label = if self.placeholder.is_empty() {
            self.state.text()
        } else {
            &self.placeholder
        };
        node.set_label(label);
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_text_input_access_role() {
        let widget = TextInputWidget::new(401, TextInputState::with_text("hello"));

        let node = widget.access_node();

        assert_eq!(node.role(), accesskit::Role::TextInput);
    }
}
