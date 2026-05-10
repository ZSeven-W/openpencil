//! `LayerPanel` — left-rail document tree.
//!
//! Rebuilt per frame from `Document::pages[0]`. Walks the node
//! forest depth-first into a flat `LayerItem` list with depth +
//! selection state, then paints rows like `TreeWidget` does in
//! Phase B (white background, blue selected row, depth-indented
//! label). Also renders a small kind-label column on the right of
//! each row so users can see node type at a glance.
//!
//! Step 2 scope: paint only. Click→selection wiring lands in
//! Step 3 alongside pointer event integration.

use crate::document::{Document, Node, NodeId};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

/// One row in the layer panel — the flat-list flattening of one
/// document node.
#[derive(Debug, Clone)]
pub struct LayerItem {
    pub node_id: NodeId,
    pub label: String,
    pub kind_label: String,
    pub depth: u8,
    pub selected: bool,
}

pub struct LayerPanel {
    pub id: WidgetId,
    pub items: Vec<LayerItem>,
}

impl LayerPanel {
    /// Build a flat depth-walked view of `Document::pages[0]`.
    /// Multi-page documents are Step 3+; today the first page is
    /// the whole canvas.
    pub fn from_document(doc: &Document) -> Self {
        let mut items = Vec::new();
        if let Some(page) = doc.pages.first() {
            for child in &page.children {
                walk(child, doc.selected, 0, &mut items);
            }
        }
        Self {
            // Reserved id range 1000-1999 for editor-UI containers
            // (LayerPanel = 1000, PropertyPanel = 2000, Toolbar =
            // 3000). Per-row child ids use the document node id
            // directly — see paint().
            id: WidgetId::new(1000),
            items,
        }
    }

    /// Empty layer panel — used when the host has no document
    /// loaded yet. Paint draws just the placeholder background.
    pub fn empty() -> Self {
        Self {
            id: WidgetId::new(1000),
            items: Vec::new(),
        }
    }
}

fn walk(node: &Node, selected: NodeId, depth: u8, out: &mut Vec<LayerItem>) {
    out.push(LayerItem {
        node_id: node.id,
        label: node.name.clone(),
        kind_label: node.kind.label().to_string(),
        depth,
        selected: node.id == selected,
    });
    for child in &node.children {
        walk(child, selected, depth.saturating_add(1), out);
    }
}

impl Widget for LayerPanel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // 28 px per row + 8 px top padding. Empty panel still
        // reserves a small visible band so the host's layout
        // doesn't collapse to zero.
        let rows = self.items.len().max(1) as f32;
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, rows * 28.0 + 8.0),
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
            // Primary label, depth-indented 16 px per level.
            let label = TextLayout::single_run(
                &item.label,
                "system-ui",
                13.0,
                jian_core::scene::Color::rgb(0, 0, 0),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &label,
                Point2D::new(
                    row.origin.x + 8.0 + f32::from(item.depth) * 16.0,
                    row.origin.y + 17.0,
                ),
            );
            // Kind-label at the right of the row in a muted tone.
            let kind = TextLayout::single_run(
                &item.kind_label,
                "system-ui",
                11.0,
                jian_core::scene::Color::rgb(120, 120, 120),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &kind,
                Point2D::new(row.origin.x + row.size.x - 60.0, row.origin.y + 17.0),
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Tree);
        node.set_label("Layers");
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_sample_doc_flattens_to_5_rows() {
        let doc = Document::sample();
        let panel = LayerPanel::from_document(&doc);
        // sample() = Frame > [Title, Button > [bg, text]] = 5 nodes
        assert_eq!(panel.items.len(), 5);
        assert_eq!(panel.items[0].label, "Frame");
        assert_eq!(panel.items[0].depth, 0);
        assert_eq!(panel.items[1].label, "Title");
        assert_eq!(panel.items[1].depth, 1);
        assert_eq!(panel.items[2].label, "Button");
        assert_eq!(panel.items[2].depth, 1);
        assert_eq!(panel.items[3].label, "Button background");
        assert_eq!(panel.items[3].depth, 2);
        assert_eq!(panel.items[4].label, "Click me");
        assert_eq!(panel.items[4].depth, 2);
    }

    #[test]
    fn selection_flag_marks_only_selected_row() {
        let doc = Document::sample(); // selected = Title (id 11)
        let panel = LayerPanel::from_document(&doc);
        let selected_count = panel.items.iter().filter(|i| i.selected).count();
        assert_eq!(selected_count, 1);
        let selected = panel.items.iter().find(|i| i.selected).unwrap();
        assert_eq!(selected.label, "Title");
    }

    #[test]
    fn empty_document_yields_empty_panel() {
        let doc = Document::empty();
        let panel = LayerPanel::from_document(&doc);
        assert!(panel.items.is_empty());
    }

    #[test]
    fn empty_panel_layout_reserves_visible_band() {
        let panel = LayerPanel::empty();
        let cx = LayoutCx {
            available_width: 200.0,
            dpi: 1.0,
        };
        let lb = panel.layout(&cx);
        // Empty -> rows=1 placeholder -> 28 + 8 = 36 px.
        assert_eq!(lb.rect.size.y, 36.0);
    }

    #[test]
    fn access_node_advertises_tree_role_and_layers_label() {
        let doc = Document::sample();
        let panel = LayerPanel::from_document(&doc);
        let node = panel.access_node();
        assert_eq!(node.role(), accesskit::Role::Tree);
        assert_eq!(node.label(), Some("Layers"));
    }

    #[test]
    fn kind_label_carries_through() {
        let doc = Document::sample();
        let panel = LayerPanel::from_document(&doc);
        // Frame > [Text "Title", Group "Button" > [Rect, Text]]
        assert_eq!(panel.items[0].kind_label, "Frame");
        assert_eq!(panel.items[1].kind_label, "Text");
        assert_eq!(panel.items[2].kind_label, "Group");
        assert_eq!(panel.items[3].kind_label, "Rect");
        assert_eq!(panel.items[4].kind_label, "Text");
    }

    #[test]
    fn other_kind_label_round_trips_string() {
        use crate::document::NodeKind;
        let doc = Document {
            pages: vec![crate::document::Page::new(
                1,
                "p",
                vec![Node::leaf(2, NodeKind::Other("Custom".into()), "n")],
            )],
            selected: NodeId::NONE,
        };
        let panel = LayerPanel::from_document(&doc);
        assert_eq!(panel.items[0].kind_label, "Custom");
    }
}
