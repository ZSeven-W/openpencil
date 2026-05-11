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
    /// True when this node has any children — drives the leading
    /// chevron-down expand caret on the row (TS LayerRow shows
    /// the caret only on container rows).
    pub has_children: bool,
    /// Hidden flag — drives the dimmed-row visual + an
    /// emphasised eye icon so the user can tell at a glance
    /// which rows are hidden.
    pub hidden: bool,
    /// Locked flag — drives an emphasised lock icon.
    pub locked: bool,
    /// Collapsed flag — drives the leading chevron direction
    /// (`▼` when expanded, `▶` when collapsed) and tells the
    /// walker to skip descending into children.
    pub collapsed: bool,
    /// Hovered flag — true iff the cursor is over this row.
    /// Drives the hover-reveal of the trailing eye + lock
    /// icons (TS parity: only the active row exposes them).
    pub hovered: bool,
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
    pub pages_label: String,
    pub layers_label: String,
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
                walk(child, doc.selected, doc.ui.hovered_layer_id, 0, &mut items);
            }
        }
        Self {
            id: WidgetId::new(1000),
            pages,
            items,
            theme: doc.theme(),
            pages_label: doc.t("pages.title").to_string(),
            layers_label: doc.t("layers.title").to_string(),
        }
    }

    pub fn empty() -> Self {
        Self {
            id: WidgetId::new(1000),
            pages: Vec::new(),
            items: Vec::new(),
            theme: Theme::dark(),
            pages_label: "Pages".to_string(),
            layers_label: "Layers".to_string(),
        }
    }

    fn intrinsic_height(&self) -> f32 {
        let pages_h = SECTION_HEADER_HEIGHT + self.pages.len() as f32 * PAGE_ROW_HEIGHT;
        let layers_h = SECTION_HEADER_HEIGHT + self.items.len().max(1) as f32 * LAYER_ROW_HEIGHT;
        pages_h + SECTION_GAP + layers_h + 16.0
    }

    /// Hit test a (rect, point) — returns either `Page(idx)` for a
    /// page row click, `Layer(node_id)` for a layer row click, or
    /// `AddPage` for a click on the `+` affordance on the Pages
    /// section header.
    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<LayerPanelHit> {
        if !rect_contains(rect, point) {
            return None;
        }
        // Pages section header — `+` add-page affordance at top-right.
        // Geometry mirrors the paint pass:
        //   plus_x = rect.origin.x + rect.size.x - ROW_PAD_X - 12.0
        //   plus_y = (rect.origin.y + 8.0) + (SECTION_HEADER_HEIGHT - 14.0) / 2.0
        //   size   = 14 px
        // 4 px slop so a sloppy click still lands.
        let plus_x = rect.origin.x + rect.size.x - ROW_PAD_X - 12.0;
        let plus_y = rect.origin.y + 8.0 + (SECTION_HEADER_HEIGHT - 14.0) / 2.0;
        let slop = 4.0;
        if point.x >= plus_x - slop
            && point.x <= plus_x + 14.0 + slop
            && point.y >= plus_y - slop
            && point.y <= plus_y + 14.0 + slop
        {
            return Some(LayerPanelHit::AddPage);
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
                // Match the paint geometry — same 14 px icon
                // boxes positioned at `trailing_right` minus 14 /
                // 32 (lock / eye). Slop of 4 px around each so
                // small mouse offsets still register.
                let inner = Rect {
                    origin: Point2D::new(row.origin.x + 6.0, y + 2.0),
                    size: Point2D::new(row.size.x - 12.0, LAYER_ROW_HEIGHT - 4.0),
                };
                let trailing_right = inner.origin.x + inner.size.x - 8.0;
                let lock_x = trailing_right - 14.0;
                // Widen eye-to-lock gap (was 18 → 22) so 14 px
                // icons + 4 px slop each side don't overlap.
                let eye_x = lock_x - 22.0;
                let icon_y = inner.origin.y + 6.0;
                let slop = 4.0;
                if point.x >= lock_x - slop
                    && point.x <= lock_x + 14.0 + slop
                    && point.y >= icon_y - slop
                    && point.y <= icon_y + 14.0 + slop
                {
                    return Some(LayerPanelHit::ToggleLocked(item.node_id));
                }
                if point.x >= eye_x - slop
                    && point.x <= eye_x + 14.0 + slop
                    && point.y >= icon_y - slop
                    && point.y <= icon_y + 14.0 + slop
                {
                    return Some(LayerPanelHit::ToggleHidden(item.node_id));
                }
                // Leading chevron — only present on container
                // rows. Painted at `inner.origin.x + indent`
                // where indent = ROW_PAD_X + depth*12.
                if item.has_children {
                    let indent = ROW_PAD_X + f32::from(item.depth) * 12.0;
                    let chev_x = inner.origin.x + indent;
                    if point.x >= chev_x - slop
                        && point.x <= chev_x + 14.0 + slop
                        && point.y >= icon_y - slop
                        && point.y <= icon_y + 14.0 + slop
                    {
                        return Some(LayerPanelHit::ToggleCollapsed(item.node_id));
                    }
                }
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
    /// Click on the eye icon — host should toggle the node's
    /// `hidden` flag.
    ToggleHidden(NodeId),
    /// Click on the lock icon — host should toggle the node's
    /// `locked` flag.
    ToggleLocked(NodeId),
    /// Click on the leading chevron — host should toggle the
    /// node's `collapsed` flag so children show/hide in the
    /// layer tree.
    ToggleCollapsed(NodeId),
    /// Click on the `+` add-page affordance in the Pages section
    /// header — host should append a fresh page and switch to it.
    AddPage,
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

fn walk(
    node: &Node,
    selected: NodeId,
    hovered: Option<NodeId>,
    depth: u8,
    out: &mut Vec<LayerItem>,
) {
    out.push(LayerItem {
        node_id: node.id,
        label: node.name.clone(),
        kind_label: node.kind.label().to_string(),
        icon: icon_for_kind(&node.kind),
        depth,
        selected: node.id == selected,
        has_children: !node.children.is_empty(),
        hidden: node.hidden,
        locked: node.locked,
        collapsed: node.collapsed,
        hovered: hovered == Some(node.id),
    });
    // Collapsed nodes hide their subtree from the LayerPanel
    // (canvas paint / hit-test are unaffected — that's a
    // tree-view-only concern).
    if !node.collapsed {
        for child in &node.children {
            walk(child, selected, hovered, depth.saturating_add(1), out);
        }
    }
}

fn icon_for_kind(kind: &crate::document::NodeKind) -> Icon {
    use crate::document::NodeKind;
    match kind {
        NodeKind::Frame => Icon::Hash,
        NodeKind::Group => Icon::Square,
        NodeKind::Rect => Icon::Square,
        NodeKind::Ellipse => Icon::Circle,
        NodeKind::Polygon => Icon::Triangle,
        NodeKind::Line => Icon::Minus,
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

        // Right-edge hairline so the LayerPanel reads as a
        // distinct surface from the canvas next to it.
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x + rect.size.x - 1.0, rect.origin.y),
                size: Point2D::new(1.0, rect.size.y),
            },
            self.theme.border,
        );

        let mut y = rect.origin.y + 8.0;

        // Pages section header.
        paint_section_header(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            rect.size.x,
            &self.pages_label,
        );
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

        // Hairline between Pages and Layers sections — mirrors
        // the TS LayerPanel's `border-t border-border`.
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x + ROW_PAD_X, y - SECTION_GAP / 2.0),
                size: Point2D::new(rect.size.x - ROW_PAD_X * 2.0, 1.0),
            },
            self.theme.border,
        );

        // Layers section header.
        paint_section_header(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            rect.size.x,
            &self.layers_label,
        );
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
            // Hidden rows dim everything by 50 % alpha — TS parity
            // with `opacity-50` on hidden layer rows.
            let dim = |c: Color, factor: f32| -> Color {
                Color {
                    r: c.r,
                    g: c.g,
                    b: c.b,
                    a: c.a * factor,
                }
            };
            let dim_factor = if item.hidden { 0.45 } else { 1.0 };
            let icon_color = if item.selected {
                dim(self.theme.primary, dim_factor)
            } else {
                dim(self.theme.muted_foreground, dim_factor)
            };
            // Leading chevron — only for container rows (TS
            // `LayerRow` shows the caret only when the node has
            // children). 12 px slot so the kind icon aligns to
            // the same x regardless.
            if item.has_children {
                let chev_icon = if item.collapsed {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronDown
                };
                draw_icon(
                    cx.backend,
                    chev_icon,
                    Point2D::new(row.origin.x + indent, row.origin.y + 6.0),
                    14.0,
                    icon_color,
                    1.4,
                );
            }
            // 18 px slot for the chevron (was 14, no breathing
            // room between chevron and kind icon — user feedback
            // 2026-05-11).
            let icon_x = row.origin.x + indent + 18.0;
            // Kind icon — switches to primary color when selected.
            draw_icon(
                cx.backend,
                item.icon,
                Point2D::new(icon_x, row.origin.y + 6.0),
                14.0,
                icon_color,
                1.4,
            );
            // Name label — dims to muted when hidden so the user
            // can tell at a glance which rows are invisible.
            let label_color = if item.selected {
                dim(self.theme.primary, dim_factor)
            } else {
                dim(self.theme.card_foreground, dim_factor)
            };
            let label = TextLayout::single_run(
                &item.label,
                "system-ui",
                ROW_FONT,
                to_jian_color(label_color),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&label, Point2D::new(icon_x + 20.0, row.origin.y + 17.0));
            // Trailing eye + lock icons. The ICON SHAPE itself
            // signals state (Eye/EyeOff, Lock/LockOpen) — TS
            // parity. Locked icon also paints in a warm orange
            // so it reads as a "can't edit" alert.
            let trailing_right = row.origin.x + row.size.x - 8.0;
            let lock_x = trailing_right - 14.0;
            // Match hit-test spacing (22 px gap) so eye-vs-lock
            // hit-test slop boxes don't overlap.
            let eye_x = lock_x - 22.0;
            let eye_icon = if item.hidden { Icon::EyeOff } else { Icon::Eye };
            let lock_icon = if item.locked {
                Icon::Lock
            } else {
                Icon::LockOpen
            };
            // Default trailing color matches the rest of the row
            // chrome (primary tint on selected rows, muted
            // otherwise). Hidden dim_factor cascades.
            let trailing_default = if item.selected {
                dim(self.theme.primary, dim_factor)
            } else {
                dim(self.theme.muted_foreground, dim_factor)
            };
            // Warm orange for the locked state — signals "this
            // row is protected" without depending on a separate
            // theme token (TS uses a comparable hue).
            let lock_locked = Color {
                r: 0.92,
                g: 0.49,
                b: 0.20,
                a: 1.0,
            };
            let eye_color = trailing_default;
            let lock_color = if item.locked {
                lock_locked
            } else {
                trailing_default
            };
            // Trailing icons are slimmer than the leading
            // chevron / kind icon — 12 px @ 1.2 stroke reads as
            // a "metadata affordance" rather than a primary
            // glyph (TS parity).
            let trailing_size = 12.0;
            let trailing_stroke = 1.2;
            let trailing_y = row.origin.y + 7.0;
            // Eye only paints on hover / selected / hidden — TS
            // parity (hover reveal). Hidden always shows so
            // the user sees state at a glance.
            let show_eye = item.hovered || item.selected || item.hidden;
            // Lock paints on hover / selected / locked.
            let show_lock = item.hovered || item.selected || item.locked;
            if show_eye {
                draw_icon(
                    cx.backend,
                    eye_icon,
                    Point2D::new(eye_x, trailing_y),
                    trailing_size,
                    eye_color,
                    trailing_stroke,
                );
            }
            if show_lock {
                draw_icon(
                    cx.backend,
                    lock_icon,
                    Point2D::new(lock_x, trailing_y),
                    trailing_size,
                    lock_color,
                    trailing_stroke,
                );
            }
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
    fn hit_test_resolves_add_page_plus_icon() {
        let doc = Document::sample();
        let panel = LayerPanel::from_document(&doc);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
        };
        // Mirror the paint geometry: plus_x = right edge - ROW_PAD_X
        // - 12, plus_y = 8 + (SECTION_HEADER_HEIGHT - 14) / 2.
        let plus_x = rect.size.x - ROW_PAD_X - 12.0;
        let plus_y = 8.0 + (SECTION_HEADER_HEIGHT - 14.0) / 2.0;
        // Centre of the 14 px icon.
        assert_eq!(
            panel.hit_test(rect, Point2D::new(plus_x + 7.0, plus_y + 7.0)),
            Some(LayerPanelHit::AddPage)
        );
        // Edge-of-slop sample — 3 px LEFT of the icon's left edge,
        // inside the 4 px slop band. Locks the slop contract: a
        // regression that shrank slop below 3 px would fail here.
        assert_eq!(
            panel.hit_test(rect, Point2D::new(plus_x - 3.0, plus_y + 7.0)),
            Some(LayerPanelHit::AddPage)
        );
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
            selected_set: Vec::new(),
            clipboard: Vec::new(),
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
