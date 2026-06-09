//! Step 4 widget glue — the only file in shell-web allowed to call
//! into `op_editor_ui::widgets`. All widget logic (state,
//! paint, layout, accesskit) lives in shell-core; this host is a
//! thin paint-loop adapter that takes a `&mut WebBackend` and
//! dispatches to the editor-UI composition.
//!
//! ### State model — `EditorState` is the source of truth
//!
//! Like the native host (`openpencil-shell-native/src/widget_host.rs`),
//! the web `WidgetHost` holds an `op_editor_core::EditorState` as its
//! single source of truth. shell-core's ~30 widgets read `EditorState`
//! directly; the canvas paint + the input hit-test read a derived
//! layout-resolved `LayoutScene`, rebuilt lazily by
//! `refresh_layout_scene()` whenever `editor_state_dirty` is set.
//! Every mutation routes through an `op-editor-core` mutator on
//! `editor_state` and flags the scene dirty.
//!
//! Layout (matches `apps/web/src/components/editor/editor-layout.tsx`):
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │ TopBar (full width × 48 px)                     │
//! ├──────────┬──────────────────────────────────────┤
//! │ Layer    │ Toolbar  Canvas (fills the middle)   │
//! │ Panel    │ ┌────┐                               │
//! │  (240)   │ │ ◯  │   AIChatPlaceholder           │
//! │          │ │ □  │   (floating bottom-center)    │
//! │          │ │ T  │                               │
//! │          │ │ #  │                  StatusBar    │
//! │          │ └────┘            (bottom-right pill)│
//! └──────────┴──────────────────────────────────────┘
//!                              ↑ RightPanel (only if selection)
//! ```
//!
//! Functions that pull in `op_editor_ui::widgets::*` MUST live
//! in this file (per spec §1.4). Phase B4 boundary check enforces.

use op_editor_ui::widgets::{
    LayoutCx, LocalePicker, Toolbar, TopBar, Widget, LOCALE_PICKER_WIDTH, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect, Theme};

mod agent_settings_mcp_server;
mod agent_settings_press;
#[cfg(test)]
mod agent_settings_press_tests;
mod ai_chat_geometry;
#[cfg(test)]
mod ai_chat_geometry_tests;
mod boolean_ops;
#[cfg(test)]
mod boolean_toolbar_tests;
mod chat_design_apply;
#[cfg(test)]
mod chat_design_apply_tests;
#[cfg(test)]
mod chat_design_hover_tests;
mod chat_model_picker_caret;
#[cfg(test)]
mod chat_model_picker_caret_tests;
mod keyboard;
mod paint;
mod press;
mod property_dispatch;
#[cfg(test)]
mod property_hover_tests;
mod scroll;
mod settings_caret;
#[cfg(test)]
mod settings_caret_tests;
#[cfg(test)]
mod theme_tests;

pub(in crate::widget_host) const TOOLBAR_INSET_X: f32 = 12.0;
pub(in crate::widget_host) const TOOLBAR_INSET_Y: f32 = 12.0;
pub(in crate::widget_host) const STATUS_INSET: f32 = 16.0;
const AICHAT_INSET_BOTTOM: f32 = 12.0;
const AICHAT_INSET_LEFT: f32 = 12.0;

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

pub struct WidgetHost {
    /// **The host's single source of truth.** All input handlers
    /// mutate this; paint + the input hit-test read the derived
    /// `layout_scene` rebuilt from it (see `refresh_layout_scene`).
    pub(in crate::widget_host) editor_state: op_editor_core::EditorState,
    /// Derived paint-only `LayoutScene` of `editor_state` — the
    /// layout-resolved render tree the `CanvasViewport` paints AND
    /// the host's canvas hit-test queries. Rebuilt lazily by
    /// `refresh_layout_scene()` whenever `editor_state_dirty` is set.
    pub(in crate::widget_host) layout_scene: op_editor_ui::layout_scene::LayoutScene,
    /// Set whenever `editor_state` is mutated. Drives the lazy rebuild
    /// of `layout_scene` — `refresh_layout_scene()` rebuilds + clears
    /// the flag, so a sequence of mutations re-derives once.
    pub(in crate::widget_host) editor_state_dirty: bool,
    pub(in crate::widget_host) theme: Theme,
    drag: Option<DragState>,
    chat_drag: Option<ChatDragState>,
    /// Active image-fill adjustment slider drag in the floating
    /// property popover.
    image_adjustment_drag: Option<op_editor_core::ImageAdjustmentField>,
    /// Active generated-code preview text selection drag.
    code_selection_drag: Option<CodeSelectionDragState>,
    /// Active marquee rect-select drag. Mirrors the native host —
    /// drag a rect on empty canvas with the Select tool, every
    /// intersecting top-level node joins (or extends) the
    /// selection on release.
    pub(in crate::widget_host) marquee_drag: Option<MarqueeDragState>,
    /// Active LayerPanel drag-to-reorder gesture. Mirrors the
    /// native host — press a layer row, drag past threshold, drop
    /// before/after the hovered row to reparent.
    pub(in crate::widget_host) layer_drag: Option<LayerDragState>,
    /// Counter for minting fresh `NodeId`s when the user duplicates
    /// a node. Bumped past the highest sample id so new + sample
    /// nodes never collide on the same key. Matches the native
    /// host's allocator.
    pub(in crate::widget_host) next_node_id: u64,
    /// Whether the shift key is currently held. The DOM listener
    /// updates this from every keyboard / mouse event so apply_press
    /// can branch on shift+click for multi-select. Matches the
    /// native host's `shift_held` flag.
    pub(in crate::widget_host) shift_held: bool,
    /// Host clock in ms — set by `lib.rs` on each event from
    /// `performance.now()`. Used for double-click detection.
    pub(in crate::widget_host) now_ms: u64,
    /// Most recent viewport size seen via `apply_press` etc. — cached
    /// so `apply_cursor_move(x, y)` can rebuild the canvas region
    /// when its signature can't carry viewport dims (mirrors native).
    pub(in crate::widget_host) last_viewport_w: f32,
    pub(in crate::widget_host) last_viewport_h: f32,
}

impl WidgetHost {
    pub fn set_now_ms(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    /// Borrow the canonical-model editor state — the host's single source of
    /// truth. Mirrors the native host's accessor; used by the web codegen
    /// session (`codegen_web`) to read the selection + codegen state.
    #[cfg(feature = "codegen")]
    pub fn editor_state(&self) -> &op_editor_core::EditorState {
        &self.editor_state
    }

    /// Mutable borrow of the canonical-model editor state. Callers that mutate
    /// through this MUST call [`mark_editor_state_dirty`] afterwards, else the
    /// paint snapshot goes stale. Used by the web codegen pump to stream
    /// progress into `editor_state.codegen`.
    ///
    /// [`mark_editor_state_dirty`]: WidgetHost::mark_editor_state_dirty
    #[cfg(feature = "codegen")]
    pub fn editor_state_mut(&mut self) -> &mut op_editor_core::EditorState {
        &mut self.editor_state
    }

    /// Public dirty-flag — mirrors the native host. Web codegen mutates
    /// `editor_state` through `editor_state_mut()` and calls this so the next
    /// paint re-derives the layout scene.
    #[cfg(feature = "codegen")]
    pub fn mark_editor_state_dirty(&mut self) {
        self.editor_state_dirty = true;
    }
}

#[derive(Debug, Clone, Copy)]
struct DragState {
    last_x: f32,
    last_y: f32,
}

#[derive(Debug, Clone, Copy)]
struct ChatDragState {
    grab_dx: f32,
    grab_dy: f32,
    pos_x: f32,
    pos_y: f32,
}

#[derive(Debug, Clone, Copy)]
struct CodeSelectionDragState {
    anchor: usize,
}

/// Active marquee rect-select state, mirroring the native host.
/// Endpoints are SCREEN coords so paint can draw without re-
/// deriving the canvas→screen transform; release converts to doc
/// space once to ask the document which nodes overlap.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct MarqueeDragState {
    pub(in crate::widget_host) start_screen_x: f32,
    pub(in crate::widget_host) start_screen_y: f32,
    pub(in crate::widget_host) current_screen_x: f32,
    pub(in crate::widget_host) current_screen_y: f32,
    /// Whether shift was held at press time. Drives whether
    /// release REPLACES the selection or adds intersecting nodes
    /// to the existing set (ADD-only — never removes).
    pub(in crate::widget_host) additive: bool,
}

/// Active LayerPanel drag-to-reorder gesture. Mirrors the native
/// host's `LayerDragState` — `source` is a shell-core `NodeId`
/// (the LayerPanel hit-test mints it) and is translated to an
/// op-editor-core id only at the commit site.
#[derive(Debug, Clone)]
pub(in crate::widget_host) struct LayerDragState {
    pub(in crate::widget_host) source: op_editor_core::NodeId,
    pub(in crate::widget_host) start_y: f32,
    pub(in crate::widget_host) current_x: f32,
    pub(in crate::widget_host) current_y: f32,
    pub(in crate::widget_host) active: bool,
}

impl WidgetHost {
    pub fn new() -> Self {
        // A fresh launch opens with a single empty starter Frame —
        // see `EditorState::starter`.
        let editor_state = op_editor_core::EditorState::starter();
        // Seed the render scene once up front; subsequent frames
        // re-derive only when `editor_state_dirty` is set.
        let layout_scene = op_pen_loader::editor_state_to_layout_scene(&editor_state);
        Self {
            editor_state,
            layout_scene,
            editor_state_dirty: false,
            theme: Theme::dark(),
            drag: None,
            chat_drag: None,
            image_adjustment_drag: None,
            code_selection_drag: None,
            marquee_drag: None,
            layer_drag: None,
            next_node_id: 100,
            shift_held: false,
            now_ms: 0,
            last_viewport_w: 0.0,
            last_viewport_h: 0.0,
        }
    }

    /// Forward the latest shift-key state from the DOM listener
    /// so apply_press can branch on shift+click. Web reads
    /// `MouseEvent.shiftKey` / `KeyboardEvent.shiftKey` per event
    /// and calls this just before dispatch.
    pub fn set_modifier_shift(&mut self, held: bool) {
        self.shift_held = held;
    }

    /// Refresh host-level theme tokens from the canonical editor UI
    /// state. Most widgets derive their theme directly, but a few
    /// paint-layer affordances still read this host cache.
    pub(in crate::widget_host) fn sync_theme_from_editor(&mut self) {
        self.theme =
            op_editor_ui::widgets::editor_state_ext::theme_for(&self.editor_state.editor_ui);
    }

    /// Rebuild the layout-resolved `LayoutScene` from `editor_state`
    /// if `editor_state_dirty` is set; clear the flag. Cheap no-op
    /// when the scene is already current. The input hit-test + the
    /// paint pass both call this before reading `layout_scene`.
    pub(in crate::widget_host) fn refresh_layout_scene(&mut self) {
        if self.editor_state_dirty {
            self.layout_scene = op_pen_loader::editor_state_to_layout_scene(&self.editor_state);
            self.editor_state_dirty = false;
        }
    }

    /// Mark `editor_state` as mutated so the next `refresh_layout_scene()`
    /// re-derives the render scene. Call after any direct mutation of
    /// `self.editor_state`.
    pub(in crate::widget_host) fn mark_dirty(&mut self) {
        self.editor_state_dirty = true;
    }

    pub(in crate::widget_host) fn canvas_region(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (f32, f32, f32, f32) {
        let canvas_left = if self.editor_state.editor_ui.sidebar_open {
            self.editor_state.editor_ui.layer_panel_width
        } else {
            0.0
        };
        let has_property = self.editor_state.property_panel_visible();
        let canvas_right = if has_property {
            viewport_w - self.editor_state.editor_ui.property_panel_width
        } else {
            viewport_w
        };
        let canvas_w = (canvas_right - canvas_left).max(0.0);
        let canvas_h = (viewport_h - TOP_BAR_HEIGHT).max(0.0);
        (canvas_left, TOP_BAR_HEIGHT, canvas_w, canvas_h)
    }

    fn over_canvas(&self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        x >= cx0 && x <= cx0 + cw && y >= cy0 && y <= cy0 + ch
    }

    pub(in crate::widget_host) fn code_text_offset_at_screen(
        &self,
        x: f32,
        y: f32,
    ) -> Option<usize> {
        if !self.editor_state.property_panel_visible()
            || !matches!(
                self.editor_state.editor_ui.property_tab,
                op_editor_core::PropertyTab::Code
            )
        {
            return None;
        }
        let pw = self.editor_state.editor_ui.property_panel_width;
        let panel_x = self.last_viewport_w - pw;
        if x < panel_x || x > self.last_viewport_w {
            return None;
        }
        let panel_rect = Rect {
            origin: Point2D::new(panel_x, TOP_BAR_HEIGHT),
            size: Point2D::new(pw, (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0)),
        };
        op_editor_ui::widgets::property_panel_code::code_text_offset_at(
            panel_rect,
            &self.editor_state.codegen,
            Point2D::new(x, y),
        )
    }

    fn apply_code_selection_drag_cursor_move(&mut self, x: f32, y: f32) -> bool {
        let Some(anchor) = self.code_selection_drag.map(|drag| drag.anchor) else {
            return false;
        };
        if let Some(focus) = self.code_text_offset_at_screen(x, y) {
            let next = Some(op_editor_core::codegen::CodeSelection { anchor, focus });
            if self.editor_state.codegen.code_selection != next {
                self.editor_state.codegen.code_selection = next;
                self.mark_dirty();
            }
        }
        true
    }

    /// Wheel zoom centered on the cursor when over the canvas.
    pub fn apply_wheel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        if self.editor_state.editor_ui.agent_settings_open {
            use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
            let panel_rect = AgentSettingsPanel::for_editor(&self.editor_state)
                .rect(viewport_width, viewport_height);
            let point = Point2D::new(x, y);
            if rect_contains(panel_rect, point) {
                if let Some(max) = AgentSettingsPanel::for_editor(&self.editor_state)
                    .builtin_preset_scroll_max_at(panel_rect, point)
                {
                    let settings = &mut self.editor_state.editor_ui.agent_settings;
                    settings.builtin_preset_menu_scroll =
                        (settings.builtin_preset_menu_scroll - delta_y).clamp(0.0, max);
                    self.mark_dirty();
                    return true;
                }
                let panel = AgentSettingsPanel::for_editor(&self.editor_state);
                let total = panel.content_total_height();
                let viewport_h_inner = panel_rect.size.y - 48.0;
                let max_scroll = (total - viewport_h_inner).max(0.0);
                self.editor_state.editor_ui.agent_settings.scroll_y =
                    (self.editor_state.editor_ui.agent_settings.scroll_y - delta_y)
                        .clamp(0.0, max_scroll);
                self.mark_dirty();
                return true;
            }
        }
        // Side rails scroll their panels instead of zooming the
        // canvas (`widget_host/scroll.rs`).
        if self.try_scroll_property_panel(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_layer_panel(x, y, delta_y, viewport_height) {
            return true;
        }
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        // Cursor is in canvas-local coords — subtract the live
        // canvas-region offset (sidebar collapse aware).
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let cursor = Point2D::new(x - cx0, y - cy0);
        self.editor_state.viewport.zoom_at(cursor, delta_y);
        self.mark_dirty();
        true
    }

    /// 2-finger trackpad pan — translate viewport by `(dx, dy)`.
    /// Same Figma-style separation as the native host: trackpad
    /// swipe pans, pinch / Cmd+wheel / mouse-wheel zooms. Public
    /// host API — a future trackpad-gesture runner wires this.
    #[allow(dead_code)]
    pub fn apply_pan_gesture(
        &mut self,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        self.editor_state.viewport.pan(dx, dy);
        self.mark_dirty();
        true
    }

    /// Update `editor_ui.hovered_layer_id` from the cursor.
    /// Returns true if hover state changed (caller should
    /// repaint). Mirrors the native host.
    pub fn update_layer_hover(&mut self, x: f32, y: f32, viewport_h: f32) -> bool {
        use op_editor_ui::widgets::{LayerPanel, LayerPanelHit, TOP_BAR_HEIGHT};
        let sidebar_open = self.editor_state.editor_ui.sidebar_open;
        let panel_w = self.editor_state.editor_ui.layer_panel_width;
        let (new_layer, new_page) =
            if sidebar_open && y >= TOP_BAR_HEIGHT && x >= 0.0 && x <= panel_w {
                self.refresh_layout_scene();
                let layer_rect = Rect {
                    origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                    size: Point2D::new(panel_w, (viewport_h - TOP_BAR_HEIGHT).max(0.0)),
                };
                let panel = LayerPanel::from_editor(&self.editor_state);
                match panel.hit_test(layer_rect, Point2D::new(x, y)) {
                    Some(LayerPanelHit::Layer(id))
                    | Some(LayerPanelHit::ToggleHidden(id))
                    | Some(LayerPanelHit::ToggleLocked(id))
                    | Some(LayerPanelHit::ToggleCollapsed(id)) => (Some(id), None),
                    Some(LayerPanelHit::Page(idx)) | Some(LayerPanelHit::DeletePage(idx)) => {
                        (None, Some(idx))
                    }
                    _ => (None, None),
                }
            } else {
                (None, None)
            };
        // shell-core hit-test returns shell-core `NodeId`s; translate
        // to op-editor-core ids for storage on `editor_ui`.
        let new_layer_ec = new_layer.clone();
        let changed = new_layer_ec != self.editor_state.editor_ui.hovered_layer_id
            || new_page != self.editor_state.editor_ui.hovered_page_index;
        if changed {
            self.editor_state.editor_ui.hovered_layer_id = new_layer_ec;
            self.editor_state.editor_ui.hovered_page_index = new_page;
            self.mark_dirty();
        }
        changed
    }

    /// Cursor-move handler — drives canvas pan-drag, marquee
    /// drag, chat drag, or no-op.
    pub(in crate::widget_host) fn update_agent_settings_hover(&mut self, x: f32, y: f32) -> bool {
        use op_editor_core::AgentSettingsTab;
        use op_editor_ui::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
        let point = Point2D::new(x, y);
        let (
            server_hover,
            copy_hover,
            add_provider_hover,
            add_acp_hover,
            image_search_test_hover,
            image_add_hover,
            new_hover,
        ) = {
            let panel = AgentSettingsPanel::for_editor(&self.editor_state);
            let panel_rect = panel.rect(self.last_viewport_w, self.last_viewport_h);
            let hit = panel.hit_test(panel_rect, point);
            let is_agents = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Agents
            );
            let is_images = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Images
            );
            let copy_hover = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Mcp
            ) && matches!(hit, AgentSettingsHit::CopyMcpClientConfig);
            let server_hover = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Mcp
            ) && matches!(hit, AgentSettingsHit::ToggleMcpServer);
            let add_provider_hover = is_agents && matches!(hit, AgentSettingsHit::AddProvider);
            let add_acp_hover = is_agents && matches!(hit, AgentSettingsHit::AddAcpAgent);
            let image_search_test_hover =
                is_images && panel.image_search_test_button_hover_at(panel_rect, point);
            let image_add_hover =
                is_images && panel.image_gen_add_button_hover_at(panel_rect, point);
            let new_hover = panel.builtin_preset_hover_at(panel_rect, point);
            (
                server_hover,
                copy_hover,
                add_provider_hover,
                add_acp_hover,
                image_search_test_hover,
                image_add_hover,
                new_hover,
            )
        };
        let mut changed = false;
        if server_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_server_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_server_button = server_hover;
            changed = true;
        }
        if copy_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_client_config_copy
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_client_config_copy = copy_hover;
            changed = true;
        }
        if new_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .builtin_preset_menu_hover
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .builtin_preset_menu_hover = new_hover;
            changed = true;
        }
        if add_provider_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_add_provider
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_add_provider = add_provider_hover;
            changed = true;
        }
        if add_acp_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_add_acp_agent
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_add_acp_agent = add_acp_hover;
            changed = true;
        }
        if image_search_test_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_test_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_test_button = image_search_test_hover;
            changed = true;
        }
        if image_add_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_add_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_add_button = image_add_hover;
            changed = true;
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }

    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        // Refresh the derived paint doc once up front so every hit-test
        // below (layer context menu, layer drag, align toolbar) reads
        // current geometry, never a stale snapshot.
        self.refresh_layout_scene();
        if self.editor_state.editor_ui.agent_settings_open && self.update_agent_settings_hover(x, y)
        {
            return true;
        }
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            let new_hover = menu.hovered_row_at(Point2D::new(x, y)).map(|i| i as u8);
            if new_hover != state.hovered_row {
                self.editor_state.editor_ui.layer_context_menu =
                    Some(op_editor_core::editor_ui_state::LayerContextMenuState {
                        hovered_row: new_hover,
                        ..state
                    });
                self.mark_dirty();
                return true;
            }
        }
        // Export-section select-popup row hover highlight.
        if self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
        {
            if let Some(panel) =
                op_editor_ui::widgets::PropertyPanel::for_selection(&self.editor_state)
            {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        op_editor_ui::widgets::TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                let new_hover = panel.export_picker_row_at(property_rect, Point2D::new(x, y));
                if new_hover != self.editor_state.editor_ui.export_picker_hover {
                    self.editor_state.editor_ui.export_picker_hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if self.apply_code_selection_drag_cursor_move(x, y) {
            return true;
        }
        if let Some(m) = self.marquee_drag.as_mut() {
            m.current_screen_x = x;
            m.current_screen_y = y;
            return true;
        }
        if self.layer_drag.is_some() {
            // Drop the gesture if the source disappeared mid-drag —
            // see the native host for the rationale.
            let source_id = self.layer_drag.as_ref().unwrap().source.clone();
            let still_present = self
                .layout_scene
                .active_page()
                .map(|p| p.find(source_id.as_str()).is_some())
                .unwrap_or(false);
            if !still_present {
                self.layer_drag = None;
                return true;
            }
            let d = self.layer_drag.as_mut().unwrap();
            d.current_x = x;
            d.current_y = y;
            // VERTICAL-ONLY activation (4 px). See the native host
            // for the rationale: pure horizontal wiggle must not
            // steal click-feel from row-level gestures.
            if !d.active && (y - d.start_y).abs() > 4.0 {
                d.active = true;
            }
            return true;
        }
        if let Some(d) = self.chat_drag.as_mut() {
            d.pos_x = x - d.grab_dx;
            d.pos_y = y - d.grab_dy;
            return true;
        }
        if let Some(field) = self.image_adjustment_drag {
            if let Some(panel) =
                op_editor_ui::widgets::PropertyPanel::for_selection(&self.editor_state)
            {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        op_editor_ui::widgets::TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                if let Some(action) = panel.image_adjustment_drag_action(property_rect, field, x) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        if let Some(drag) = self.drag.as_mut() {
            let dx = x - drag.last_x;
            let dy = y - drag.last_y;
            drag.last_x = x;
            drag.last_y = y;
            self.editor_state.viewport.pan(dx, dy);
            self.mark_dirty();
            return true;
        }
        // Toolbar per-button hover wash — AFTER drag detection so a
        // path-anchor / node / pan drag whose cursor crosses the
        // toolbar isn't intercepted by the hover update (mirrors
        // native widget_host/input.rs ordering).
        if self.update_toolbar_hover(x, y) {
            return true;
        }
        // TopBar chrome-button hover wash (sidebar / file-menu / figma /
        // theme / locale / fullscreen / agent chip). The git button is
        // compiled out on wasm32; every other button lights up the same
        // as native. Reuses the click hit-test so paint can't drift.
        {
            use op_editor_ui::widgets::{TopBar, TOP_BAR_HEIGHT};
            let tb_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, TOP_BAR_HEIGHT),
            };
            let new_hover = TopBar::for_editor_ui(&self.editor_state.editor_ui)
                .hit_test(tb_rect, Point2D::new(x, y))
                .map(op_editor_ui::widgets::editor_state_ext::topbar_button_hover);
            if new_hover != self.editor_state.editor_ui.topbar_button_hover {
                self.editor_state.editor_ui.topbar_button_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        // StatusBar control hover wash (search / zoom-out / zoom-in).
        {
            let new_hover = self
                .status_bar_rect(self.last_viewport_w, self.last_viewport_h)
                .and_then(|r| {
                    op_editor_ui::widgets::StatusBar::for_editor(&self.editor_state)
                        .control_at(r, Point2D::new(x, y))
                });
            if new_hover != self.editor_state.editor_ui.statusbar_hover {
                self.editor_state.editor_ui.statusbar_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        // AI chat header buttons (chevron / maximize / new chat) hover.
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .hit_test(chat_rect, Point2D::new(x, y))
                .as_ref()
                .and_then(op_editor_ui::widgets::editor_state_ext::chat_header_hover);
            if new_hover != self.editor_state.editor_ui.chat_header_hover {
                self.editor_state.editor_ui.chat_header_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
            use op_editor_ui::widgets::AIChatPlaceholder;
            let new_hover = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .footer_hover_at(chat_rect, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.chat_footer_hover {
                self.editor_state.editor_ui.chat_footer_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        } else if self
            .editor_state
            .editor_ui
            .chat_footer_hover
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        // PropertyPanel tab/action hover wash. Shown with a selection.
        let mut property_hover_changed = false;
        if self.editor_state.property_panel_visible() {
            use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
            if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                let point = Point2D::new(x, y);
                let new_tab_hover = panel.tab_hover_at(property_rect, point);
                if new_tab_hover != self.editor_state.editor_ui.property_tab_hover {
                    self.editor_state.editor_ui.property_tab_hover = new_tab_hover;
                    property_hover_changed = true;
                }
                let new_action_hover = panel.action_hover_index(property_rect, point);
                if new_action_hover != self.editor_state.editor_ui.property_action_hover {
                    self.editor_state.editor_ui.property_action_hover = new_action_hover;
                    property_hover_changed = true;
                }
            }
        } else if self
            .editor_state
            .editor_ui
            .property_tab_hover
            .take()
            .is_some()
        {
            property_hover_changed = true;
        }
        // Code-panel hover wash. Reuses the panel's click geometry for
        // framework chips, scroll chevrons, and body actions.
        let (new_fw_hover, new_action_hover) = if self.editor_state.property_panel_visible()
            && matches!(
                self.editor_state.editor_ui.property_tab,
                op_editor_core::PropertyTab::Code
            ) {
            let pw = self.editor_state.editor_ui.property_panel_width;
            let panel_x = self.last_viewport_w - pw;
            let panel_rect = Rect {
                origin: Point2D::new(panel_x, TOP_BAR_HEIGHT),
                size: Point2D::new(pw, (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0)),
            };
            if x >= panel_x && x <= self.last_viewport_w {
                op_editor_ui::widgets::property_panel_code::code_hover_at(
                    panel_rect,
                    &self.editor_state.codegen,
                    Point2D::new(x, y),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        if new_fw_hover != self.editor_state.codegen.framework_hover
            || new_action_hover != self.editor_state.codegen.action_hover
        {
            self.editor_state.codegen.framework_hover = new_fw_hover;
            self.editor_state.codegen.action_hover = new_action_hover;
            self.mark_dirty();
            return true;
        }
        if property_hover_changed {
            self.mark_dirty();
            return true;
        }
        // No drag active — sync align toolbar hover. AFTER all drag
        // branches so an active drag isn't intercepted (codex CONCERN
        // — mirrors native widget_host/input.rs ordering).
        let new_hover = if self.editor_state.selection_count() >= 2 {
            use op_editor_ui::widgets::{AlignToolbar, TOP_BAR_HEIGHT};
            let (cx, _, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let canvas_region = op_editor_ui::Rect {
                origin: Point2D::new(cx, TOP_BAR_HEIGHT),
                size: Point2D::new(cw, ch),
            };
            AlignToolbar::for_canvas_region(canvas_region, &self.editor_state)
                .and_then(|tb| tb.hit_test(Point2D::new(x, y)))
        } else {
            None
        };
        let new_hover_ec = new_hover;
        if new_hover_ec != self.editor_state.editor_ui.align_toolbar_hover {
            self.editor_state.editor_ui.align_toolbar_hover = new_hover_ec;
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Convert a marquee drag (screen-space) into a doc-space
    /// rect, ask the document which top-level nodes overlap it,
    /// and either replace or extend the selection. Mirrors
    /// native `WidgetHostNative::commit_marquee_selection`.
    pub(in crate::widget_host) fn commit_marquee_selection(
        &mut self,
        m: MarqueeDragState,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        // Near-zero marquee = a click without drag. Threshold is
        // measured in SCREEN pixels (2 px) so it stays consistent
        // regardless of canvas zoom — matches native.
        let screen_dx = (m.current_screen_x - m.start_screen_x).abs();
        let screen_dy = (m.current_screen_y - m.start_screen_y).abs();
        if screen_dx < 2.0 && screen_dy < 2.0 {
            return;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let p0 = {
            let local = Point2D::new(m.start_screen_x - cx0, m.start_screen_y - cy0);
            self.editor_state.viewport.to_document(local)
        };
        let p1 = {
            let local = Point2D::new(m.current_screen_x - cx0, m.current_screen_y - cy0);
            self.editor_state.viewport.to_document(local)
        };
        let x = p0.x.min(p1.x);
        let y = p0.y.min(p1.y);
        let w = (p1.x - p0.x).abs();
        let h = (p1.y - p0.y).abs();
        let rect = Rect::xywh(x, y, w, h);
        // `nodes_intersecting_doc_rect` queries the `LayoutScene` —
        // it returns the resolved-scene node id strings.
        self.refresh_layout_scene();
        let ids = self.layout_scene.nodes_intersecting_doc_rect(rect);
        if m.additive {
            // ADD-only: every hit joins the set; already-selected
            // hits stay selected. Shift-marquee never removes.
            for id in ids {
                let ec_id = op_editor_core::NodeId::new(&id);
                if !self.editor_state.is_selected(&ec_id) {
                    self.editor_state.toggle_selection(ec_id);
                }
            }
            self.mark_dirty();
        } else if !ids.is_empty() {
            let ec_ids: Vec<op_editor_core::NodeId> =
                ids.iter().map(op_editor_core::NodeId::new).collect();
            let anchor = ec_ids.last().unwrap().clone();
            self.editor_state.selection.set = ec_ids;
            self.editor_state.selection.anchor = anchor;
            self.mark_dirty();
        }
    }

    /// Mouse-release handler — commits any marquee drag, snaps a
    /// chat-panel drag to the nearest corner, or ends the canvas
    /// pan-drag.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        self.last_viewport_w = viewport_w;
        self.last_viewport_h = viewport_h;
        if let Some(m) = self.marquee_drag.take() {
            self.commit_marquee_selection(m, viewport_w, viewport_h);
            return true;
        }
        if self.code_selection_drag.take().is_some() {
            return true;
        }
        if let Some(d) = self.layer_drag.take() {
            return self.commit_layer_drag(d, viewport_h);
        }
        if let Some(d) = self.chat_drag.take() {
            // Use the live panel size (expanded vs collapsed) so a
            // dragged collapsed pill snaps to the corner closest to
            // its actual center, matching native.
            let (panel_w, panel_h) = self.ai_chat_size();
            let center = Point2D::new(d.pos_x + panel_w / 2.0, d.pos_y + panel_h / 2.0);
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
            self.editor_state.chat.anchor =
                op_editor_core::ChatAnchor::nearest(center, cx0, cy0, cw, ch);
            self.mark_dirty();
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging
    }

    /// Mouse-release handler — viewport-less variant. Public host
    /// API parity with the native shell; the browser runner wires
    /// the viewport-aware `apply_release_with_viewport` instead.
    #[allow(dead_code)]
    pub fn apply_release(&mut self) -> bool {
        if self.marquee_drag.take().is_some() {
            // Can't compute the doc-space marquee rect without a
            // viewport; drop without committing. The viewport-
            // aware variant is the one runners should call.
            return true;
        }
        if self.code_selection_drag.take().is_some() {
            return true;
        }
        if self.layer_drag.take().is_some() {
            // Same — drop_target_at needs the panel rect (and thus
            // viewport_h). Drop the candidate without committing.
            return true;
        }
        if self.chat_drag.take().is_some() {
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging
    }

    /// Resolve a layer drag-to-reorder gesture on release. Mirrors
    /// native `WidgetHostNative::commit_layer_drag`.
    pub(in crate::widget_host) fn commit_layer_drag(
        &mut self,
        d: LayerDragState,
        viewport_h: f32,
    ) -> bool {
        if !d.active {
            return false;
        }
        self.refresh_layout_scene();
        // Defensive source-validity check (mirrors native) — bail
        // if the dragged node disappeared between move and release.
        if self
            .layout_scene
            .active_page()
            .map(|p| p.find(d.source.as_str()).is_none())
            .unwrap_or(true)
        {
            return false;
        }
        use op_editor_ui::widgets::{DropPosition, LayerPanel};
        let layer_rect = self.layer_panel_rect(viewport_h);
        // Source-excluded panel so the indicator y the user saw and
        // the row landed on match the post-commit layout — see the
        // native `commit_layer_drag` for the rationale.
        let panel = LayerPanel::from_editor_with_drag_source(&self.editor_state, &d.source);
        let cursor = Point2D::new(d.current_x, d.current_y);
        let Some(drop) = panel.drop_target_at(layer_rect, cursor) else {
            return true;
        };
        if drop.anchor == d.source {
            return true;
        }
        let source = d.source.clone();
        let anchor = drop.anchor.clone();
        match drop.position {
            DropPosition::Before => {
                self.editor_state.reorder_before(source, anchor);
            }
            DropPosition::After => {
                self.editor_state.reorder_after(source, anchor);
            }
            DropPosition::Into => {
                self.editor_state.reorder_into(source, anchor);
            }
        }
        self.mark_dirty();
        true
    }

    // Keyboard / clipboard handlers (`apply_text` / `apply_backspace`
    // / `apply_send` / `apply_delete` / `apply_duplicate` /
    // `apply_nudge` / `apply_select_all` / `apply_copy` /
    // `apply_cut` / `apply_paste` / `apply_reorder` /
    // `apply_escape` / `apply_ime` / `apply_key`) live in
    // `widget_host/keyboard.rs` — split out to keep this spine
    // file under the 800-line ceiling.

    pub(in crate::widget_host) fn locale_picker_rect(&self, viewport_w: f32) -> Rect {
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        };
        let globe = TopBar::globe_rect(top_bar_rect);
        let panel_h = LocalePicker::panel_height();
        let x = (globe.origin.x + globe.size.x / 2.0 - LOCALE_PICKER_WIDTH / 2.0)
            .max(8.0)
            .min(viewport_w - LOCALE_PICKER_WIDTH - 8.0);
        let y = globe.origin.y + globe.size.y + 6.0;
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(LOCALE_PICKER_WIDTH, panel_h),
        }
    }

    pub(in crate::widget_host) fn layer_panel_rect(&self, viewport_h: f32) -> Rect {
        Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.editor_state.editor_ui.layer_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        }
    }

    fn toolbar_rect(&mut self, viewport_w: f32) -> Rect {
        // Anchor follows canvas_region (sidebar-collapse aware) so
        // hit-test matches paint regardless of sidebar state.
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_w, f32::INFINITY);
        self.refresh_layout_scene();
        let toolbar = Toolbar::for_editor(&self.editor_state);
        let h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, h),
        }
    }

    /// Per-button hover wash on the floating toolbar. Mirrors
    /// `op_host_native::widget_host::geometry::update_toolbar_hover`.
    /// Returns `true` if the hover state changed.
    fn update_toolbar_hover(&mut self, x: f32, y: f32) -> bool {
        let rect = self.toolbar_rect(self.last_viewport_w);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        let new_hover = toolbar
            .hit_test(rect, Point2D::new(x, y))
            .map(op_editor_ui::widgets::editor_state_ext::toolbar_hover);
        if new_hover != self.editor_state.editor_ui.toolbar_hover {
            self.editor_state.editor_ui.toolbar_hover = new_hover;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    // `paint` lives in `widget_host/paint.rs` — split out to keep
    // this file under the 800-line ceiling.
}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}
