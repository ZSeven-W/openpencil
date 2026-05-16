//! `PropertyRow` — Step 1b inspector widget showing a label/value pair.
//!
//! Phase B static slice: text content is owned by the widget and never
//! mutates here; Phase C event handling will introduce a separate
//! `*State` if the row gains an editable value.

use super::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

pub struct PropertyRow {
    pub id: WidgetId,
    pub label: String,
    pub value: String,
}

impl PropertyRow {
    /// Construct a non-root property row. `id` MUST be non-zero (id 0 is
    /// reserved for the host root — see [`super::ROOT_WIDGET_ID`]); the
    /// [`super::WidgetId::new`] debug_assert enforces this in dev builds.
    pub fn new(id: u64, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: WidgetId::new(id),
            label: label.into(),
            value: value.into(),
        }
    }
}

impl Widget for PropertyRow {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, 32.0),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, Color::WHITE);
        cx.backend.stroke_rect(rect, Color::BLACK, 1.0);
        let label = TextLayout::single_run(
            &self.label,
            "system-ui",
            13.0,
            jian_core::scene::Color::rgb(80, 80, 80),
            Point2D::new(0.0, 0.0),
        );
        let value = TextLayout::single_run(
            &self.value,
            "system-ui",
            13.0,
            jian_core::scene::Color::rgb(20, 20, 20),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&label, rect.origin + Point2D::new(8.0, 20.0));
        cx.backend
            .draw_text(&value, rect.origin + Point2D::new(rect.size.x * 0.5, 20.0));
    }

    fn access_node(&self) -> accesskit::Node {
        // `Role::Group` instead of `Role::GenericContainer`: GenericContainer
        // maps to ARIA `none`/`presentation` and is filtered out of platform
        // a11y trees, so the label would be dropped on the way to VoiceOver
        // / NVDA. `Group` is the canonical "labelled cluster of related
        // children" role and survives the filter (codex B2 R1 CONCERN).
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(format!("{} {}", self.label, self.value));
        node
    }
}
