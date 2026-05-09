//! Step 1b §1.4 widget glue — the only file in shell-web allowed to call
//! into `openpencil_shell_core::widgets`. All widget logic (state, paint,
//! layout, accesskit) lives in shell-core; this host is a thin paint-loop
//! adapter that takes a `&mut WebBackend` and dispatches to the four
//! inspector widgets.
//!
//! Any function that pulls in `openpencil_shell_core::widgets::*` MUST
//! live in this file (per spec §1.4). The B4 boundary check script
//! greps for the `// glue:` marker on the paint signature so the
//! invariant survives drift.

use crate::backend::WebBackend;
use openpencil_shell_core::widgets::{
    Dropdown, LayoutCx, PaintCx, PropertyRow, TextInput, TreeWidget, Widget,
};
use openpencil_shell_core::{Point2D, Rect, RenderBackend};

/// The Step 1b inspector composition. Owns one of each widget kind so
/// the first-frame Phase B mount paints something meaningful; Phase C
/// event handling will mutate the dropdown / text-input state in place.
pub struct WidgetHost {
    tree: TreeWidget,
    width: PropertyRow,
    dropdown: Dropdown,
    text_input: TextInput,
}

impl WidgetHost {
    pub fn new() -> Self {
        Self {
            tree: TreeWidget::sample(),
            width: PropertyRow::new(200, "Width", "960"),
            dropdown: Dropdown::sample(),
            text_input: TextInput::sample(),
        }
    }

    /// Phase C2 IME forwarding: route a composition event into the
    /// text-input widget's state. Phase D+ may choose a different
    /// target widget (focus-follows-IME); for now the inspector's
    /// only text input takes everything.
    pub fn apply_ime(&mut self, event: &openpencil_shell_core::ImeEvent) { // glue:
        self.text_input.state.apply_ime(event);
    }

    /// Phase C2 keyboard forwarding: arrow / Enter / Escape land on
    /// the dropdown. Phase D+ may add focus-follows-key routing; for
    /// now the dropdown is the only keyboard-aware widget in the
    /// inspector slice.
    pub fn apply_key(&mut self, event: &openpencil_shell_core::KeyEvent) { // glue:
        self.dropdown
            .state
            .apply_key(event, self.dropdown.options.len());
    }

    /// Dispatches paint to the shell-core widgets stacked vertically with
    /// 12 px gaps. The `// glue:` marker on the same line as the
    /// signature is what `tools/check-widget-boundary.sh` (Phase B4)
    /// greps for to confirm widget calls happen here and nowhere else
    /// in shell-web.
    pub fn paint(&self, backend: &mut WebBackend, available_width: f32) { // glue:
        let layout = LayoutCx {
            available_width,
            dpi: backend.dpi_scale(),
        };
        let mut y = 16.0;
        let widgets: [&dyn Widget; 4] = [
            &self.tree,
            &self.width,
            &self.dropdown,
            &self.text_input,
        ];
        for widget in widgets {
            let box_ = widget.layout(&layout);
            let rect = Rect {
                origin: Point2D::new(16.0, y),
                size: Point2D::new(available_width, box_.rect.size.y),
            };
            // Reborrow on every iteration so the `&mut WebBackend` is not
            // moved into the first PaintCx — without `&mut *backend` the
            // second iteration would fail borrow check.
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            widget.paint(&mut cx, rect);
            y += rect.size.y + 12.0;
        }
    }
}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}
