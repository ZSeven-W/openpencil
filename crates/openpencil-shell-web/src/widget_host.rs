//! Step 4 widget glue — the only file in shell-web allowed to call
//! into `openpencil_shell_core::widgets`. All widget logic (state,
//! paint, layout, accesskit) lives in shell-core; this host is a
//! thin paint-loop adapter that takes a `&mut WebBackend` and
//! dispatches to the editor-UI composition.
//!
//! ### State model — `EditorState` is the source of truth
//!
//! Like the native host (`openpencil-shell-native/src/widget_host.rs`),
//! the web `WidgetHost` holds an `op_editor_core::EditorState` as its
//! single source of truth. shell-core's ~30 widgets + hit-test helpers
//! are `&Document`-bound and read-only; they are fed a derived
//! read-only `Document` snapshot (`paint_doc`), rebuilt lazily by
//! `paint_document()` whenever `editor_state_dirty` is set. Every
//! mutation routes through an `op-editor-core` mutator on
//! `editor_state` and flags the snapshot dirty.
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
//! Functions that pull in `openpencil_shell_core::widgets::*` MUST live
//! in this file (per spec §1.4). Phase B4 boundary check enforces.

use openpencil_shell_core::document::{ChatAnchor, Document};
use openpencil_shell_core::widgets::{
    LocalePicker, Toolbar, TopBar, Widget, LayoutCx, AI_CHAT_COLLAPSED_HEIGHT,
    AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_HEIGHT, AI_CHAT_WIDTH, LOCALE_PICKER_WIDTH,
    TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect, Theme};

mod keyboard;
mod paint;
mod press;

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
    /// mutate this; the paint pass derives a read-only `Document`
    /// snapshot from it (see `paint_doc` / `paint_document`).
    pub(in crate::widget_host) editor_state: op_editor_core::EditorState,
    /// Derived paint-only `Document` snapshot of `editor_state`.
    /// shell-core's widgets + hit-test helpers are `&Document`-bound
    /// and read-only; they are fed this snapshot, never `editor_state`.
    /// Rebuilt lazily by `paint_document()` whenever
    /// `editor_state_dirty` is set.
    pub(in crate::widget_host) paint_doc: Document,
    /// Set whenever `editor_state` is mutated. Drives the lazy rebuild
    /// of `paint_doc` — `paint_document()` rebuilds + clears the flag,
    /// so a sequence of mutations only re-derives once.
    pub(in crate::widget_host) editor_state_dirty: bool,
    pub(in crate::widget_host) theme: Theme,
    drag: Option<DragState>,
    chat_drag: Option<ChatDragState>,
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
    pub(in crate::widget_host) source: openpencil_shell_core::document::NodeId,
    pub(in crate::widget_host) start_y: f32,
    pub(in crate::widget_host) current_x: f32,
    pub(in crate::widget_host) current_y: f32,
    pub(in crate::widget_host) active: bool,
}

impl WidgetHost {
    pub fn new() -> Self {
        let editor_state = op_editor_core::EditorState::sample();
        // Seed the paint snapshot once up front; subsequent frames
        // re-derive only when `editor_state_dirty` is set.
        let paint_doc = derive_paint_doc(&editor_state);
        Self {
            editor_state,
            paint_doc,
            editor_state_dirty: false,
            theme: Theme::dark(),
            drag: None,
            chat_drag: None,
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

    /// Derive a fresh paint-only `Document` snapshot from
    /// `editor_state` if `editor_state_dirty` is set; clear the flag.
    /// Cheap no-op when the snapshot is already current.
    pub(in crate::widget_host) fn refresh_paint_doc(&mut self) {
        if self.editor_state_dirty {
            self.paint_doc = derive_paint_doc(&self.editor_state);
            self.editor_state_dirty = false;
        }
    }

    /// The read-only paint `Document` snapshot of the live
    /// `EditorState`. Rebuilt on demand when the state changed since
    /// the last derive. Every widget paint + `&Document`-bound
    /// hit-test reads through this. Public so a future web runner
    /// (file I/O / export) can read the derived document.
    #[allow(dead_code)]
    pub fn paint_document(&mut self) -> &Document {
        self.refresh_paint_doc();
        &self.paint_doc
    }

    /// Mark `editor_state` as mutated so the next `paint_document()`
    /// re-derives the paint snapshot. Call after any direct mutation
    /// of `self.editor_state`.
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

    /// Mouse-press handler. Hit-test order mirrors paint order in
    /// REVERSE so the topmost overlay always wins (Step 5 codex
    /// stop-hook fix — toolbar paints AFTER the chat panel, so it
    /// sits on top in any overlap region and must be checked
    /// first, otherwise the taller chat panel intercepts toolbar
    /// clicks):
    ///   1. Toolbar (top z-order among overlays)
    ///   2. AI chat panel — DragHandle starts a chat drag,
    ///      everything else defers to apply_click
    ///   3. apply_click handles AI chat focus/send/example +
    ///      LayerPanel hit + chat-defocus side effect
    ///   4. Otherwise: start canvas pan-drag.

    /// Update `editor_ui.hovered_layer_id` from the cursor.
    /// Returns true if hover state changed (caller should
    /// repaint). Mirrors the native host.
    pub fn update_layer_hover(&mut self, x: f32, y: f32, viewport_h: f32) -> bool {
        use openpencil_shell_core::widgets::{LayerPanel, LayerPanelHit, TOP_BAR_HEIGHT};
        let sidebar_open = self.editor_state.editor_ui.sidebar_open;
        let panel_w = self.editor_state.editor_ui.layer_panel_width;
        let (new_layer, new_page) = if sidebar_open
            && y >= TOP_BAR_HEIGHT
            && x >= 0.0
            && x <= panel_w
        {
            self.refresh_paint_doc();
            let layer_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(panel_w, (viewport_h - TOP_BAR_HEIGHT).max(0.0)),
            };
            let panel = LayerPanel::from_document(&self.paint_doc);
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
        let new_layer_ec = new_layer.as_ref().map(op_pen_loader::rev::node_id);
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
    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        // Refresh the derived paint doc once up front so every hit-test
        // below (layer context menu, layer drag, align toolbar) reads
        // current geometry, never a stale snapshot.
        self.refresh_paint_doc();
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use openpencil_shell_core::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(
                &self.paint_doc,
                self.paint_doc.ui.layer_context_menu.clone().unwrap(),
            );
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
                .paint_doc
                .active_page()
                .map(|p| p.find(&source_id).is_some())
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
        if let Some(drag) = self.drag.as_mut() {
            let dx = x - drag.last_x;
            let dy = y - drag.last_y;
            drag.last_x = x;
            drag.last_y = y;
            self.editor_state.viewport.pan(dx, dy);
            self.mark_dirty();
            return true;
        }
        // No drag active — sync align toolbar hover. AFTER all drag
        // branches so an active drag isn't intercepted (codex CONCERN
        // — mirrors native widget_host/input.rs ordering).
        let new_hover = if self.editor_state.selection_count() >= 2 {
            use openpencil_shell_core::widgets::{AlignToolbar, TOP_BAR_HEIGHT};
            let (cx, _, cw, ch) =
                self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let canvas_region = openpencil_shell_core::Rect {
                origin: Point2D::new(cx, TOP_BAR_HEIGHT),
                size: Point2D::new(cw, ch),
            };
            AlignToolbar::for_canvas_region(canvas_region, &self.editor_state)
                .and_then(|tb| tb.hit_test(Point2D::new(x, y)))
        } else {
            None
        };
        let new_hover_ec = new_hover.map(op_pen_loader::rev::align_action);
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
        // `nodes_intersecting_doc_rect` is a `&Document`-bound helper —
        // it returns shell-core `NodeId`s.
        self.refresh_paint_doc();
        let ids = self.paint_doc.nodes_intersecting_doc_rect(rect);
        if m.additive {
            // ADD-only: every hit joins the set; already-selected
            // hits stay selected. Shift-marquee never removes.
            for id in ids {
                let ec_id = op_pen_loader::rev::node_id(&id);
                if !self.editor_state.is_selected(&ec_id) {
                    self.editor_state.toggle_selection(ec_id);
                }
            }
            self.mark_dirty();
        } else if !ids.is_empty() {
            let ec_ids: Vec<op_editor_core::NodeId> =
                ids.iter().map(op_pen_loader::rev::node_id).collect();
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
        if self.layer_drag.take().is_some() {
            // Same — drop_target_at needs the panel rect (and thus
            // viewport_h). Drop the candidate without committing.
            return true;
        }
        if self.chat_drag.take().is_some() {
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
        self.refresh_paint_doc();
        // Defensive source-validity check (mirrors native) — bail
        // if the dragged node disappeared between move and release.
        if self
            .paint_doc
            .active_page()
            .map(|p| p.find(&d.source).is_none())
            .unwrap_or(true)
        {
            return false;
        }
        use openpencil_shell_core::widgets::{DropPosition, LayerPanel};
        let layer_rect = self.layer_panel_rect(viewport_h);
        // Source-excluded panel so the indicator y the user saw and
        // the row landed on match the post-commit layout — see the
        // native `commit_layer_drag` for the rationale.
        let panel =
            LayerPanel::from_document_with_drag_source(&self.paint_doc, d.source.clone());
        let cursor = Point2D::new(d.current_x, d.current_y);
        let Some(drop) = panel.drop_target_at(layer_rect, cursor) else {
            return true;
        };
        if drop.anchor == d.source {
            return true;
        }
        let source = op_pen_loader::rev::node_id(&d.source);
        let anchor = op_pen_loader::rev::node_id(&drop.anchor);
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

    fn ai_chat_size(&self) -> (f32, f32) {
        if self.editor_state.chat.collapsed {
            (AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_COLLAPSED_HEIGHT)
        } else {
            (AI_CHAT_WIDTH, AI_CHAT_HEIGHT)
        }
    }

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

    pub(in crate::widget_host) fn ai_chat_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let (panel_w, panel_h) = self.ai_chat_size();
        if cw <= panel_w + AICHAT_INSET_LEFT + 16.0 || ch <= panel_h + 16.0 {
            return None;
        }
        if let Some(d) = self.chat_drag {
            return Some(Rect {
                origin: Point2D::new(d.pos_x, d.pos_y),
                size: Point2D::new(panel_w, panel_h),
            });
        }
        // `editor_state.chat.anchor` is op-editor-core's `ChatAnchor`;
        // shell-core's is a structurally identical four-variant enum.
        let (x, y) = match self.editor_state.chat.anchor {
            op_editor_core::ChatAnchor::TopLeft => {
                (cx0 + AICHAT_INSET_LEFT, cy0 + AICHAT_INSET_BOTTOM)
            }
            op_editor_core::ChatAnchor::TopRight => (
                cx0 + cw - panel_w - AICHAT_INSET_BOTTOM,
                cy0 + AICHAT_INSET_BOTTOM,
            ),
            op_editor_core::ChatAnchor::BottomLeft => (
                cx0 + AICHAT_INSET_LEFT,
                cy0 + ch - panel_h - AICHAT_INSET_BOTTOM,
            ),
            op_editor_core::ChatAnchor::BottomRight => (
                cx0 + cw - panel_w - AICHAT_INSET_BOTTOM,
                cy0 + ch - panel_h - AICHAT_INSET_BOTTOM,
            ),
        };
        // `ChatAnchor` import kept for the `nearest` call in release.
        let _ = ChatAnchor::TopLeft;
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(panel_w, panel_h),
        })
    }

    /// Apply a primary-button mouse click at `(x, y)` (canvas-local
    /// coordinates). Routes to AI chat / Toolbar / LayerPanel
    /// hit-tests, in floating-z-order. Returns `true` if the click
    /// was consumed by a widget so the caller knows whether to
    /// repaint.

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
        self.refresh_paint_doc();
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

    // `paint` lives in `widget_host/paint.rs` — split out to keep
    // this file under the 800-line ceiling.
}

/// Derive a faithful paint-only `Document` from an `EditorState`:
/// node tree + geometry from the canonical doc, then the chrome /
/// chat / components / variable / scalar state layered on.
pub(in crate::widget_host) fn derive_paint_doc(
    state: &op_editor_core::EditorState,
) -> Document {
    let mut d = op_pen_loader::pen_document_to_document(&state.doc);
    op_pen_loader::apply_editor_state_ui(&mut d, state);
    d
}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}
