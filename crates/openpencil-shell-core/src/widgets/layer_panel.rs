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

pub(crate) const SECTION_HEADER_HEIGHT: f32 = 28.0;
pub(crate) const PAGE_ROW_HEIGHT: f32 = 32.0;
pub(crate) const LAYER_ROW_HEIGHT: f32 = 28.0;
pub(crate) const ROW_PAD_X: f32 = 12.0;
pub(crate) const SECTION_GAP: f32 = 8.0;
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
    /// Active drop target while a drag-to-reorder gesture is in
    /// flight. `None` outside of a drag. Drives the drop-indicator
    /// paint between rows.
    pub drop_target: Option<DropTarget>,
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
            drop_target: None,
        }
    }

    /// Variant that lets the host pre-compute a drop target so the
    /// drop-indicator paints between rows during a drag.
    pub fn from_document_with_drop(doc: &Document, drop_target: Option<DropTarget>) -> Self {
        let mut panel = Self::from_document(doc);
        panel.drop_target = drop_target;
        panel
    }

    pub fn empty() -> Self {
        Self {
            id: WidgetId::new(1000),
            pages: Vec::new(),
            items: Vec::new(),
            theme: Theme::dark(),
            pages_label: "Pages".to_string(),
            layers_label: "Layers".to_string(),
            drop_target: None,
        }
    }

    pub(crate) fn intrinsic_height(&self) -> f32 {
        let pages_h = SECTION_HEADER_HEIGHT + self.pages.len() as f32 * PAGE_ROW_HEIGHT;
        let layers_h = SECTION_HEADER_HEIGHT + self.items.len().max(1) as f32 * LAYER_ROW_HEIGHT;
        pages_h + SECTION_GAP + layers_h + 16.0
    }

    /// Compute the drop target for a drag-in-progress over the
    /// layer rows. Returns `None` when the cursor isn't over a
    /// layer row. `Before` when the cursor is in the upper half
    /// of the row, `After` in the lower half; `indicator_y` is
    /// where the host paints the drop-indicator line.
    pub fn drop_target_at(&self, rect: Rect, point: Point2D) -> Option<DropTarget> {
        if !rect_contains(rect, point) {
            return None;
        }
        let mut y = rect.origin.y
            + 8.0
            + SECTION_HEADER_HEIGHT
            + self.pages.len() as f32 * PAGE_ROW_HEIGHT
            + SECTION_GAP
            + SECTION_HEADER_HEIGHT;
        for item in &self.items {
            let row_top = y;
            let row_bottom = y + LAYER_ROW_HEIGHT;
            if point.y >= row_top && point.y <= row_bottom {
                let mid = row_top + LAYER_ROW_HEIGHT / 2.0;
                let position = if point.y < mid {
                    DropPosition::Before
                } else {
                    DropPosition::After
                };
                let indicator_y = match position {
                    DropPosition::Before => row_top,
                    DropPosition::After => row_bottom,
                };
                return Some(DropTarget {
                    anchor: item.node_id,
                    position,
                    indicator_y,
                });
            }
            y = row_bottom;
        }
        None
    }

    /// Hit test a (rect, point) — returns `Page(idx)` for a page
    /// row, `Layer(node_id)` for a layer row, eye/lock/chevron
    /// toggles for the trailing icons, or `AddPage` for the `+`
    /// on the Pages section header.
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

/// Where a layer-drag would drop relative to the hovered anchor row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPosition {
    /// Drop slides in immediately ABOVE the anchor row in the
    /// flat layer list (parent's children vec index decreases by
    /// one for a same-parent move).
    Before,
    /// Drop slides in immediately BELOW the anchor row.
    After,
}

/// Hit-test result for a drag-in-progress over the LayerPanel rows.
/// Carries the anchor NodeId (the row the cursor is over), the
/// relative position (top half = Before, bottom half = After), and
/// the y in panel-local space where the drop-indicator should
/// paint (between rows).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropTarget {
    pub anchor: NodeId,
    pub position: DropPosition,
    pub indicator_y: f32,
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
            // Trailing eye + lock icons. Icon shape signals state
            // (Eye/EyeOff, Lock/LockOpen); locked Lock paints in
            // warm orange so it reads as a "can't edit" alert.
            // Eye-to-lock gap (22 px) matches hit-test spacing.
            let trailing_right = row.origin.x + row.size.x - 8.0;
            let lock_x = trailing_right - 14.0;
            let eye_x = lock_x - 22.0;
            let eye_icon = if item.hidden { Icon::EyeOff } else { Icon::Eye };
            let lock_icon = if item.locked {
                Icon::Lock
            } else {
                Icon::LockOpen
            };
            let trailing_default = if item.selected {
                dim(self.theme.primary, dim_factor)
            } else {
                dim(self.theme.muted_foreground, dim_factor)
            };
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
            // Slimmer than the leading icons (12 px @ 1.2 stroke).
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

        // Drop-indicator — a 2 px primary-tint line painted between
        // rows when a drag-to-reorder gesture is in flight. Sits on
        // top of all row chrome so it remains visible regardless of
        // hover/selected backgrounds underneath.
        if let Some(drop) = self.drop_target {
            let indicator_rect = Rect {
                origin: Point2D::new(rect.origin.x + ROW_PAD_X, drop.indicator_y - 1.0),
                size: Point2D::new(rect.size.x - ROW_PAD_X * 2.0, 2.0),
            };
            cx.backend.fill_rect(indicator_rect, self.theme.primary);
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
