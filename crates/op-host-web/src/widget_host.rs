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

use op_editor_ui::widgets::TOP_BAR_HEIGHT;
use op_editor_ui::{Point2D, Rect, Theme};

mod a11y_bridge;
#[cfg(test)]
mod agent_settings_acp_press_tests;
#[cfg(test)]
mod agent_settings_compact_press_tests;
#[cfg(test)]
mod agent_settings_form_press_tests;
mod agent_settings_mcp_server;
mod agent_settings_press;
#[cfg(test)]
mod agent_settings_press_tests;
mod ai_chat_geometry;
#[cfg(test)]
mod ai_chat_geometry_tests;
mod blur_inputs;
#[cfg(test)]
mod blur_inputs_tests;
#[cfg(test)]
mod boolean_toolbar_tests;
mod chat_design_apply;
#[cfg(test)]
mod chat_design_apply_tests;
mod chat_design_hover;
#[cfg(test)]
mod chat_design_hover_tests;
mod chat_model_picker_caret;
#[cfg(test)]
mod chat_model_picker_caret_tests;
#[cfg(test)]
mod chat_send_tests;
mod chrome_menu_press;
mod click;
mod color_picker_press;
mod component_browser_press;
mod cursor_input;
#[cfg(test)]
mod deferred_press_tests;
mod design_md_press;
#[cfg(test)]
mod design_md_press_tests;
pub(crate) mod icon_ingest;
// Browser file-IO ingestion (Open / Figma import / clipboard paste)
// — needs the codegen-gated document-pipeline deps (jian-ops-schema).
#[cfg(feature = "canvaskit")]
mod file_ingest;
#[cfg(test)]
mod file_menu_paint_tests;
mod geometry;
mod group_ops;
mod history_guard;
mod icon_picker_press;
mod image_panel_dispatch;
#[cfg(all(test, feature = "canvaskit"))]
mod io_tests;
mod keyboard;
mod keyboard_edit_ops;
mod keyboard_git;
mod keyboard_ime;
mod keyboard_settings_commit;
#[cfg(test)]
mod keyboard_tests;
mod keyboard_text_inputs;
#[cfg(test)]
mod layer_context_history_tests;
#[cfg(test)]
mod layer_panel_rename_tests;
#[cfg(test)]
mod locale_picker_scroll_tests;
mod node_drag;
#[cfg(test)]
mod node_drag_tests;
mod overlay_cursor;
mod overlay_keys;
#[cfg(test)]
mod overlay_press_tests;
mod overlay_rects;
mod paint;
#[cfg(test)]
mod paint_caret_tests;
#[cfg(test)]
mod pan_tests;
mod pen_press;
#[cfg(test)]
mod pen_press_tests;
mod press;
mod property_dispatch;
mod property_focus_press;
#[cfg(test)]
mod property_hover_tests;
#[cfg(test)]
mod property_input_tests;
mod property_layout_dispatch;
#[cfg(test)]
mod property_panel_press_tests;
mod release_input;
mod resize_drag;
#[cfg(test)]
mod resize_drag_tests;
mod scroll;
mod settings_caret;
#[cfg(test)]
mod settings_caret_tests;
mod shape_create;
#[cfg(test)]
mod shape_create_tests;
mod shape_picker_press;
mod text_edit_caret;
#[cfg(test)]
mod theme_tests;
mod toolbar_actions;
#[cfg(test)]
mod topbar_hover_tests;
mod variables_panel_commit;
mod variables_panel_geometry;
mod variables_panel_press;
mod variables_panel_rows;
#[cfg(test)]
mod variables_panel_tests;
mod web_fonts;

pub(in crate::widget_host) const TOOLBAR_INSET_X: f32 = 12.0;
pub(in crate::widget_host) const TOOLBAR_INSET_Y: f32 = 12.0;
pub(in crate::widget_host) const STATUS_INSET: f32 = 16.0;
const AICHAT_INSET_BOTTOM: f32 = 12.0;
const AICHAT_INSET_LEFT: f32 = 12.0;

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
    /// Live-sync push gate: raised alongside `editor_state_dirty` but
    /// consumed by the 2 s document-push tick instead of the paint pass
    /// (a conservative superset of document edits — the push path's
    /// content-hash check absorbs UI-only false positives).
    #[cfg(feature = "canvaskit")]
    doc_sync_dirty: bool,
    pub(in crate::widget_host) theme: Theme,
    drag: Option<DragState>,
    /// True while Space is held and no text input owns the keyboard.
    /// Mirrors native/TS transient pan mode: canvas presses pan even
    /// when the active tool is Select or a creation tool.
    space_pan: bool,
    chat_drag: Option<ChatDragState>,
    /// Active image-fill adjustment slider drag in the floating
    /// property popover.
    image_adjustment_drag: Option<op_editor_core::ImageAdjustmentField>,
    /// Active generated-code preview text selection drag.
    code_selection_drag: Option<CodeSelectionDragState>,
    /// Active chat input text selection drag.
    chat_input_selection_drag: Option<ChatInputSelectionDragState>,
    /// Active chat transcript text selection drag.
    chat_text_selection_drag: Option<ChatTextSelectionDragState>,
    /// Active shape-create drag — set when pressing empty canvas
    /// with a shape / frame / text tool selected.
    pub(in crate::widget_host) create_drag: Option<shape_create::CreateDragState>,
    /// Active path-anchor / bezier-handle drag. Mirrors the native
    /// host so Select-tool path editing keeps the same no-snap and
    /// history-on-release behavior.
    pub(in crate::widget_host) path_anchor_drag: Option<PathAnchorDragState>,
    /// Active marquee rect-select drag. Mirrors the native host —
    /// drag a rect on empty canvas with the Select tool, every
    /// intersecting top-level node joins (or extends) the
    /// selection on release.
    pub(in crate::widget_host) marquee_drag: Option<MarqueeDragState>,
    /// Active LayerPanel drag-to-reorder gesture. Mirrors the
    /// native host — press a layer row, drag past threshold, drop
    /// before/after the hovered row to reparent.
    pub(in crate::widget_host) layer_drag: Option<LayerDragState>,
    /// Active floating-panel header drags (Design-MD / Component-
    /// Browser / Icon-picker). Mirror the native host's per-panel
    /// drag states; one shared shape since all three carry only the
    /// grab offset.
    pub(in crate::widget_host) design_md_drag: Option<PanelDragState>,
    pub(in crate::widget_host) component_browser_drag: Option<PanelDragState>,
    pub(in crate::widget_host) icon_picker_drag: Option<PanelDragState>,
    /// Active floating-VariablesPanel resize drag (right / bottom /
    /// corner edge). Mirrors the native host; the live size is
    /// written into `editor_ui.variables_panel_size`.
    pub(in crate::widget_host) variables_resize:
        Option<op_editor_ui::widgets::variables_panel::VariablesResizeEdge>,
    /// Active selection-handle resize drag. Mirrors native
    /// `handle_drag`: pressing one of the 8 selection grips captures
    /// the starting bounds and move writes `set_selected_bounds`.
    pub(in crate::widget_host) handle_drag: Option<resize_drag::HandleDragState>,
    /// Active Select-tool node move drag. Mirrors native
    /// `node_drag`: press a selected canvas node, translate the
    /// selection live on cursor move, then end the transient state on
    /// release.
    pub(in crate::widget_host) node_drag: Option<node_drag::NodeDragState>,
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
    /// Unix wall-clock seconds. Kept separate from `now_ms` because
    /// browser `performance.now()` is navigation-relative, while
    /// recent-file timestamps are Unix seconds.
    pub(in crate::widget_host) wall_now_secs: u64,
    /// Most recent viewport size seen via `apply_press` etc. — cached
    /// so `apply_cursor_move(x, y)` can rebuild the canvas region
    /// when its signature can't carry viewport dims (mirrors native).
    pub(in crate::widget_host) last_viewport_w: f32,
    pub(in crate::widget_host) last_viewport_h: f32,
}

impl WidgetHost {
    #[allow(dead_code)]
    pub fn set_now_ms(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    #[allow(dead_code)]
    pub fn set_wall_now_secs(&mut self, secs: u64) {
        self.wall_now_secs = secs;
    }

    pub fn set_clocks(&mut self, now_ms: u64, wall_now_secs: u64) {
        self.now_ms = now_ms;
        self.wall_now_secs = wall_now_secs;
    }

    // Caret-blink / animation scheduling — tested + ready to wire, but the
    // CanvasKit mount repaints on events rather than a blink-deadline pump.
    #[allow(dead_code)]
    pub fn caret_animation_active(&self) -> bool {
        self.editor_state.active_text_input().is_some()
    }

    #[allow(dead_code)]
    pub fn next_animation_deadline_ms(&self) -> Option<u64> {
        let mut next = op_editor_core::agent_indicators::next_reveal_deadline_ms(self.now_ms);
        if let Some(input) = self.editor_state.active_text_input() {
            let deadline = input.next_blink_flip_ms(self.now_ms);
            next = Some(next.map_or(deadline, |current| current.min(deadline)));
        }
        next
    }

    /// Borrow the canonical-model editor state — the host's single source of
    /// truth. Mirrors the native host's accessor; used by the web codegen
    /// session (`codegen_web`) to read the selection + codegen state, and by
    /// the live-sync glue to serialize the document + selection for pushes.
    #[cfg(feature = "canvaskit")]
    pub fn editor_state(&self) -> &op_editor_core::EditorState {
        &self.editor_state
    }

    /// Take the live-sync push gate (see the field docs) — `true` when any
    /// mutation may have touched the document since the last take.
    #[cfg(feature = "canvaskit")]
    pub fn take_doc_sync_dirty(&mut self) -> bool {
        std::mem::take(&mut self.doc_sync_dirty)
    }

    /// Mutable borrow of the canonical-model editor state. Callers that mutate
    /// through this MUST call [`mark_editor_state_dirty`] afterwards, else the
    /// paint snapshot goes stale. Used by the web codegen pump to stream
    /// progress into `editor_state.codegen`.
    ///
    /// [`mark_editor_state_dirty`]: WidgetHost::mark_editor_state_dirty
    #[cfg(feature = "canvaskit")]
    pub fn editor_state_mut(&mut self) -> &mut op_editor_core::EditorState {
        &mut self.editor_state
    }

    /// Public dirty-flag — mirrors the native host. Web codegen mutates
    /// `editor_state` through `editor_state_mut()` and calls this so the next
    /// paint re-derives the layout scene.
    #[cfg(feature = "canvaskit")]
    pub fn mark_editor_state_dirty(&mut self) {
        self.editor_state_dirty = true;
        #[cfg(feature = "canvaskit")]
        {
            self.doc_sync_dirty = true;
        }
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

/// Header drag on a floating panel (Design-MD / Component-Browser /
/// Icon-picker) — the panel's top-left follows the cursor minus the
/// grab offset. Mirrors the native host's `*DragState` trio.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct PanelDragState {
    pub(in crate::widget_host) grab_dx: f32,
    pub(in crate::widget_host) grab_dy: f32,
}

#[derive(Debug, Clone, Copy)]
struct CodeSelectionDragState {
    anchor: usize,
}

#[derive(Debug, Clone, Copy)]
struct ChatInputSelectionDragState {
    anchor: usize,
}

#[derive(Debug, Clone, Copy)]
struct ChatTextSelectionDragState {
    message_index: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::widget_host) enum AnchorDragTarget {
    Anchor,
    Handle(op_editor_core::pen::PathHandleSide),
}

#[derive(Debug, Clone)]
pub(in crate::widget_host) struct PathAnchorDragState {
    pub(in crate::widget_host) node_id: op_editor_core::NodeId,
    pub(in crate::widget_host) anchor_index: usize,
    pub(in crate::widget_host) target: AnchorDragTarget,
    pub(in crate::widget_host) anchor_doc: op_editor_ui::Point2D,
    pub(in crate::widget_host) start_doc: op_editor_ui::Point2D,
    pub(in crate::widget_host) grab_offset: Option<op_editor_ui::Point2D>,
    pub(in crate::widget_host) shift: bool,
    pub(in crate::widget_host) moved: bool,
    pub(in crate::widget_host) pre_drag_snapshot: op_editor_core::EditorSnapshot,
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
            #[cfg(feature = "canvaskit")]
            doc_sync_dirty: false,
            theme: Theme::dark(),
            drag: None,
            space_pan: false,
            chat_drag: None,
            image_adjustment_drag: None,
            code_selection_drag: None,
            chat_input_selection_drag: None,
            chat_text_selection_drag: None,
            create_drag: None,
            path_anchor_drag: None,
            marquee_drag: None,
            layer_drag: None,
            design_md_drag: None,
            component_browser_drag: None,
            icon_picker_drag: None,
            variables_resize: None,
            handle_drag: None,
            node_drag: None,
            next_node_id: 100,
            shift_held: false,
            now_ms: 0,
            wall_now_secs: 0,
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

    pub fn set_space_pan(&mut self, held: bool) {
        self.space_pan = held;
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
        #[cfg(feature = "canvaskit")]
        {
            self.doc_sync_dirty = true;
        }
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

    fn chat_transcript_text_offset_at_screen(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let chat_rect = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h)?;
        match op_editor_ui::widgets::AIChatPlaceholder::from_editor_at(
            &self.editor_state,
            self.now_ms,
        )
        .hit_test(chat_rect, Point2D::new(x, y))
        {
            Some(op_editor_ui::widgets::AIChatHit::SelectTranscriptText(message_index, offset)) => {
                Some((message_index, offset))
            }
            _ => None,
        }
    }

    fn chat_input_text_offset_at_screen(&self, x: f32, y: f32) -> Option<usize> {
        let chat_rect = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h)?;
        match op_editor_ui::widgets::AIChatPlaceholder::from_editor_at(
            &self.editor_state,
            self.now_ms,
        )
        .hit_test(chat_rect, Point2D::new(x, y))
        {
            Some(op_editor_ui::widgets::AIChatHit::SelectInputText(offset)) => Some(offset),
            _ => None,
        }
    }

    fn apply_chat_input_selection_drag_cursor_move(&mut self, x: f32, y: f32) -> bool {
        let Some(drag) = self.chat_input_selection_drag else {
            return false;
        };
        if let Some(focus) = self.chat_input_text_offset_at_screen(x, y) {
            if self
                .editor_state
                .chat
                .drag_input_selection(drag.anchor, focus, self.now_ms)
            {
                self.editor_state.chat.focused = true;
                self.mark_dirty();
            }
        }
        true
    }

    fn apply_chat_text_selection_drag_cursor_move(&mut self, x: f32, y: f32) -> bool {
        let Some(drag) = self.chat_text_selection_drag else {
            return false;
        };
        if let Some((message_index, focus)) = self.chat_transcript_text_offset_at_screen(x, y) {
            if message_index == drag.message_index {
                let next = Some(op_editor_core::chat::ChatTranscriptSelection {
                    message_index,
                    anchor: drag.anchor,
                    focus,
                });
                if self.editor_state.chat.transcript_selection != next {
                    self.editor_state.chat.transcript_selection = next;
                    self.mark_dirty();
                }
            }
        }
        true
    }

    fn try_scroll_chat_checklist(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let point = Point2D::new(x, y);
        let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) else {
            return false;
        };
        let (checklist, max) = {
            let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor_at(
                &self.editor_state,
                self.now_ms,
            );
            let Some(checklist) = panel.fixed_checklist_bounds(chat_rect) else {
                return false;
            };
            (checklist, panel.fixed_checklist_scroll_max())
        };
        if !(checklist).contains(point) {
            return false;
        }
        let before = self.editor_state.chat.checklist_scroll.offset;
        self.editor_state
            .chat
            .checklist_scroll
            .scroll_by(-delta, max, 0.0);
        if self.editor_state.chat.checklist_scroll.offset != before {
            self.mark_dirty();
        }
        true
    }

    /// Scroll the chat transcript message list when a wheel / trackpad
    /// pan lands over the panel body — pinned-to-bottom auto-follow
    /// resumes once the user scrolls back to the bottom. Mirrors the
    /// native host's `try_scroll_chat_transcript`.
    fn try_scroll_chat_transcript(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if self.editor_state.chat.messages.is_empty() {
            return false;
        }
        let point = Point2D::new(x, y);
        let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) else {
            return false;
        };
        let (body, max) = {
            let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor_at(
                &self.editor_state,
                self.now_ms,
            );
            (
                panel.body_rect(chat_rect),
                panel.transcript_scroll_max(chat_rect),
            )
        };
        if !(body).contains(point) {
            return false;
        }
        let chat = &mut self.editor_state.chat;
        if max <= 0.0 {
            if !chat.transcript_pinned || chat.transcript_scroll.offset != 0.0 {
                chat.transcript_pinned = true;
                chat.transcript_scroll.offset = 0.0;
                self.mark_dirty();
            }
            return true;
        }
        let cur = if chat.transcript_pinned {
            max
        } else {
            chat.transcript_scroll.offset.clamp(0.0, max)
        };
        let next = (cur - delta).clamp(0.0, max);
        let pinned = next >= max - 0.5;
        if (next - chat.transcript_scroll.offset).abs() > f32::EPSILON
            || chat.transcript_pinned != pinned
        {
            chat.transcript_scroll.offset = next;
            chat.transcript_pinned = pinned;
            self.mark_dirty();
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
            if (panel_rect).contains(point) {
                if let Some(max) = AgentSettingsPanel::for_editor(&self.editor_state)
                    .builtin_preset_scroll_max_at(panel_rect, point)
                {
                    let settings = &mut self.editor_state.editor_ui.agent_settings;
                    settings
                        .builtin_preset_menu_scroll
                        .scroll_by(-delta_y, max, 0.0);
                    self.mark_dirty();
                    return true;
                }
                let panel = AgentSettingsPanel::for_editor(&self.editor_state);
                let total = panel.content_total_height();
                let viewport_h_inner = panel_rect.size.y - 48.0;
                let max_scroll = (total - viewport_h_inner).max(0.0);
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .scroll_y
                    .scroll_by(-delta_y, max_scroll, 0.0);
                self.mark_dirty();
                return true;
            }
        }
        if self.try_scroll_chat_checklist(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_chat_transcript(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Floating VariablesPanel owns the wheel over its rect.
        if self.try_scroll_variables_panel(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_locale_picker(x, y, delta_y, viewport_width) {
            return true;
        }
        if self.try_scroll_design_md_panel(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Side rails scroll their panels instead of zooming the
        // canvas (`widget_host/scroll.rs`).
        if self.try_scroll_property_panel(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_layer_panel(x, y, 0.0, delta_y, viewport_height) {
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
        // A wheel zoom only changes the viewport (camera); the document-space
        // layout scene is unchanged, so keep the layout cache intact — no
        // `mark_dirty()` (matches native `scroll.rs`). The wheel listener
        // repaints off this `true` return.
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
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        if self.try_scroll_variables_panel(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_locale_picker(x, y, dy, viewport_width) {
            return true;
        }
        if self.try_scroll_design_md_panel(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_chat_checklist(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_chat_transcript(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_property_panel(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_layer_panel(x, y, dx, dy, viewport_height) {
            return true;
        }
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        self.editor_state.viewport.pan(dx, dy);
        // Trackpad pan only translates the viewport; keep the layout cache
        // intact — no `mark_dirty()` (matches the canvas pan-drag + native).
        true
    }

    pub fn apply_pan_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        self.drag = Some(DragState {
            last_x: x,
            last_y: y,
        });
        true
    }

    /// Update `editor_ui.hovered_layer_id` from the cursor.
    /// Returns true if hover state changed (caller should
    /// repaint). Mirrors the native host.
    pub fn update_layer_hover(&mut self, x: f32, y: f32, viewport_h: f32) -> bool {
        use op_editor_ui::widgets::{LayerPanel, LayerPanelHit, TOP_BAR_HEIGHT};
        let sidebar_open = self.editor_state.editor_ui.sidebar_open;
        let panel_w = self.editor_state.editor_ui.layer_panel_width;
        let blocked_by_overlay = self.over_topmost_panel(x, y, self.last_viewport_w, viewport_h)
            || self.over_dropdown_overlay(x, y, self.last_viewport_w, viewport_h);
        let (new_layer, new_page) = if sidebar_open
            && !blocked_by_overlay
            && y >= TOP_BAR_HEIGHT
            && x >= 0.0
            && x <= panel_w
        {
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

    pub(in crate::widget_host) fn clear_layer_panel_hover(&mut self) -> bool {
        let ui = &mut self.editor_state.editor_ui;
        let cleared_layer = ui.hovered_layer_id.take().is_some();
        let cleared_page = ui.hovered_page_index.take().is_some();
        let changed = cleared_layer || cleared_page;
        if changed {
            self.mark_dirty();
        }
        changed
    }

    pub(in crate::widget_host) fn clear_lower_overlay_hover(&mut self) -> bool {
        let mut changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            changed |= ui.canvas_hover_node.take().is_some();
            changed |= ui.hovered_layer_id.take().is_some();
            changed |= ui.hovered_page_index.take().is_some();
            changed |= ui.file_menu.hover.take().is_some();
            changed |= ui.locale_picker.hover.take().is_some();
            changed |= ui.shape_picker.hover.take().is_some();
            changed |= ui.fill_type_picker.hover.take().is_some();
            changed |= ui.toolbar_hover.take().is_some();
            changed |= ui.align_toolbar_hover.take().is_some();
            changed |= ui.statusbar_hover.take().is_some();
            changed |= ui.topbar_button_hover.take().is_some();
            changed |= ui.chat_model_picker.hover.take().is_some();
            changed |= ui.chat_design_block_hover.take().is_some();
            changed |= ui.chat_footer_hover.take().is_some();
            changed |= ui.chat_example_hover.take().is_some();
            changed |= ui.export_picker_hover.take().is_some();
            changed |= ui.property_action_hover.take().is_some();
            changed |= ui.property_tab_hover.take().is_some();
            if let Some(menu) = ui.layer_context_menu.as_mut() {
                changed |= menu.menu.hover.take().is_some();
            }
        }
        if let Some(menu) = self.editor_state.ui.path_anchor_menu.as_mut() {
            changed |= menu.menu.hover.take().is_some();
        }
        changed |= self.editor_state.codegen.framework_hover.take().is_some();
        changed |= self.editor_state.codegen.action_hover.take().is_some();
        if changed {
            self.mark_dirty();
        }
        changed
    }

    // Cursor-move dispatch (`apply_cursor_move` /
    // `update_agent_settings_hover`) lives in
    // `widget_host/cursor_input.rs`; mouse-release handling
    // (`apply_release[_with_viewport]` / `commit_marquee_selection` /
    // `commit_layer_drag`) in `widget_host/release_input.rs` — both
    // split out to keep this spine file under the 800-line ceiling.

    // Keyboard / clipboard handlers (`apply_text` / `apply_backspace`
    // / `apply_send` / `apply_delete` / `apply_duplicate` /
    // `apply_nudge` / `apply_select_all` / `apply_copy` /
    // `apply_cut` / `apply_paste` / `apply_reorder` /
    // `apply_escape` / `apply_ime` / `apply_key`) live in
    // `widget_host/keyboard.rs` — split out to keep this spine
    // file under the 800-line ceiling.

    // `paint` lives in `widget_host/paint.rs` — split out to keep
    // this file under the 800-line ceiling.
}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}
