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

impl DropdownState {
    /// Phase C2 keyboard handler: arrow keys cycle the selection
    /// (and open the menu), Enter / Escape close it. `option_count`
    /// is the parent Dropdown's `options.len()` — passed in so this
    /// stays a pure state mutator without borrowing the Dropdown
    /// struct around it.
    ///
    /// Skips when the key is released, the dropdown has no options,
    /// OR an IME composition is in progress (`event.is_composing ==
    /// true`). The IME-skip is required by spec §2.4: while the
    /// user is composing CJK / dead-key sequences, arrow keys belong
    /// to the IME's candidate-picker UI, NOT to widgets that read
    /// keys directly. Without this guard, every arrow keystroke
    /// during a composition would cycle the dropdown selection
    /// behind the IME panel — surfaced by the codex Phase C
    /// stop-hook ("focused IME textarea lets composing keys mutate
    /// dropdown state").
    ///
    /// Non-navigation keys fall through unchanged so widget code can
    /// layer additional handlers later (e.g. typeahead in Phase D+).
    pub fn apply_key(&mut self, event: &crate::KeyEvent, option_count: usize) {
        if event.state != crate::KeyState::Pressed || option_count == 0 || event.is_composing {
            return;
        }
        match event.key {
            crate::KeyValue::Named(crate::NamedKey::ArrowDown) => {
                self.open = true;
                self.selected = (self.selected + 1).min(option_count - 1);
            }
            crate::KeyValue::Named(crate::NamedKey::ArrowUp) => {
                self.open = true;
                self.selected = self.selected.saturating_sub(1);
            }
            crate::KeyValue::Named(crate::NamedKey::Enter) => {
                self.open = false;
            }
            crate::KeyValue::Named(crate::NamedKey::Escape) => {
                self.open = false;
            }
            _ => {}
        }
    }
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
