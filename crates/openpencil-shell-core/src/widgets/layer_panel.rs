//! `LayerPanel` — left-rail document tree (Step 4 visual lift).
//!
//! Rebuilt to match the TS app's left sidebar: a "Pages" section at
//! the top with a `+` add-page affordance and the active page row
//! highlighted, then a "Layers" section walking the active page's
//! node tree. Dark-theme aware via [`crate::theme::Theme`].
//!
//! Step 2 scope: paint only. P6 wires click → selection.

use crate::document::{Document, Node, NodeId};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

/// Outer width of the panel — matches the TS `w-72` (288 px) layer
/// panel; used by the host layout.
pub const LAYER_PANEL_WIDTH: f32 = 240.0;

const SECTION_HEADER_HEIGHT: f32 = 28.0;
const PAGE_ROW_HEIGHT: f32 = 32.0;
const LAYER_ROW_HEIGHT: f32 = 28.0;
const ROW_PAD_X: f32 = 12.0;
const SECTION_GAP: f32 = 8.0;
const HEADER_FONT: f32 = 12.0;
const ROW_FONT: f32 = 13.0;

/// One row in the layers tree — flat depth-walked view.
#[derive(Debug, Clone)]
pub struct LayerItem {
    pub node_id: NodeId,
    pub label: String,
    pub kind_label: String,
    pub icon: Icon,
    pub depth: u8,
    pub selected: bool,
}

/// Pages-section row.
#[derive(Debug, Clone)]
pub struct PageItem {
    pub page_index: usize,
    pub label: String,
    pub active: bool,
}

pub struct LayerPanel {
    pub id: WidgetId,
    pub pages: Vec<PageItem>,
    pub items: Vec<LayerItem>,
    pub theme: Theme,
}

impl LayerPanel {
    /// Build a panel from the document. Pages list mirrors `doc.pages`
    /// with the active row flagged; layers list walks `doc.active_page()`
    /// depth-first.
    pub fn from_document(doc: &Document) -> Self {
        let pages = doc
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| PageItem {
                page_index: i,
                label: p.name.clone(),
                active: i == doc.active_page_index,
            })
            .collect();
        let mut items = Vec::new();
        if let Some(page) = doc.active_page() {
            for child in &page.children {
                walk(child, doc.selected, 0, &mut items);
            }
        }
        Self {
            id: WidgetId::new(1000),
            pages,
            items,
            theme: doc.theme(),
        }
    }

    pub fn empty() -> Self {
        Self {
            id: WidgetId::new(1000),
            pages: Vec::new(),
            items: Vec::new(),
            theme: Theme::dark(),
        }
    }

    fn intrinsic_height(&self) -> f32 {
        let pages_h = SECTION_HEADER_HEIGHT + self.pages.len() as f32 * PAGE_ROW_HEIGHT;
        let layers_h =
            SECTION_HEADER_HEIGHT + self.items.len().max(1) as f32 * LAYER_ROW_HEIGHT;
        pages_h + SECTION_GAP + layers_h + 16.0
    }

    /// Hit test a (rect, point) — returns either `Page(idx)` for a
    /// page row click or `Layer(node_id)` for a layer row click.
    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<LayerPanelHit> {
        if !rect_contains(rect, point) {
            return None;
        }
        let mut y = rect.origin.y + 8.0 + SECTION_HEADER_HEIGHT;
        for page in &self.pages {
            let row = Rect {
                origin: Point2D::new(rect.origin.x, y),
                size: Point2D::new(rect.size.x, PAGE_ROW_HEIGHT),
            };
            if rect_contains(row, point) {
                return Some(LayerPanelHit::Page(page.page_index));
            }
            y += PAGE_ROW_HEIGHT;
        }
        y += SECTION_GAP + SECTION_HEADER_HEIGHT;
        for item in &self.items {
            let row = Rect {
                origin: Point2D::new(rect.origin.x, y),
                size: Point2D::new(rect.size.x, LAYER_ROW_HEIGHT),
            };
            if rect_contains(row, point) {
                return Some(LayerPanelHit::Layer(item.node_id));
            }
            y += LAYER_ROW_HEIGHT;
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerPanelHit {
    Page(usize),
    Layer(NodeId),
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

fn walk(node: &Node, selected: NodeId, depth: u8, out: &mut Vec<LayerItem>) {
    out.push(LayerItem {
        node_id: node.id,
        label: node.name.clone(),
        kind_label: node.kind.label().to_string(),
        icon: icon_for_kind(&node.kind),
        depth,
        selected: node.id == selected,
    });
    for child in &node.children {
        walk(child, selected, depth.saturating_add(1), out);
    }
}

fn icon_for_kind(kind: &crate::document::NodeKind) -> Icon {
    use crate::document::NodeKind;
    match kind {
        NodeKind::Frame => Icon::Hash,
        NodeKind::Group => Icon::Square,
        NodeKind::Rect => Icon::Square,
        NodeKind::Text => Icon::Type,
        NodeKind::Other(_) => Icon::Square,
    }
}

impl Widget for LayerPanel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, self.intrinsic_height()),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        // Card background.
        cx.backend.fill_rect(rect, self.theme.card);

        let mut y = rect.origin.y + 8.0;

        // Pages section header.
        paint_section_header(cx, &self.theme, rect.origin.x, y, rect.size.x, "页面");
        // "+" add-page affordance, top-right of header row.
        let plus_x = rect.origin.x + rect.size.x - ROW_PAD_X - 12.0;
        let plus_y = y + (SECTION_HEADER_HEIGHT - 14.0) / 2.0;
        draw_icon(
            cx.backend,
            Icon::Plus,
            Point2D::new(plus_x, plus_y),
            14.0,
            self.theme.muted_foreground,
            1.4,
        );
        y += SECTION_HEADER_HEIGHT;

        // Page rows.
        for page in &self.pages {
            let row = Rect {
                origin: Point2D::new(rect.origin.x + 6.0, y + 2.0),
                size: Point2D::new(rect.size.x - 12.0, PAGE_ROW_HEIGHT - 4.0),
            };
            if page.active {
                cx.backend
                    .fill_round_rect(row, 6.0, self.theme.row_selected);
            }
            let label = TextLayout::single_run(
                &page.label,
                "system-ui",
                ROW_FONT,
                to_jian_color(if page.active {
                    self.theme.foreground
                } else {
                    self.theme.muted_foreground
                }),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &label,
                Point2D::new(row.origin.x + 12.0, row.origin.y + 19.0),
            );
            y += PAGE_ROW_HEIGHT;
        }

        y += SECTION_GAP;

        // Layers section header.
        paint_section_header(cx, &self.theme, rect.origin.x, y, rect.size.x, "图层");
        y += SECTION_HEADER_HEIGHT;

        // Layer rows.
        for item in &self.items {
            let row = Rect {
                origin: Point2D::new(rect.origin.x + 6.0, y + 2.0),
                size: Point2D::new(rect.size.x - 12.0, LAYER_ROW_HEIGHT - 4.0),
            };
            if item.selected {
                // TS uses bg-blue-500/15 + primary text + primary
                // icon for the selected layer row.
                cx.backend
                    .fill_round_rect(row, 6.0, self.theme.row_selected_primary);
            }

            let indent = ROW_PAD_X + f32::from(item.depth) * 12.0;
            // Kind icon — switches to primary color when selected.
            let icon_color = if item.selected {
                self.theme.primary
            } else {
                self.theme.muted_foreground
            };
            draw_icon(
                cx.backend,
                item.icon,
                Point2D::new(row.origin.x + indent, row.origin.y + 6.0),
                14.0,
                icon_color,
                1.4,
            );
            // Name label.
            let label_color = if item.selected {
                self.theme.primary
            } else {
                self.theme.card_foreground
            };
            let label = TextLayout::single_run(
                &item.label,
                "system-ui",
                ROW_FONT,
                to_jian_color(label_color),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &label,
                Point2D::new(row.origin.x + indent + 22.0, row.origin.y + 17.0),
            );
            y += LAYER_ROW_HEIGHT;
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Tree);
        node.set_label("Layers");
        node
    }
}

fn paint_section_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    _width: f32,
    label: &str,
) {
    let header_text = TextLayout::single_run(
        label,
        "system-ui",
        HEADER_FONT,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&header_text, Point2D::new(x + ROW_PAD_X, y + 19.0));
}

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_sample_doc_flattens_to_5_layer_rows() {
        let doc = Document::sample();
        let panel = LayerPanel::from_document(&doc);
        assert_eq!(panel.items.len(), 5);
        assert_eq!(panel.items[0].label, "Frame");
        assert_eq!(panel.items[0].depth, 0);
        assert_eq!(panel.items[1].depth, 1);
    }

    #[test]
    fn from_sample_doc_has_one_active_page() {
        let doc = Document::sample();
        let panel = LayerPanel::from_document(&doc);
        assert_eq!(panel.pages.len(), 1);
        assert!(panel.pages[0].active);
        assert_eq!(panel.pages[0].label, "Page 1");
    }

    #[test]
    fn selection_flag_marks_only_selected_row() {
        let doc = Document::sample(); // selected = Title
        let panel = LayerPanel::from_document(&doc);
        let selected = panel.items.iter().filter(|i| i.selected).count();
        assert_eq!(selected, 1);
    }

    #[test]
    fn empty_document_yields_one_default_page_no_layers() {
        let doc = Document::empty();
        let panel = LayerPanel::from_document(&doc);
        assert_eq!(panel.pages.len(), 1);
        assert!(panel.items.is_empty());
    }

    #[test]
    fn hit_test_resolves_first_layer_row() {
        let doc = Document::sample();
        let panel = LayerPanel::from_document(&doc);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
        };
        // Skip pages section (header + 1 page row + section gap +
        // layers header) → land on the first layer row.
        let layer_y = 8.0
            + SECTION_HEADER_HEIGHT
            + PAGE_ROW_HEIGHT
            + SECTION_GAP
            + SECTION_HEADER_HEIGHT
            + LAYER_ROW_HEIGHT / 2.0;
        let p = Point2D::new(rect.size.x / 2.0, layer_y);
        match panel.hit_test(rect, p) {
            Some(LayerPanelHit::Layer(id)) => assert_eq!(id, panel.items[0].node_id),
            other => panic!("expected first layer hit, got {:?}", other),
        }
    }

    #[test]
    fn hit_test_resolves_first_page_row() {
        let doc = Document::sample();
        let panel = LayerPanel::from_document(&doc);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
        };
        let page_y = 8.0 + SECTION_HEADER_HEIGHT + PAGE_ROW_HEIGHT / 2.0;
        let p = Point2D::new(rect.size.x / 2.0, page_y);
        assert_eq!(panel.hit_test(rect, p), Some(LayerPanelHit::Page(0)));
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
    fn from_document_scopes_to_active_page_only() {
        let page1 = crate::document::Page::new(
            1,
            "Page 1",
            vec![Node::leaf(2, crate::document::NodeKind::Frame, "P1-Node")],
        );
        let page2 = crate::document::Page::new(
            3,
            "Page 2",
            vec![Node::leaf(4, crate::document::NodeKind::Frame, "P2-Node")],
        );
        let doc = Document {
            pages: vec![page1, page2],
            active_page_index: 1,
            selected: NodeId::NONE,
            tool: crate::document::Tool::Select,
            viewport: crate::document::Viewport::IDENTITY,
            chat: crate::document::ChatState::default(),
            ui: crate::document::UiState::default(),
        };
        let panel = LayerPanel::from_document(&doc);
        assert_eq!(panel.items.len(), 1);
        assert_eq!(panel.items[0].label, "P2-Node");
        assert_eq!(panel.pages.len(), 2);
        assert!(!panel.pages[0].active);
        assert!(panel.pages[1].active);
    }
}
