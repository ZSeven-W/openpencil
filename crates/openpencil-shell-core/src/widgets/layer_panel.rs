//! `LayerPanel` — left-rail document tree (Pages + Layers sections).

use crate::document::{Document, NodeId};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::layer_panel_walkers::{icon_for_kind, walk, walk_excluding};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

/// Outer panel width; host layout uses this.
pub const LAYER_PANEL_WIDTH: f32 = 240.0;

/// Rename draft for the given page index, when one is active.
fn rename_override_page(doc: &Document, idx: usize) -> Option<String> {
    use crate::document::LayerContextTarget;
    doc.ui.layer_rename.as_ref().and_then(|s| match s.target {
        LayerContextTarget::Page(i) if i == idx => Some(s.draft.clone()),
        _ => None,
    })
}

/// Rename draft for the given layer id, when one is active.
fn rename_override_layer(doc: &Document, id: NodeId) -> Option<String> {
    use crate::document::LayerContextTarget;
    doc.ui.layer_rename.as_ref().and_then(|s| match s.target {
        LayerContextTarget::Layer(i) if i == id => Some(s.draft.clone()),
        _ => None,
    })
}

fn rename_is_page(doc: &Document, idx: usize) -> bool {
    use crate::document::LayerContextTarget;
    matches!(
        doc.ui.layer_rename.as_ref().map(|s| s.target),
        Some(LayerContextTarget::Page(i)) if i == idx
    )
}

pub(crate) const SECTION_HEADER_HEIGHT: f32 = 28.0;
pub(crate) const PAGE_ROW_HEIGHT: f32 = 32.0;
pub(crate) const LAYER_ROW_HEIGHT: f32 = 28.0;
pub(crate) const ROW_PAD_X: f32 = 12.0;
pub(crate) const SECTION_GAP: f32 = 8.0;
use crate::widgets::layer_panel_paint::{
    paint_drag_ghost, paint_rename_input, paint_section_header, to_jian_color, truncate_to_fit,
    ROW_FONT,
};

/// One row in the layers tree — flat depth-walked view.
#[derive(Debug, Clone)]
pub struct LayerItem {
    pub node_id: NodeId,
    pub label: String,
    pub kind_label: String,
    pub icon: Icon,
    pub depth: u8,
    pub selected: bool,
    pub has_children: bool,
    pub hidden: bool,
    pub locked: bool,
    pub collapsed: bool,
    pub hovered: bool,
    /// True when the row can host children (Frame/Group); gates
    /// the middle drag-Into band.
    pub is_container: bool,
    /// Active inline-rename target — paints the input instead.
    pub renaming: bool,
}

/// Pages-section row.
#[derive(Debug, Clone)]
pub struct PageItem {
    pub page_index: usize,
    pub label: String,
    pub active: bool,
    pub hovered: bool,
    pub renaming: bool,
}

pub struct LayerPanel {
    pub id: WidgetId,
    pub pages: Vec<PageItem>,
    pub items: Vec<LayerItem>,
    pub theme: Theme,
    pub pages_label: &'static str,
    pub layers_label: &'static str,
    pub drop_target: Option<DropTarget>,
    pub drag_ghost: Option<(LayerItem, f32)>,
    pub now_ms: u64,
    pub caret_anchor_ms: u64,
}

impl LayerPanel {
    pub fn from_document(doc: &Document) -> Self {
        let pages = doc
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| PageItem {
                page_index: i,
                label: rename_override_page(doc, i).unwrap_or_else(|| p.name.clone()),
                active: i == doc.active_page_index,
                hovered: doc.ui.hovered_page_index == Some(i),
                renaming: rename_is_page(doc, i),
            })
            .collect();
        let mut items = Vec::new();
        if let Some(page) = doc.active_page() {
            for child in &page.children {
                walk(child, doc.selected, doc.ui.hovered_layer_id, 0, &mut items);
            }
        }
        for item in &mut items {
            if let Some(draft) = rename_override_layer(doc, item.node_id) {
                item.label = draft;
                item.renaming = true;
            }
        }
        Self {
            id: WidgetId::new(1000),
            pages,
            items,
            theme: doc.theme(),
            pages_label: doc.t("pages.title"),
            layers_label: doc.t("layers.title"),
            drop_target: None,
            drag_ghost: None,
            now_ms: 0,
            caret_anchor_ms: 0,
        }
    }

    /// Floating ghost row for the dragged source — host paints it
    /// at the cursor's y. None when the source isn't on the
    /// active page.
    pub fn ghost_item_for(doc: &Document, source: NodeId) -> Option<LayerItem> {
        let node = doc.active_page()?.find(source)?;
        Some(LayerItem {
            node_id: node.id,
            label: node.name.clone(),
            kind_label: node.kind.label().to_string(),
            icon: icon_for_kind(&node.kind),
            depth: 0,
            selected: false,
            has_children: !node.children.is_empty(),
            hidden: node.hidden,
            locked: node.locked,
            collapsed: node.collapsed,
            hovered: false,
            is_container: matches!(
                node.kind,
                crate::document::NodeKind::Frame | crate::document::NodeKind::Group
            ),
            renaming: false,
        })
    }

    /// Panel for a drag-in-progress — layer list excludes the
    /// dragged source's subtree, mirroring the post-commit layout
    /// so `drop_target_at` returns y values that match where the
    /// source lands after reorder.
    pub fn from_document_with_drag_source(doc: &Document, drag_source: NodeId) -> Self {
        let pages = doc
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| PageItem {
                page_index: i,
                label: rename_override_page(doc, i).unwrap_or_else(|| p.name.clone()),
                active: i == doc.active_page_index,
                hovered: doc.ui.hovered_page_index == Some(i),
                renaming: rename_is_page(doc, i),
            })
            .collect();
        let mut items = Vec::new();
        if let Some(page) = doc.active_page() {
            for child in &page.children {
                walk_excluding(
                    child,
                    doc.selected,
                    doc.ui.hovered_layer_id,
                    drag_source,
                    0,
                    &mut items,
                );
            }
        }
        Self {
            id: WidgetId::new(1000),
            pages,
            items,
            theme: doc.theme(),
            pages_label: doc.t("pages.title"),
            layers_label: doc.t("layers.title"),
            drop_target: None,
            drag_ghost: None,
            now_ms: 0,
            caret_anchor_ms: 0,
        }
    }

    pub fn empty() -> Self {
        Self {
            id: WidgetId::new(1000),
            pages: Vec::new(),
            items: Vec::new(),
            theme: Theme::dark(),
            pages_label: "Pages",
            layers_label: "Layers",
            drop_target: None,
            drag_ghost: None,
            now_ms: 0,
            caret_anchor_ms: 0,
        }
    }

    pub(crate) fn intrinsic_height(&self) -> f32 {
        let pages_h = SECTION_HEADER_HEIGHT + self.pages.len() as f32 * PAGE_ROW_HEIGHT;
        let layers_h = SECTION_HEADER_HEIGHT + self.items.len().max(1) as f32 * LAYER_ROW_HEIGHT;
        pages_h + SECTION_GAP + layers_h + 16.0
    }

    /// Drop target for a drag-in-progress. Over a row: top-25 %
    /// Before, middle 50 % Into (container rows only), bottom 25 %
    /// After. Below-rows area: After-last. Outside / above rows:
    /// None.
    pub fn drop_target_at(&self, rect: Rect, point: Point2D) -> Option<DropTarget> {
        if !rect_contains(rect, point) {
            return None;
        }
        let layers_top = rect.origin.y
            + 8.0
            + SECTION_HEADER_HEIGHT
            + self.pages.len() as f32 * PAGE_ROW_HEIGHT
            + SECTION_GAP
            + SECTION_HEADER_HEIGHT;
        let mut y = layers_top;
        for item in &self.items {
            let row_top = y;
            let row_bottom = y + LAYER_ROW_HEIGHT;
            if point.y >= row_top && point.y <= row_bottom {
                // Three-zone split for container rows: top 25 % →
                // Before, middle 50 % → Into (child of anchor),
                // bottom 25 % → After. Non-container rows fall back
                // to two-zone Before/After (no middle band).
                let local = point.y - row_top;
                let position = if item.is_container {
                    if local < LAYER_ROW_HEIGHT * 0.25 {
                        DropPosition::Before
                    } else if local > LAYER_ROW_HEIGHT * 0.75 {
                        DropPosition::After
                    } else {
                        DropPosition::Into
                    }
                } else if local < LAYER_ROW_HEIGHT / 2.0 {
                    DropPosition::Before
                } else {
                    DropPosition::After
                };
                let indicator_y = match position {
                    DropPosition::Before => row_top,
                    DropPosition::After => row_bottom,
                    // For Into the host paints a row-outline
                    // highlight rather than a horizontal line —
                    // `indicator_y` is the row TOP so the paint
                    // helper can derive the full row rect.
                    DropPosition::Into => row_top,
                };
                return Some(DropTarget {
                    anchor: item.node_id,
                    position,
                    indicator_y,
                });
            }
            y = row_bottom;
        }
        // Cursor is below the last row but still inside the panel —
        // drop at end (anchor = last layer, position = After). `y`
        // already points at the bottom of the final row, which is
        // exactly where the indicator paints.
        if point.y > layers_top {
            if let Some(last) = self.items.last() {
                return Some(DropTarget {
                    anchor: last.node_id,
                    position: DropPosition::After,
                    indicator_y: y,
                });
            }
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
                // × delete button — only hit-tested when the row is
                // hovered (paint shows it under the same gate). 14
                // px icon, right-aligned with 4 px slop.
                if page.hovered {
                    let close_x = rect.origin.x + rect.size.x - ROW_PAD_X - 14.0;
                    let close_y = y + (PAGE_ROW_HEIGHT - 14.0) / 2.0;
                    let slop = 4.0;
                    if point.x >= close_x - slop
                        && point.x <= close_x + 14.0 + slop
                        && point.y >= close_y - slop
                        && point.y <= close_y + 14.0 + slop
                    {
                        return Some(LayerPanelHit::DeletePage(page.page_index));
                    }
                }
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
    /// Click on the trailing × on a page row — host should
    /// remove the page via `Document::remove_page(idx)`. Only
    /// hit-tested when the row's `hovered` flag is true so the
    /// affordance stays click-only-on-hover.
    DeletePage(usize),
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
    /// Drop becomes the LAST child of the anchor (a container —
    /// Frame / Group). Cursor in the middle band of the row,
    /// only when the anchor's `NodeKind` can host children.
    Into,
}

/// Drop-target result: anchor node, drop position, and the
/// panel-local y where the indicator paints.
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
            } else if page.hovered {
                cx.backend.fill_round_rect(row, 6.0, self.theme.muted);
            }
            let label_x = row.origin.x + 12.0;
            let label_max_x = rect.origin.x + rect.size.x - ROW_PAD_X - 18.0;
            let available_w = (label_max_x - label_x).max(0.0);
            if page.renaming {
                paint_rename_input(
                    cx,
                    &self.theme,
                    &page.label,
                    label_x,
                    y + 2.0,
                    available_w,
                    self.now_ms,
                    self.caret_anchor_ms,
                );
            } else {
                let display = truncate_to_fit(&page.label, ROW_FONT, available_w);
                let label = TextLayout::single_run(
                    &display,
                    "system-ui",
                    ROW_FONT,
                    to_jian_color(if page.active {
                        self.theme.foreground
                    } else {
                        self.theme.muted_foreground
                    }),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend
                    .draw_text(&label, Point2D::new(label_x, row.origin.y + 19.0));
            }
            // Hover-reveal × delete button on the trailing edge —
            // matches TS page-row hover affordance. Hit-test geometry
            // mirrors the paint exactly.
            if page.hovered {
                let close_x = rect.origin.x + rect.size.x - ROW_PAD_X - 14.0;
                let close_y = y + (PAGE_ROW_HEIGHT - 14.0) / 2.0;
                draw_icon(
                    cx.backend,
                    Icon::Close,
                    Point2D::new(close_x, close_y),
                    14.0,
                    self.theme.muted_foreground,
                    1.4,
                );
            }
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
            } else if item.hovered {
                cx.backend.fill_round_rect(row, 6.0, self.theme.muted);
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
            let label_x = icon_x + 20.0;
            // Reserve space for trailing eye + lock so the label
            // doesn't overlap them when the row is hovered/selected.
            let label_max_x = row.origin.x + row.size.x - 8.0 - 14.0 - 22.0 - 4.0;
            let available_w = (label_max_x - label_x).max(0.0);
            if item.renaming {
                paint_rename_input(
                    cx,
                    &self.theme,
                    &item.label,
                    label_x,
                    row.origin.y,
                    available_w,
                    self.now_ms,
                    self.caret_anchor_ms,
                );
            } else {
                let display = truncate_to_fit(&item.label, ROW_FONT, available_w);
                let label = TextLayout::single_run(
                    &display,
                    "system-ui",
                    ROW_FONT,
                    to_jian_color(label_color),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend
                    .draw_text(&label, Point2D::new(label_x, row.origin.y + 17.0));
            }
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

        // Drop-indicator — paints AFTER row chrome so it sits on top.
        // Before/After: a 2 px horizontal line between rows.
        // Into: a 1.5 px outline around the target row (signals
        // "drop becomes child of this container").
        if let Some(drop) = self.drop_target {
            match drop.position {
                DropPosition::Before | DropPosition::After => {
                    let indicator_rect = Rect {
                        origin: Point2D::new(rect.origin.x + ROW_PAD_X, drop.indicator_y - 1.0),
                        size: Point2D::new(rect.size.x - ROW_PAD_X * 2.0, 2.0),
                    };
                    cx.backend.fill_rect(indicator_rect, self.theme.primary);
                }
                DropPosition::Into => {
                    // 1.5 px outline rect spanning the full row.
                    let outline = Rect {
                        origin: Point2D::new(rect.origin.x + 6.0, drop.indicator_y + 2.0),
                        size: Point2D::new(rect.size.x - 12.0, LAYER_ROW_HEIGHT - 4.0),
                    };
                    cx.backend
                        .stroke_round_rect(outline, 6.0, self.theme.primary, 1.5);
                }
            }
        }

        if let Some((ghost, cursor_y)) = &self.drag_ghost {
            paint_drag_ghost(cx, &self.theme, ghost, *cursor_y, rect);
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Tree);
        node.set_label("Layers");
        node
    }
}
