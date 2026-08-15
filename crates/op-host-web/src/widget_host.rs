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

use op_editor_ui::widgets::host_frame_bookkeeping as bookkeeping;
use op_editor_ui::widgets::TOP_BAR_HEIGHT;
use op_editor_ui::{Point2D, Rect, Theme};

mod a11y_bridge;
mod account_press;
#[cfg(test)]
mod agent_settings_acp_press_tests;
#[cfg(test)]
mod agent_settings_compact_press_tests;
#[cfg(test)]
mod agent_settings_form_press_tests;
mod agent_settings_hover;
mod agent_settings_mcp_server;
#[cfg(test)]
mod agent_settings_model_input_tests;
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
#[cfg(test)]
mod canvas_hierarchy_tests;
mod chat_design_apply;
#[cfg(test)]
mod chat_design_apply_tests;
mod chat_design_hover;
#[cfg(test)]
mod chat_design_hover_tests;
#[cfg(test)]
mod chat_input_caret_tests;
mod chat_model_picker_caret;
#[cfg(test)]
mod chat_model_picker_caret_tests;
#[cfg(test)]
mod chat_send_tests;
mod chrome_menu_press;
mod click;
#[cfg(test)]
mod codegen_framework_tests;
#[cfg(test)]
mod collab_id_alloc_tests;
#[cfg(test)]
mod collab_input_tests;
mod color_picker_press;
mod component_browser_press;
mod cursor_input;
mod cursor_input_modals;
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
#[cfg(test)]
mod font_picker_keyboard_tests;
mod geometry;
mod group_ops;
mod history_guard;
mod host_lifecycle;
mod hover_clear;
mod hover_property_panel;
mod html_import_diagnostics_press;
mod icon_picker_press;
mod image_crop_drag;
#[cfg(test)]
mod image_crop_drag_tests;
mod image_panel_dispatch;
#[cfg(test)]
mod image_panel_overlay_tests;
mod image_panel_selection;
#[cfg(test)]
mod image_panel_selection_tests;
#[cfg(all(test, feature = "canvaskit"))]
mod io_tests;
mod keyboard;
mod keyboard_edit_ops;
mod keyboard_escape;
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
mod missing_fonts_press;
mod node_drag;
#[cfg(test)]
mod node_drag_tests;
mod overlay_cursor;
mod overlay_keys;
#[cfg(test)]
mod overlay_press_tests;
mod overlay_rects;
#[cfg(test)]
mod page_switch_center_tests;
mod paint;
#[cfg(test)]
mod paint_caret_tests;
#[cfg(test)]
mod pan_tests;
mod pen_press;
#[cfg(test)]
mod pen_press_tests;
mod press;
mod press_canvas_tiers;
mod press_chrome_tiers;
mod press_ctx;
mod press_overlay_tiers;
mod press_property_tiers;
mod press_surface_tiers;
#[cfg(test)]
mod prompt_center_host_tests;
mod prompt_center_press;
#[cfg(test)]
mod property_compositing_tests;
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
#[cfg(test)]
mod scene_template_ime_tests;
mod scene_template_press;
mod scroll;
mod settings_caret;
#[cfg(test)]
mod settings_caret_tests;
mod shape_create;
#[cfg(test)]
mod shape_create_tests;
mod shape_picker_press;
mod slides_panel;
mod text_drag;
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
mod variables_preset_press;
mod viewport_fit;
mod web_fonts;
mod wheel_pan;

/// Device-login side effects requested by press dispatchers and executed
/// by `web_auth_sync` against the daemon's `/api/auth/*` proxy. (Begin is
/// NOT queued: it must fire inside the click's user-activation window so
/// the sign-in popup isn't blocked — see `web_auth_sync::begin_login_now`.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingAuthAction {
    /// Abort the in-flight flow (modal dismissed).
    CancelLogin,
    /// Drop the shared session and revoke the device token.
    SignOut,
}

// Floating-chrome insets live with the canvas-region math they are
// measured from, so the native + web hosts can't drift on where the
// chat pill / toolbar / status pill sit.
#[allow(unused_imports)]
pub(in crate::widget_host) use op_editor_ui::widgets::host_canvas_geometry::{
    AICHAT_INSET_BOTTOM, AICHAT_INSET_LEFT, STATUS_INSET, TOOLBAR_INSET_X, TOOLBAR_INSET_Y,
};

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
    /// Paint-only interpolation from the previous layout scene to the
    /// current one. Used by canvas node drag/reorder previews.
    pub(in crate::widget_host) layout_transition:
        Option<op_editor_ui::widgets::CanvasLayoutTransition>,
    /// Skips the `layout_scene` rebuild when the document / active theme /
    /// active page are unchanged. `editor_state_dirty` fires on nearly every
    /// interaction (hover, scroll, selection, caret drafts, and per-frame chat /
    /// codegen streaming), but most leave the scene inputs identical.
    pub(in crate::widget_host) scene_cache: op_pen_loader::SceneBuildCache,
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
    /// Monotonic identity of the complete EditorState installed in this host.
    /// EditorState's generation may restart at zero across Open/New/import, so
    /// delayed acknowledgements pair that generation with this epoch.
    document_epoch: u64,
    /// Per-mount owner token for asynchronous HTML/Figma document imports.
    pub(crate) document_import_generation: u64,
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
    /// Active bitmap pan while the selected image fill is in crop edit mode.
    image_crop_drag: Option<image_crop_drag::ImageCropDragState>,
    effect_radius_drag: Option<usize>,
    /// Active generated-code preview text selection drag.
    code_selection_drag: Option<CodeSelectionDragState>,
    /// Active chat input text selection drag.
    chat_input_selection_drag: Option<ChatInputSelectionDragState>,
    /// Active Search / Generate popover input selection drag.
    image_input_selection_drag: Option<image_panel_selection::ImageInputSelectionDragState>,
    /// Latest Search / Generate input geometry measured by the real painter.
    image_input_geometry:
        Option<op_editor_ui::widgets::property_panel_image_assets::ImagePopoverInputGeometry>,
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
    /// Original selected ids for an active Option-drag clone move.
    /// Drop hit-testing skips these so a fresh clone does not
    /// immediately reparent back into the source it overlaps.
    pub(in crate::widget_host) option_drag_source_ids: Vec<op_editor_core::NodeId>,
    /// Counter for minting fresh `NodeId`s when the user duplicates
    /// a node. Bumped past the highest sample id so new + sample
    /// nodes never collide on the same key. Matches the native
    /// host's allocator.
    pub(in crate::widget_host) next_node_id: u64,
    /// Owner-assigned id allocator, set while a collaboration session is live.
    ///
    /// The protocol replays node ids verbatim, so a peer minting from the
    /// private `next_node_id` counter would eventually agree on an id for two
    /// different nodes. `None` restores the standalone policy.
    pub(in crate::widget_host) collab_id_allocator: Option<op_editor_core::DocumentIdAllocator>,
    /// Whether the shift key is currently held. The DOM listener
    /// updates this from every keyboard / mouse event so apply_press
    /// can branch on shift+click for multi-select. Matches the
    /// native host's `shift_held` flag.
    pub(in crate::widget_host) shift_held: bool,
    /// Whether Alt/Option is currently held. Node dragging uses this
    /// to duplicate the current selection before moving it.
    pub(in crate::widget_host) alt_held: bool,
    /// Host clock in ms — set by `lib.rs` on each event from
    /// `performance.now()`. Used for double-click detection.
    pub(in crate::widget_host) now_ms: u64,
    /// Unix wall-clock seconds. Kept separate from `now_ms` because
    /// browser `performance.now()` is navigation-relative, while
    /// recent-file timestamps are Unix seconds.
    pub(in crate::widget_host) wall_now_secs: u64,
    /// Rect the toast banner painted last frame, or `None` when nothing
    /// painted. Cached because the banner's width is measured in the font it
    /// paints with, which only the paint pass can do — the press arm re-checks
    /// the toast is still live before trusting it (see
    /// `op_editor_ui::widgets::editor_toast_flow::press`).
    pub(in crate::widget_host) toast_rect: Option<op_editor_ui::Rect>,
    /// Most recent viewport size seen via `apply_press` etc. — cached
    /// so `apply_cursor_move(x, y)` can rebuild the canvas region
    /// when its signature can't carry viewport dims (mirrors native).
    pub(in crate::widget_host) last_viewport_w: f32,
    pub(in crate::widget_host) last_viewport_h: f32,
    /// Most recent pointer position in logical canvas coordinates. Older text
    /// inputs that do not yet publish precise caret geometry use this as the
    /// browser IME candidate-window fallback instead of viewport (0, 0).
    pub(in crate::widget_host) last_cursor_x: f32,
    pub(in crate::widget_host) last_cursor_y: f32,
    /// Queued device-login actions for the daemon auth proxy; the
    /// `web_auth_sync` poll tick drains them into `/api/auth/*` calls.
    /// Press dispatchers only enqueue — they never issue requests.
    pub(in crate::widget_host) pending_auth_actions: Vec<PendingAuthAction>,
    /// Stable, process-unique id scoping this host's chat-panel transcript
    /// cache (mirrors native `WidgetHostNative::chat_panel_owner`). Stamped onto
    /// every `AIChatPlaceholder` this host builds so the thread-local canonical
    /// slot is owned by this panel instance and never cross-paired with another.
    pub(in crate::widget_host) chat_panel_owner: u64,
    /// Stable, process-unique id scoping this host's LayerPanel row-model
    /// cache (mirrors native `WidgetHostNative::layer_panel_owner`).
    /// Stamped onto the per-frame paint build (`from_editor_owned`). Page
    /// switches don't rotate (the key includes `active_page_index`), but
    /// whole-document replacements do — see
    /// `force_rotate_layer_panel_owner` (revision counters restart at 0).
    pub(in crate::widget_host) layer_panel_owner: u64,
    /// Active chat-session (tab) index observed at the last owner rotation; a
    /// change rotates `chat_panel_owner` (see
    /// [`Self::rotate_chat_owner_if_session_changed`]).
    pub(in crate::widget_host) last_chat_session_index: usize,
}

impl WidgetHost {
    // Single-clock setters — retained for parity with the native host's
    // `WidgetHostNative`, whose tests advance one clock at a time. The web
    // mount only ever sets both together via `set_clocks`.
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

    /// Companion to `caret_animation_active` — unwired for the same reason
    /// (the CanvasKit mount has no deadline pump to feed it).
    #[allow(dead_code)]
    pub fn next_animation_deadline_ms(&self) -> Option<u64> {
        // The web host contributes no platform clauses of its own — the
        // native spine folds gesture-degrade / pan-cache / preview / git-clone
        // wake-ups onto this same base.
        bookkeeping::base_animation_deadline_ms(
            &self.editor_state,
            self.layout_transition.as_ref(),
            self.now_ms,
        )
    }

    /// Whether a top-bar tooltip is waiting out its dwell — i.e. a
    /// repaint is owed at a future instant with no DOM event to carry
    /// it. The mount's rAF pump (`tooltip_pump`) drives that repaint,
    /// standing in for the deadline scheduler the browser host lacks.
    pub fn top_bar_tooltip_pending(&self) -> bool {
        op_editor_ui::widgets::top_bar_tooltip::next_deadline_ms(
            &self.editor_state.editor_ui,
            self.now_ms,
        )
        .is_some()
    }

    pub fn layout_transition_active(&self) -> bool {
        self.layout_transition
            .as_ref()
            .is_some_and(|transition| transition.is_active(self.now_ms))
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

    /// Identity of the currently installed complete EditorState.
    #[cfg(feature = "canvaskit")]
    pub(crate) fn document_epoch(&self) -> u64 {
        self.document_epoch
    }

    /// Install a complete state without rebuilding it through
    /// `EditorState::replace_document`, which would drop incoming document-
    /// adjacent state. Every whole-state seam uses this helper so async work
    /// observes one monotonic identity and host caches cannot alias revision 0.
    #[cfg(feature = "canvaskit")]
    pub(crate) fn replace_editor_state(&mut self, state: op_editor_core::EditorState) {
        self.editor_state = state;
        self.document_epoch = self.document_epoch.wrapping_add(1).max(1);
        self.force_rotate_layer_panel_owner();
        self.scene_cache.invalidate();
        self.mark_editor_state_dirty();
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

    /// Force the next `refresh_layout_scene` to rebuild the layout scene even
    /// though the document is unchanged. A font import / removal / restore
    /// changes text metrics (and which family renders) without touching the
    /// doc/page/theme, and the web build has no `jian_skia` font-generation
    /// signal for the scene cache to detect it (the native host watches that
    /// generation instead). So the font paths call this to avoid a stale scene.
    #[cfg(feature = "canvaskit")]
    pub fn invalidate_layout_scene(&mut self) {
        self.scene_cache.invalidate();
    }
}

// Transient pointer-drag records that carry no platform types are
// single-sourced in `op_editor_core::host_drag_state` (the native spine
// declared byte-identical copies). Re-exported here under their
// historical `widget_host` names so every submodule's `use super::{…}`
// and struct-literal site stays unchanged.
pub(in crate::widget_host) use op_editor_core::host_drag_state::{
    AnchorDragTarget, ChatDragState, ChatInputSelectionDragState, ChatTextSelectionDragState,
    CodeSelectionDragState, DragState, LayerDragState, MarqueeDragState, PanelDragState,
    PathAnchorDragState,
};

impl WidgetHost {}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}
