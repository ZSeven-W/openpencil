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

impl TextInputState {
    /// Phase C2 IME handler: feed a `compositionstart / update / end`
    /// event into the text input's preedit/value buffers. This is the
    /// only stateful path Phase B widgets need before C2 wires browser
    /// listeners; widget code stays platform-free.
    ///
    /// Semantics (per spec §2.4):
    /// - `CompositionStart`: clear preedit (caret enters composing mode).
    /// - `CompositionUpdate`: replace preedit with the in-flight text.
    /// - `CompositionEnd`: append the commit text to value, clear preedit.
    ///
    /// `selection` is recorded by the host elsewhere (Phase D DOM mirror);
    /// the static widget paint only needs the preedit string for the
    /// underline branch in `TextInput::paint`.
    pub fn apply_ime(&mut self, event: &crate::ImeEvent) {
        match &event.kind {
            crate::ImeKind::CompositionStart => {
                self.preedit.clear();
            }
            crate::ImeKind::CompositionUpdate { selection: _ } => {
                self.preedit = event.text.clone();
            }
            crate::ImeKind::CompositionEnd => {
                self.value.push_str(&event.text);
                self.preedit.clear();
            }
        }
    }
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
