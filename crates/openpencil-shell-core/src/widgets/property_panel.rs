//! `PropertyPanel` — right-rail property inspector.
//!
//! Step 2 scope: rebuilt per frame from the document's
//! `selected_node`. Renders a 3-row placeholder set (Name / Type
//! / Children count) when something is selected; a single
//! "(no selection)" row otherwise. Step 3 expands into grouped
//! sections (Layout / Fill / Stroke / Effects) once the document
//! model carries those fields.

use crate::document::Document;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, PropertyRow, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

pub struct PropertyPanel {
    pub id: WidgetId,
    pub header: String,
    pub rows: Vec<PropertyRow>,
}

impl PropertyPanel {
    /// Build a property panel from the current selection. Reserves
    /// the WidgetId range 2000-2999 (header = 2000, rows =
    /// 2001..). The `Select something` placeholder uses 2001 too;
    /// only one or the other exists per frame.
    pub fn for_selected(doc: &Document) -> Self {
        match doc.selected_node() {
            Some(node) => Self {
                id: WidgetId::new(2000),
                header: format!("{} — {}", node.kind.label(), node.name),
                rows: vec![
                    PropertyRow::new(2001, "Name", node.name.clone()),
                    PropertyRow::new(2002, "Type", node.kind.label().to_string()),
                    PropertyRow::new(2003, "Children", node.children.len().to_string()),
                ],
            },
            None => Self {
                id: WidgetId::new(2000),
                header: "Properties".to_string(),
                rows: vec![PropertyRow::new(2001, "Selection", "(none)")],
            },
        }
    }
}

impl Widget for PropertyPanel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Header row (28 px) + per-row 32 px each (matches
        // PropertyRow's own layout) + 8 px top/bottom padding.
        let rows = self.rows.len() as f32;
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, 28.0 + rows * 32.0 + 16.0),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, Color::WHITE);
        // Header strip.
        let header_rect = Rect {
            origin: rect.origin,
            size: Point2D::new(rect.size.x, 28.0),
        };
        cx.backend.fill_rect(header_rect, Color::WHITE);
        cx.backend
            .stroke_rect(header_rect, Color::BLACK, 1.0);
        let header_text = TextLayout::single_run(
            &self.header,
            "system-ui",
            13.0,
            jian_core::scene::Color::rgb(20, 20, 20),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &header_text,
            Point2D::new(rect.origin.x + 8.0, rect.origin.y + 19.0),
        );
        // Rows below the header.
        let mut y = rect.origin.y + 28.0 + 8.0;
        let layout_cx = LayoutCx {
            available_width: rect.size.x,
            dpi: cx.backend.dpi_scale(),
        };
        for row in &self.rows {
            let row_layout = row.layout(&layout_cx);
            let row_rect = Rect {
                origin: Point2D::new(rect.origin.x, y),
                size: Point2D::new(rect.size.x, row_layout.rect.size.y),
            };
            row.paint(cx, row_rect);
            y += row_rect.size.y;
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(self.header.clone());
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::NodeId;

    #[test]
    fn for_selected_with_real_node_emits_three_rows() {
        let doc = Document::sample(); // selected = Title (id 11)
        let panel = PropertyPanel::for_selected(&doc);
        assert_eq!(panel.rows.len(), 3);
        assert_eq!(panel.header, "Text — Title");
        assert_eq!(panel.rows[0].label, "Name");
        assert_eq!(panel.rows[0].value, "Title");
        assert_eq!(panel.rows[1].label, "Type");
        assert_eq!(panel.rows[1].value, "Text");
        assert_eq!(panel.rows[2].label, "Children");
        assert_eq!(panel.rows[2].value, "0");
    }

    #[test]
    fn for_selected_with_no_selection_renders_placeholder() {
        let doc = Document::empty();
        let panel = PropertyPanel::for_selected(&doc);
        assert_eq!(panel.rows.len(), 1);
        assert_eq!(panel.header, "Properties");
        assert_eq!(panel.rows[0].label, "Selection");
        assert_eq!(panel.rows[0].value, "(none)");
    }

    #[test]
    fn for_selected_with_stale_selection_renders_placeholder() {
        let mut doc = Document::sample();
        doc.selected = NodeId::new(9999); // points at no real node
        let panel = PropertyPanel::for_selected(&doc);
        assert_eq!(panel.header, "Properties");
        assert_eq!(panel.rows.len(), 1);
        assert_eq!(panel.rows[0].value, "(none)");
    }

    #[test]
    fn access_node_advertises_group_with_header_label() {
        let doc = Document::sample();
        let panel = PropertyPanel::for_selected(&doc);
        let node = panel.access_node();
        assert_eq!(node.role(), accesskit::Role::Group);
        assert_eq!(node.label(), Some("Text — Title"));
    }

    #[test]
    fn children_count_reflects_selected_node_subtree_size() {
        // Select "Frame" (id 10) which has 2 direct children
        // (Title + Button); PropertyPanel reports DIRECT children
        // count, not transitive descendants.
        let mut doc = Document::sample();
        doc.selected = NodeId::new(10);
        let panel = PropertyPanel::for_selected(&doc);
        assert_eq!(panel.rows[2].value, "2");
    }
}
