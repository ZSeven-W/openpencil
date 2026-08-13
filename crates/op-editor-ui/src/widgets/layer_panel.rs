//! `LayerPanel` — left-rail document tree (Pages + Layers sections).
//!
//! Its page rows and depth-flattened layer rows are built from the canonical
//! `PenNode` tree on `op_editor_core::EditorState`.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::layer_panel_cache::{self, CachedLayerRows};
use crate::widgets::layer_panel_metrics::{
    add_page_target, collapse_target, delete_page_target, glyph_rect_in, layer_action_targets,
    layer_drag_target, layer_node_icon_x, LayerPanelMetrics,
};
use crate::widgets::layer_panel_walkers::{
    apply_layer_rename, icon_for_node, kind_label, layer_regions, layers_content_width,
    pages_content_width, pages_from_state, visible_row_range, walk, walk_excluding,
    LayerRegionInput, LayerRegions, LayerScrollSnapshot, RenameView, WalkCx,
};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use jian_core::text_input::TextInputState;
use op_editor_core::{NodeId, SelectionState};
use std::rc::Rc;

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

pub(crate) const ROW_PAD_X: f32 = 12.0;
use crate::widgets::layer_panel_paint::{
    layer_content_clip_rect_with_metrics, layer_label_available_width_with_metrics,
    paint_drag_ghost, paint_layer_action_backing, paint_layer_drag_handle,
    paint_rename_input_with_metrics, paint_section_header_with_metrics, truncate_to_fit,
    truncate_to_fit_measured,
};

/// One row in the layers tree — flat depth-walked view.
#[derive(Debug, Clone)]
pub struct LayerItem {
    pub node_id: NodeId,
    pub label: String,
    pub kind_label: String,
    pub icon: Icon,
    pub depth: usize,
    pub has_children: bool,
    pub hidden: bool,
    pub locked: bool,
    pub collapsed: bool,
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
    pub renaming: bool,
}

pub struct LayerPanel {
    pub id: WidgetId,
    /// Cached, styling-neutral row models (shared via `Rc`; a cache hit
    /// is a refcount bump, not a per-row re-allocation).
    pub pages: Rc<Vec<PageItem>>,
    pub items: Rc<Vec<LayerItem>>,
    pub theme: Theme,
    pub(crate) metrics: LayerPanelMetrics,
    pub pages_label: &'static str,
    pub layers_label: &'static str,
    pub drop_target: Option<DropTarget>,
    pub drag_ghost: Option<(LayerItem, f32)>,
    pub now_ms: u64,
    pub rename_input: Option<TextInputState>,
    /// Live styling overlay — selection + hover are applied at paint /
    /// hit-test time so they never invalidate the cached row model.
    pub selection: SelectionState,
    pub hovered_layer: Option<NodeId>,
    pub hovered_page: Option<usize>,
    /// Scroll state for the bounded Pages / Layers regions.
    pub pages_scroll: LayerScrollSnapshot,
    pub layers_scroll: LayerScrollSnapshot,
}

impl LayerPanel {
    /// Allocate a process-unique owner id for a persistent host's
    /// LayerPanel cache slot (mirrors `AIChatPlaceholder::next_owner`).
    /// A host pulls an id at construction, passes it to
    /// [`Self::from_editor_owned`] on paint and event paths, and pulls a FRESH
    /// id after every whole-document replacement
    /// (`force_rotate_layer_panel_owner`) — ids are stable only between
    /// replacements.
    pub fn next_layer_panel_owner() -> u64 {
        layer_panel_cache::next_owner()
    }

    /// Build the panel from the editor state via the CACHE-BYPASS path —
    /// always a fresh walk that never touches the shared slot. Used primarily
    /// by unit tests and one-off callers without a persistent owner. Hosts call
    /// [`Self::from_editor_owned`] for paint and event-time hit tests so those
    /// paths share the same row model.
    pub fn from_editor(state: &EditorState) -> Self {
        Self::from_editor_owned(state, layer_panel_cache::UNOWNED)
    }

    /// Owner-scoped build for a persistent host's paint and event paths.
    /// Resolves the row model through the thread-local cache scoped to
    /// `owner`; rebuilds only when the document revision / active page /
    /// collapsed set / rename draft change. Selection + hover are applied as
    /// a live overlay (see `layer_panel_cache`), so a selection-/hover-only
    /// change reuses the cached rows.
    pub fn from_editor_owned(state: &EditorState, owner: u64) -> Self {
        let metrics = LayerPanelMetrics::for_ui(&state.editor_ui);
        let rows =
            layer_panel_cache::resolve_owned(owner, state, || build_layer_rows(state, metrics));
        Self::assemble(
            state,
            metrics,
            rows.pages.clone(),
            rows.items.clone(),
            rows.pages_content_width,
            rows.layers_content_width,
        )
    }

    /// Live selection overlay — true when `id` is in the editor
    /// selection. Paint / hit-test read this instead of a baked flag so
    /// selection changes never invalidate the cached row model.
    pub fn is_row_selected(&self, id: &NodeId) -> bool {
        self.selection.contains(id)
    }

    /// Live hover overlay for a layer row.
    pub fn is_row_hovered(&self, id: &NodeId) -> bool {
        self.hovered_layer.as_ref() == Some(id)
    }

    /// Live hover overlay for a page row.
    pub fn is_page_hovered(&self, page_index: usize) -> bool {
        self.hovered_page == Some(page_index)
    }

    /// Assemble a panel from an already-built (cached or fresh) row model,
    /// stamping the live styling overlay + scroll snapshots + chrome.
    fn assemble(
        state: &EditorState,
        metrics: LayerPanelMetrics,
        pages: Rc<Vec<PageItem>>,
        items: Rc<Vec<LayerItem>>,
        pages_content_width: f32,
        layers_content_width: f32,
    ) -> Self {
        Self {
            id: WidgetId::new(1000),
            pages,
            items,
            theme: theme_for(&state.editor_ui),
            metrics,
            pages_label: t(&state.editor_ui, "pages.title"),
            layers_label: t(&state.editor_ui, "layers.title"),
            drop_target: None,
            drag_ghost: None,
            now_ms: 0,
            rename_input: state.ui.layer_rename.as_ref().map(|r| r.input.clone()),
            selection: state.selection.clone(),
            hovered_layer: state.editor_ui.hovered_layer_id.clone(),
            hovered_page: state.editor_ui.hovered_page_index,
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
            has_children: node.children().map(|c| !c.is_empty()).unwrap_or(false),
            hidden: base.visible == Some(false),
            locked: base.locked.unwrap_or(false),
            collapsed: state.editor_ui.collapsed_layers.contains(source),
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
        // Active drag rows bypass the idle cache because their source subtree
        // is excluded; selection and hover still overlay via `assemble`.
        let rename = RenameView::from_state(state);
        let pages = pages_from_state(state, &rename);
        let cx = WalkCx::from_state(state);
        let mut items = Vec::new();
        for child in state.active_children() {
            walk_excluding(child, &cx, drag_source, 0, &mut items);
        }
        apply_layer_rename(&mut items, &rename);
        let metrics = LayerPanelMetrics::for_ui(&state.editor_ui);
        let pages_content_width = pages_content_width(&pages, LAYER_PANEL_WIDTH, metrics);
        let layers_content_width = layers_content_width(&items, LAYER_PANEL_WIDTH, metrics);
        Self::assemble(
            state,
            metrics,
            Rc::new(pages),
            Rc::new(items),
            pages_content_width,
            layers_content_width,
        )
    }

    pub fn empty() -> Self {
        Self {
            id: WidgetId::new(1000),
            pages: Rc::new(Vec::new()),
            items: Rc::new(Vec::new()),
            theme: Theme::dark(),
            metrics: LayerPanelMetrics::DESKTOP,
            // No editor state (and therefore no locale) is reachable in
            // the empty skeleton — route through the canonical English
            // table instead of hardcoding literals.
            pages_label: op_i18n::translate(op_editor_core::Locale::EnUs, "pages.title"),
            layers_label: op_i18n::translate(op_editor_core::Locale::EnUs, "layers.title"),
            drop_target: None,
            drag_ghost: None,
            now_ms: 0,
            rename_input: None,
            selection: SelectionState::empty(),
            hovered_layer: None,
            hovered_page: None,
            pages_scroll: LayerScrollSnapshot::default(),
            layers_scroll: LayerScrollSnapshot::default(),
        }
    }

    pub(crate) fn intrinsic_height(&self) -> f32 {
        let pages_h = self.metrics.section_header_height
            + self.pages.len() as f32 * self.metrics.page_row_height;
        let layers_h = self.metrics.section_header_height
            + self.items.len().max(1) as f32 * self.metrics.layer_row_height;
        pages_h + self.metrics.section_gap + layers_h + 16.0
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
            metrics: self.metrics,
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
        let row_top = index as f32 * self.metrics.layer_row_height;
        let row_bottom = row_top + self.metrics.layer_row_height;
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

/// Walk the active page into the styling-neutral row model the cache
/// stores: page rows + depth-flattened layer rows + both content widths.
/// This is the expensive per-frame work (tree walk + per-node `String`
/// allocation + label measurement) the cache exists to skip.
fn build_layer_rows(state: &EditorState, metrics: LayerPanelMetrics) -> CachedLayerRows {
    let rename = RenameView::from_state(state);
    let pages = pages_from_state(state, &rename);
    let cx = WalkCx::from_state(state);
    let mut items = Vec::new();
    for child in state.active_children() {
        walk(child, &cx, 0, &mut items);
    }
    apply_layer_rename(&mut items, &rename);
    let pages_content_width = pages_content_width(&pages, LAYER_PANEL_WIDTH, metrics);
    let layers_content_width = layers_content_width(&items, LAYER_PANEL_WIDTH, metrics);
    CachedLayerRows {
        pages: Rc::new(pages),
        items: Rc::new(items),
        pages_content_width,
        layers_content_width,
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
        paint_section_header_with_metrics(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            rect.size.x,
            self.pages_label,
            self.metrics,
        );
        // "+" add-page affordance, top-right of header row.
        let plus = glyph_rect_in(
            add_page_target(rect, y, self.metrics),
            self.metrics.glyph_size,
        );
        draw_icon(
            cx.backend,
            Icon::Plus,
            plus.origin,
            self.metrics.glyph_size,
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
            self.metrics.page_row_height,
        ) {
            let page = &self.pages[index];
            y = r.pages_rows_top - r.pages.offset + index as f32 * self.metrics.page_row_height;
            let row = Rect {
                origin: Point2D::new(rect.origin.x + 6.0, y + 2.0),
                size: Point2D::new(rect.size.x - 12.0, self.metrics.page_row_height - 4.0),
            };
            let page_hovered = self.is_page_hovered(index);
            if page.active {
                cx.backend
                    .fill_round_rect(row, 6.0, self.theme.row_selected);
            } else if page_hovered {
                cx.backend
                    .fill_round_rect(row, 6.0, self.theme.button_hover);
            }
            let label_x = row.origin.x + 12.0;
            let delete_target = delete_page_target(rect, y, self.metrics);
            let label_max_x = if self.metrics.touch {
                delete_target.origin.x - 4.0
            } else {
                rect.origin.x + rect.size.x - self.metrics.row_pad_x - 18.0
            };
            let available_w = (label_max_x - label_x).max(0.0);
            if page.renaming {
                paint_rename_input_with_metrics(
                    cx,
                    &self.theme,
                    self.rename_input.as_ref().expect("renaming row has input"),
                    label_x,
                    y + 2.0,
                    available_w.max(40.0),
                    self.now_ms,
                    self.metrics,
                );
            } else {
                let display = truncate_to_fit(&page.label, self.metrics.row_font, available_w);
                let label = TextLayout::single_run(
                    &display,
                    "system-ui",
                    self.metrics.row_font,
                    (if page.active {
                        self.theme.foreground
                    } else {
                        self.theme.muted_foreground
                    })
                    .to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                let baseline = if self.metrics.touch {
                    jian_widgets::centered_text_baseline_y(row, self.metrics.row_font)
                } else {
                    row.origin.y + 19.0
                };
                cx.backend
                    .draw_text(&label, Point2D::new(label_x, baseline));
            }
            // Hover-reveal × delete button on the trailing edge —
            // matches TS page-row hover affordance. Hit-test geometry
            // mirrors the paint exactly.
            if page_hovered || self.metrics.touch {
                let close = glyph_rect_in(delete_target, self.metrics.glyph_size);
                draw_icon(
                    cx.backend,
                    Icon::Close,
                    close.origin,
                    self.metrics.glyph_size,
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
                origin: Point2D::new(
                    rect.origin.x + self.metrics.row_pad_x,
                    y - self.metrics.section_gap / 2.0,
                ),
                size: Point2D::new(rect.size.x - self.metrics.row_pad_x * 2.0, 1.0),
            },
            self.theme.border,
        );

        // Layers section header.
        paint_section_header_with_metrics(
            cx,
            &self.theme,
            rect.origin.x,
            y,
            rect.size.x,
            self.layers_label,
            self.metrics,
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
            self.metrics.layer_row_height,
        ) {
            let item = &self.items[index];
            // Live styling overlay — never baked into the cached item.
            let selected = self.is_row_selected(&item.node_id);
            let hovered = self.is_row_hovered(&item.node_id);
            y = r.layers_rows_top - r.layers.offset + index as f32 * self.metrics.layer_row_height;
            let row = Rect {
                origin: Point2D::new(rect.origin.x + 6.0, y + 2.0),
                size: Point2D::new(rect.size.x - 12.0, self.metrics.layer_row_height - 4.0),
            };
            if selected {
                // TS uses bg-blue-500/15 + primary text + primary
                // icon for the selected layer row.
                cx.backend
                    .fill_round_rect(row, 6.0, self.theme.row_selected_primary);
            } else if hovered {
                cx.backend
                    .fill_round_rect(row, 6.0, self.theme.button_hover);
            }

            let indent = self.metrics.row_pad_x + item.depth as f32 * 12.0;
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
            let icon_color = if selected {
                dim(self.theme.primary, dim_factor)
            } else if let Some(tint) = component_tint {
                dim(tint, dim_factor)
            } else {
                dim(self.theme.muted_foreground, dim_factor)
            };
            let content_clip =
                layer_content_clip_rect_with_metrics(row, item.renaming, self.metrics);
            cx.backend.save();
            cx.backend.clip_rect(content_clip);
            cx.backend
                .translate(Point2D::new(-r.layers.horizontal_offset, 0.0));
            if item.has_children {
                let chev_icon = if item.collapsed {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronDown
                };
                let chevron = glyph_rect_in(
                    collapse_target(row, indent, 0.0, self.metrics),
                    self.metrics.glyph_size,
                );
                draw_icon(
                    cx.backend,
                    chev_icon,
                    chevron.origin,
                    self.metrics.glyph_size,
                    icon_color,
                    1.4,
                );
            }
            let icon_x = layer_node_icon_x(row, indent, self.metrics);
            let icon_y = if self.metrics.touch {
                row.origin.y + (row.size.y - self.metrics.glyph_size) / 2.0
            } else {
                row.origin.y + 6.0
            };
            draw_icon(
                cx.backend,
                item.icon,
                Point2D::new(icon_x, icon_y),
                self.metrics.glyph_size,
                icon_color,
                1.4,
            );
            let label_color = if selected {
                dim(self.theme.primary, dim_factor)
            } else if let Some(tint) = component_tint {
                dim(tint, dim_factor)
            } else {
                dim(self.theme.card_foreground, dim_factor)
            };
            let label_x =
                icon_x + self.metrics.glyph_size + if self.metrics.touch { 8.0 } else { 6.0 };
            let available_w = layer_label_available_width_with_metrics(
                row,
                label_x,
                r.layers.horizontal_offset,
                item.renaming,
                self.metrics,
            );
            if item.renaming {
                paint_rename_input_with_metrics(
                    cx,
                    &self.theme,
                    self.rename_input.as_ref().expect("renaming row has input"),
                    label_x,
                    row.origin.y,
                    available_w,
                    self.now_ms,
                    self.metrics,
                );
            } else {
                let display = truncate_to_fit_measured(
                    cx.backend,
                    &item.label,
                    self.metrics.row_font,
                    available_w,
                );
                let label = TextLayout::single_run(
                    &display,
                    "system-ui",
                    self.metrics.row_font,
                    (label_color).to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                let baseline = if self.metrics.touch {
                    jian_widgets::centered_text_baseline_y(row, self.metrics.row_font)
                } else {
                    row.origin.y + 17.0
                };
                cx.backend
                    .draw_text(&label, Point2D::new(label_x, baseline));
            }
            cx.backend.restore();
            let (eye_target, lock_target) = layer_action_targets(row, self.metrics);
            let drag_target = layer_drag_target(row, self.metrics).filter(|_| !item.renaming);
            let eye_icon = if item.hidden { Icon::EyeOff } else { Icon::Eye };
            let lock_icon = if item.locked {
                Icon::Lock
            } else {
                Icon::LockOpen
            };
            let trailing_default = if selected {
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
            let trailing_size = self.metrics.trailing_glyph_size;
            let trailing_stroke = 1.2;
            let eye_glyph = glyph_rect_in(eye_target, trailing_size);
            let lock_glyph = glyph_rect_in(lock_target, trailing_size);
            let show_eye = (hovered || self.metrics.touch) && !item.renaming;
            let show_lock = (hovered || self.metrics.touch) && !item.renaming;
            paint_layer_action_backing(
                cx,
                row,
                self.metrics,
                &self.theme,
                selected,
                hovered,
                drag_target.is_some() || show_eye || show_lock,
            );
            paint_layer_drag_handle(cx, drag_target, trailing_default);
            if show_eye {
                draw_icon(
                    cx.backend,
                    eye_icon,
                    eye_glyph.origin,
                    trailing_size,
                    eye_color,
                    trailing_stroke,
                );
            }
            if show_lock {
                draw_icon(
                    cx.backend,
                    lock_icon,
                    lock_glyph.origin,
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
                        origin: Point2D::new(
                            rect.origin.x + self.metrics.row_pad_x,
                            drop.indicator_y - 1.0,
                        ),
                        size: Point2D::new(rect.size.x - self.metrics.row_pad_x * 2.0, 2.0),
                    };
                    cx.backend.fill_rect(indicator_rect, self.theme.primary);
                }
                DropPosition::Into => {
                    // 1.5 px outline rect spanning the full row.
                    let outline = Rect {
                        origin: Point2D::new(rect.origin.x + 6.0, drop.indicator_y + 2.0),
                        size: Point2D::new(rect.size.x - 12.0, self.metrics.layer_row_height - 4.0),
                    };
                    cx.backend
                        .stroke_round_rect(outline, 6.0, self.theme.primary, 1.5);
                }
            }
        }

        cx.backend.restore();

        if let Some((ghost, cursor_y)) = &self.drag_ghost {
            paint_drag_ghost(cx, &self.theme, ghost, *cursor_y, rect, self.metrics);
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Tree);
        node.set_label("Layers");
        node
    }
}
