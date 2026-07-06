//! `LayerPanel` — left-rail document tree (Pages + Layers sections).
//!
//! Phase 6 migration: the panel's display model (page rows + the
//! depth-flattened layer rows) is built directly from
//! `op_editor_core::EditorState` — it walks the canonical `PenNode`
//! tree on `EditorState.doc`. The widget keeps a shell-core
//! `document::NodeId` in its hit-test surface; the hosts' input
//! path wraps it into an `op-editor-core` id at the call boundary.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::layer_panel_walkers::{
    apply_layer_rename, icon_for_node, kind_label, layer_regions, layers_content_width,
    pages_content_width, pages_from_state, visible_row_range, walk, walk_excluding,
    LayerRegionInput, LayerRegions, LayerScrollSnapshot, RenameView, WalkCx,
};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use jian_core::text_input::TextInputState;
use op_editor_core::NodeId;

use jian_ops_schema::node::PenNode;
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;

/// Outer panel width; host layout uses this.
pub const LAYER_PANEL_WIDTH: f32 = 240.0;

/// Translate a chrome string key against the active editor locale.
fn t(ui: &EditorUiState, key: &'static str) -> &'static str {
    crate::widgets::editor_state_ext::translate(ui, key)
}

pub(crate) const SECTION_HEADER_HEIGHT: f32 = 28.0;
pub(crate) const PAGE_ROW_HEIGHT: f32 = 32.0;
pub(crate) const LAYER_ROW_HEIGHT: f32 = 28.0;
pub(crate) const ROW_PAD_X: f32 = 12.0;
pub(crate) const SECTION_GAP: f32 = 8.0;
use crate::widgets::layer_panel_paint::{
    paint_drag_ghost, paint_rename_input, paint_section_header, truncate_to_fit, ROW_FONT,
};

/// One row in the layers tree — flat depth-walked view.
#[derive(Debug, Clone)]
pub struct LayerItem {
    pub node_id: NodeId,
    pub label: String,
    pub kind_label: String,
    pub icon: Icon,
    pub depth: usize,
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
    /// Reusable COMPONENT definition — Diamond icon + #a855f7 tint.
    pub is_reusable: bool,
    /// Component INSTANCE (`Ref`) — Diamond icon + #9281f7 tint.
    pub is_instance: bool,
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
    pub rename_input: Option<TextInputState>,
    /// Scroll state for the bounded Pages / Layers regions.
    pub pages_scroll: LayerScrollSnapshot,
    pub layers_scroll: LayerScrollSnapshot,
}

impl LayerPanel {
    /// Build the panel from the editor state — walks the canonical
    /// `PenNode` tree on the active page.
    pub fn from_editor(state: &EditorState) -> Self {
        let rename = RenameView::from_state(state);
        let pages = pages_from_state(state, &rename);
        let cx = WalkCx::from_state(state);
        let mut items = Vec::new();
        for child in state.active_children() {
            walk(child, &cx, 0, &mut items);
        }
        apply_layer_rename(&mut items, &rename);
        let pages_content_width = pages_content_width(&pages, LAYER_PANEL_WIDTH);
        let layers_content_width = layers_content_width(&items, LAYER_PANEL_WIDTH);
        Self {
            id: WidgetId::new(1000),
            pages,
            items,
            theme: theme_for(&state.editor_ui),
            pages_label: t(&state.editor_ui, "pages.title"),
            layers_label: t(&state.editor_ui, "layers.title"),
            drop_target: None,
            drag_ghost: None,
            now_ms: 0,
            rename_input: state.ui.layer_rename.as_ref().map(|r| r.input.clone()),
            pages_scroll: LayerScrollSnapshot::new(
                state.editor_ui.layer_pages_scroll,
                state.editor_ui.layer_pages_h_scroll,
                pages_content_width,
            ),
            layers_scroll: LayerScrollSnapshot::new(
                state.editor_ui.layer_layers_scroll,
                state.editor_ui.layer_layers_h_scroll,
                layers_content_width,
            ),
        }
    }

    /// Floating ghost row for the dragged source — host paints it
    /// at the cursor's y. None when the source isn't on the
    /// active page.
    pub fn ghost_item_for(state: &EditorState, source: &NodeId) -> Option<LayerItem> {
        let node = op_editor_core::walkers::find_node(state.active_children(), source)?;
        let base = node.base();
        Some(LayerItem {
            node_id: source.clone(),
            // Name-or-kind fallback, same as `item_for` (TS `name ?? type`)
            // so a nameless node's drag ghost isn't blank.
            label: base
                .name
                .clone()
                .unwrap_or_else(|| kind_label(node).to_string()),
            kind_label: kind_label(node).to_string(),
            icon: icon_for_node(node),
            depth: 0,
            selected: false,
            has_children: node.children().map(|c| !c.is_empty()).unwrap_or(false),
            hidden: base.visible == Some(false),
            locked: base.locked.unwrap_or(false),
            collapsed: state.editor_ui.collapsed_layers.contains(source),
            hovered: false,
            // Reparent-into drop targets match TS CONTAINER_TYPES
            // (layer-panel.tsx:14 — frame/group/rectangle/ref).
            is_container: matches!(
                node,
                PenNode::Frame(_) | PenNode::Group(_) | PenNode::Rectangle(_) | PenNode::Ref(_)
            ),
            renaming: false,
            is_reusable: matches!(node, PenNode::Frame(f) if f.reusable == Some(true)),
            is_instance: matches!(node, PenNode::Ref(_)),
        })
    }

    /// Panel for a drag-in-progress — layer list excludes the
    /// dragged source's subtree, mirroring the post-commit layout
    /// so `drop_target_at` returns y values that match where the
    /// source lands after reorder.
    pub fn from_editor_with_drag_source(state: &EditorState, drag_source: &NodeId) -> Self {
        let rename = RenameView::from_state(state);
        let pages = pages_from_state(state, &rename);
        let cx = WalkCx::from_state(state);
        let mut items = Vec::new();
        for child in state.active_children() {
            walk_excluding(child, &cx, drag_source, 0, &mut items);
        }
        apply_layer_rename(&mut items, &rename);
        let pages_content_width = pages_content_width(&pages, LAYER_PANEL_WIDTH);
        let layers_content_width = layers_content_width(&items, LAYER_PANEL_WIDTH);
        Self {
            id: WidgetId::new(1000),
            pages,
            items,
            theme: theme_for(&state.editor_ui),
            pages_label: t(&state.editor_ui, "pages.title"),
            layers_label: t(&state.editor_ui, "layers.title"),
            drop_target: None,
            drag_ghost: None,
            now_ms: 0,
            rename_input: state.ui.layer_rename.as_ref().map(|r| r.input.clone()),
            pages_scroll: LayerScrollSnapshot::new(
                state.editor_ui.layer_pages_scroll,
                state.editor_ui.layer_pages_h_scroll,
                pages_content_width,
            ),
            layers_scroll: LayerScrollSnapshot::new(
                state.editor_ui.layer_layers_scroll,
                state.editor_ui.layer_layers_h_scroll,
                layers_content_width,
            ),
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
            rename_input: None,
            pages_scroll: LayerScrollSnapshot::default(),
            layers_scroll: LayerScrollSnapshot::default(),
        }
    }

    pub(crate) fn intrinsic_height(&self) -> f32 {
        let pages_h = SECTION_HEADER_HEIGHT + self.pages.len() as f32 * PAGE_ROW_HEIGHT;
        let layers_h = SECTION_HEADER_HEIGHT + self.items.len().max(1) as f32 * LAYER_ROW_HEIGHT;
        pages_h + SECTION_GAP + layers_h + 16.0
    }

    /// Bounded Pages / Layers scroll-region geometry for `rect` —
    /// the single source paint + hit-test + drop-target derive from.
    /// `pub` so the host's wheel handler can route a scroll to the
    /// region under the cursor.
    pub fn regions(&self, rect: Rect) -> LayerRegions {
        layer_regions(LayerRegionInput {
            rect,
            pages_len: self.pages.len(),
            items_len: self.items.len(),
            pages: self.pages_scroll,
            layers: self.layers_scroll,
        })
    }

    /// Return the layer-list scroll offset that keeps `node_id`'s
    /// row visible inside the clipped Layers viewport.
    pub fn layers_offset_revealing(&self, rect: Rect, node_id: &NodeId) -> Option<f32> {
        let index = self
            .items
            .iter()
            .position(|item| &item.node_id == node_id)?;
        let r = self.regions(rect);
        if r.layers_view_h <= 0.0 {
            return None;
        }
        let row_top = index as f32 * LAYER_ROW_HEIGHT;
        let row_bottom = row_top + LAYER_ROW_HEIGHT;
        let view_top = r.layers.offset;
        let view_bottom = view_top + r.layers_view_h;
        let next = if row_top < view_top {
            row_top
        } else if row_bottom > view_bottom {
            row_bottom - r.layers_view_h
        } else {
            view_top
        };
        Some(next.clamp(0.0, r.layers.max_offset))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerPanelHit {
    Page(usize),
    Layer(NodeId),
    ToggleHidden(NodeId),
    ToggleLocked(NodeId),
    ToggleCollapsed(NodeId),
    AddPage,
    DeletePage(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPosition {
    Before,
    After,
    Into,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DropTarget {
    pub anchor: NodeId,
    pub position: DropPosition,
    pub indicator_y: f32,
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

        let r = self.regions(rect);
        let mut y = r.pages_header_y;

        // Pages section header.
        paint_section_header(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            rect.size.x,
            self.pages_label,
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
        // Page rows — clipped + scrolled inside the bounded viewport.
        cx.backend.save();
        cx.backend.clip_rect(Rect {
            origin: Point2D::new(rect.origin.x, r.pages_rows_top),
            size: Point2D::new(rect.size.x, r.pages_view_h),
        });
        for index in visible_row_range(
            self.pages.len(),
            r.pages.offset,
            r.pages_view_h,
            PAGE_ROW_HEIGHT,
        ) {
            let page = &self.pages[index];
            y = r.pages_rows_top - r.pages.offset + index as f32 * PAGE_ROW_HEIGHT;
            let row = Rect {
                origin: Point2D::new(rect.origin.x + 6.0, y + 2.0),
                size: Point2D::new(rect.size.x - 12.0, PAGE_ROW_HEIGHT - 4.0),
            };
            if page.active {
                cx.backend
                    .fill_round_rect(row, 6.0, self.theme.row_selected);
            } else if page.hovered {
                cx.backend
                    .fill_round_rect(row, 6.0, self.theme.button_hover);
            }
            let label_x = row.origin.x + 12.0;
            let label_max_x = rect.origin.x + rect.size.x - ROW_PAD_X - 18.0;
            let available_w = (label_max_x - label_x).max(0.0);
            if page.renaming {
                paint_rename_input(
                    cx,
                    &self.theme,
                    self.rename_input.as_ref().expect("renaming row has input"),
                    label_x,
                    y + 2.0,
                    available_w,
                    self.now_ms,
                );
            } else {
                let display = truncate_to_fit(&page.label, ROW_FONT, available_w);
                let label = TextLayout::single_run(
                    &display,
                    "system-ui",
                    ROW_FONT,
                    (if page.active {
                        self.theme.foreground
                    } else {
                        self.theme.muted_foreground
                    })
                    .to_jian(),
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
        }
        cx.backend.restore();

        y = r.layers_header_y;
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
            self.layers_label,
        );
        // Layer rows — clipped + scrolled inside the bounded viewport.
        cx.backend.save();
        cx.backend.clip_rect(Rect {
            origin: Point2D::new(rect.origin.x, r.layers_rows_top),
            size: Point2D::new(rect.size.x, r.layers_view_h),
        });
        for index in visible_row_range(
            self.items.len(),
            r.layers.offset,
            r.layers_view_h,
            LAYER_ROW_HEIGHT,
        ) {
            let item = &self.items[index];
            y = r.layers_rows_top - r.layers.offset + index as f32 * LAYER_ROW_HEIGHT;
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
                cx.backend
                    .fill_round_rect(row, 6.0, self.theme.button_hover);
            }

            let indent = ROW_PAD_X + item.depth as f32 * 12.0;
            let dim = |c: Color, factor: f32| -> Color {
                Color {
                    r: c.r,
                    g: c.g,
                    b: c.b,
                    a: c.a * factor,
                }
            };
            let dim_factor = if item.hidden { 0.45 } else { 1.0 };
            // Component / instance rows tint purple like TS
            // (`text-purple-400` #a855f7 / instance #9281f7);
            // selection still wins so the selected row reads as one.
            let component_tint = if item.is_reusable {
                Some(crate::widgets::property_panel_inputs::COMPONENT_ACCENT)
            } else if item.is_instance {
                Some(crate::widgets::property_panel_inputs::INSTANCE_ACCENT)
            } else {
                None
            };
            let icon_color = if item.selected {
                dim(self.theme.primary, dim_factor)
            } else if let Some(tint) = component_tint {
                dim(tint, dim_factor)
            } else {
                dim(self.theme.muted_foreground, dim_factor)
            };
            cx.backend.save();
            cx.backend.clip_rect(row);
            cx.backend
                .translate(Point2D::new(-r.layers.horizontal_offset, 0.0));
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
            let icon_x = row.origin.x + indent + 18.0;
            draw_icon(
                cx.backend,
                item.icon,
                Point2D::new(icon_x, row.origin.y + 6.0),
                14.0,
                icon_color,
                1.4,
            );
            let label_color = if item.selected {
                dim(self.theme.primary, dim_factor)
            } else if let Some(tint) = component_tint {
                dim(tint, dim_factor)
            } else {
                dim(self.theme.card_foreground, dim_factor)
            };
            let label_x = icon_x + 20.0;
            let label_max_x = row.origin.x + r.layers.content_width - 16.0;
            let available_w = (label_max_x - label_x).max(0.0);
            if item.renaming {
                paint_rename_input(
                    cx,
                    &self.theme,
                    self.rename_input.as_ref().expect("renaming row has input"),
                    label_x,
                    row.origin.y,
                    available_w,
                    self.now_ms,
                );
            } else {
                let display = truncate_to_fit(&item.label, ROW_FONT, available_w);
                let label = TextLayout::single_run(
                    &display,
                    "system-ui",
                    ROW_FONT,
                    (label_color).to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend
                    .draw_text(&label, Point2D::new(label_x, row.origin.y + 17.0));
            }
            cx.backend.restore();
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
            let state_color = |r, g, b| Color { r, g, b, a: 1.0 };
            let eye_hidden = state_color(0.98039216, 0.8, 0.08235294);
            let lock_locked = state_color(0.92, 0.49, 0.20);
            let eye_color = if item.hidden {
                eye_hidden
            } else {
                trailing_default
            };
            let lock_color = if item.locked {
                lock_locked
            } else {
                trailing_default
            };
            let trailing_size = 12.0;
            let trailing_stroke = 1.2;
            let trailing_y = row.origin.y + 7.0;
            let show_eye = item.hovered;
            let show_lock = item.hovered;
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
        }

        // Drop-indicator — paints AFTER row chrome so it sits on top.
        // Before/After: a 2 px horizontal line between rows.
        // Into: a 1.5 px outline around the target row (signals
        // "drop becomes child of this container").
        if let Some(drop) = &self.drop_target {
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

        cx.backend.restore();

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
