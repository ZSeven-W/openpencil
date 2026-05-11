//! Step 4 widget glue — the only file in shell-web allowed to call
//! into `openpencil_shell_core::widgets` / `::document`. All widget
//! logic (state, paint, layout, accesskit) lives in shell-core; this
//! host is a thin paint-loop adapter that takes a `&mut WebBackend`
//! and dispatches to the editor-UI composition.
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
    AIChatHit, AIChatPlaceholder, LayerPanel, LayoutCx, LocalePicker, PropertyPanel, Toolbar,
    TopBar, TopBarHit, Widget, AI_CHAT_COLLAPSED_HEIGHT, AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_HEIGHT,
    AI_CHAT_WIDTH, LAYER_PANEL_WIDTH, LOCALE_PICKER_WIDTH, PROPERTY_PANEL_WIDTH, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
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
    pub(in crate::widget_host) document: Document,
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
    next_node_id: u64,
    /// Whether the shift key is currently held. The DOM listener
    /// updates this from every keyboard / mouse event so apply_press
    /// can branch on shift+click for multi-select. Matches the
    /// native host's `shift_held` flag.
    pub(in crate::widget_host) shift_held: bool,
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
/// host's `LayerDragState`.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct LayerDragState {
    pub(in crate::widget_host) source: openpencil_shell_core::document::NodeId,
    pub(in crate::widget_host) start_y: f32,
    pub(in crate::widget_host) current_x: f32,
    pub(in crate::widget_host) current_y: f32,
    pub(in crate::widget_host) active: bool,
}

/// Mirrors the native `apply_property_action` — wired off the
/// shared `PropertyPanelAction` enum so the web shell dispatches
/// flex-layout / size-checkbox clicks the same way the desktop
/// shell does.
fn apply_property_action_impl(
    document: &mut Document,
    action: openpencil_shell_core::widgets::PropertyPanelAction,
) {
    use openpencil_shell_core::widgets::PropertyPanelAction as A;
    match action {
        A::SetFlexLayout(mode) => document.ui.flex_layout = mode,
        A::ToggleSizeFillWidth => {
            document.ui.size_fill_width = !document.ui.size_fill_width;
        }
        A::ToggleSizeFillHeight => {
            document.ui.size_fill_height = !document.ui.size_fill_height;
        }
        A::ToggleSizeHugWidth => {
            document.ui.size_hug_width = !document.ui.size_hug_width;
        }
        A::ToggleSizeHugHeight => {
            document.ui.size_hug_height = !document.ui.size_hug_height;
        }
        A::ToggleSizeClipContent => {
            document.ui.size_clip_content = !document.ui.size_clip_content;
        }
        A::ToggleFillTypePicker => {
            document.ui.fill_type_picker_open = !document.ui.fill_type_picker_open;
        }
        A::SetFillType(t) => {
            // Per-node now (was `document.ui.fill_type` until
            // 2026-05-11). Mirrors native — gated by `is_editable`
            // inside the mutator.
            document.set_selected_fill_type(t);
            document.ui.fill_type_picker_open = false;
        }
    }
}

impl WidgetHost {
    fn apply_property_action(
        &mut self,
        action: openpencil_shell_core::widgets::PropertyPanelAction,
    ) {
        apply_property_action_impl(&mut self.document, action);
    }
}

impl WidgetHost {
    pub fn new() -> Self {
        Self {
            document: Document::sample(),
            theme: Theme::dark(),
            drag: None,
            chat_drag: None,
            marquee_drag: None,
            layer_drag: None,
            next_node_id: 100,
            shift_held: false,
        }
    }

    /// Forward the latest shift-key state from the DOM listener
    /// so apply_press can branch on shift+click. Web reads
    /// `MouseEvent.shiftKey` / `KeyboardEvent.shiftKey` per event
    /// and calls this just before dispatch.
    pub fn set_modifier_shift(&mut self, held: bool) {
        self.shift_held = held;
    }

    pub(in crate::widget_host) fn canvas_region(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (f32, f32, f32, f32) {
        let canvas_left = if self.document.ui.sidebar_open {
            self.document.ui.layer_panel_width
        } else {
            0.0
        };
        let has_property = self.document.property_panel_visible();
        let canvas_right = if has_property {
            viewport_w - self.document.ui.property_panel_width
        } else {
            viewport_w
        };
        let canvas_w = (canvas_right - canvas_left).max(0.0);
        let canvas_h = (viewport_h - TOP_BAR_HEIGHT).max(0.0);
        (canvas_left, TOP_BAR_HEIGHT, canvas_w, canvas_h)
    }

    fn over_canvas(&self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        let canvas_left = if self.document.ui.sidebar_open {
            self.document.ui.layer_panel_width
        } else {
            0.0
        };
        let has_property = self.document.property_panel_visible();
        let canvas_right = if has_property {
            viewport_w - self.document.ui.property_panel_width
        } else {
            viewport_w
        };
        x >= canvas_left && x <= canvas_right && y >= TOP_BAR_HEIGHT && y <= viewport_h
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
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        // Cursor is in canvas-local coords — subtract the live
        // canvas-region offset (sidebar collapse aware).
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let cursor = Point2D::new(x - cx0, y - cy0);
        self.document.viewport.zoom_at(cursor, delta_y);
        true
    }

    /// 2-finger trackpad pan — translate viewport by `(dx, dy)`.
    /// Same Figma-style separation as the native host: trackpad
    /// swipe pans, pinch / Cmd+wheel / mouse-wheel zooms.
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
        self.document.viewport.pan(dx, dy);
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

    /// Update `Document.ui.hovered_layer_id` from the cursor.
    /// Returns true if hover state changed (caller should
    /// repaint). Mirrors the native host.
    pub fn update_layer_hover(&mut self, x: f32, y: f32, viewport_h: f32) -> bool {
        use openpencil_shell_core::widgets::{LayerPanel, LayerPanelHit, TOP_BAR_HEIGHT};
        let new_hover = if self.document.ui.sidebar_open
            && y >= TOP_BAR_HEIGHT
            && x >= 0.0
            && x <= self.document.ui.layer_panel_width
        {
            let layer_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    self.document.ui.layer_panel_width,
                    (viewport_h - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let panel = LayerPanel::from_document(&self.document);
            match panel.hit_test(layer_rect, Point2D::new(x, y)) {
                Some(LayerPanelHit::Layer(id))
                | Some(LayerPanelHit::ToggleHidden(id))
                | Some(LayerPanelHit::ToggleLocked(id))
                | Some(LayerPanelHit::ToggleCollapsed(id)) => Some(id),
                _ => None,
            }
        } else {
            None
        };
        if new_hover != self.document.ui.hovered_layer_id {
            self.document.ui.hovered_layer_id = new_hover;
            true
        } else {
            false
        }
    }

    /// Cursor-move handler — drives canvas pan-drag, marquee
    /// drag, chat drag, or no-op.
    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        if let Some(m) = self.marquee_drag.as_mut() {
            m.current_screen_x = x;
            m.current_screen_y = y;
            return true;
        }
        if let Some(d) = self.layer_drag.as_mut() {
            // Drop the gesture if the source disappeared mid-drag —
            // see the native host for the rationale.
            let source_id = d.source;
            let still_present = self
                .document
                .active_page()
                .map(|p| p.find(source_id).is_some())
                .unwrap_or(false);
            if !still_present {
                self.layer_drag = None;
                return true;
            }
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
            self.document.viewport.pan(dx, dy);
            true
        } else {
            false
        }
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
        let to_doc = |sx: f32, sy: f32| -> Point2D {
            let local = Point2D::new(sx - cx0, sy - cy0);
            self.document.viewport.to_document(local)
        };
        let p0 = to_doc(m.start_screen_x, m.start_screen_y);
        let p1 = to_doc(m.current_screen_x, m.current_screen_y);
        let x = p0.x.min(p1.x);
        let y = p0.y.min(p1.y);
        let w = (p1.x - p0.x).abs();
        let h = (p1.y - p0.y).abs();
        let rect = Rect::xywh(x, y, w, h);
        let ids = self.document.nodes_intersecting_doc_rect(rect);
        if m.additive {
            // ADD-only: every hit joins the set; already-selected
            // hits stay selected. Shift-marquee never removes.
            for id in ids {
                if !self.document.is_selected(id) {
                    self.document.toggle_selection(id);
                }
            }
        } else if !ids.is_empty() {
            let anchor = *ids.last().unwrap();
            self.document.selected_set = ids;
            self.document.selected = anchor;
        }
    }

    /// Mouse-release handler — commits any marquee drag, snaps a
    /// chat-panel drag to the nearest corner, or ends the canvas
    /// pan-drag.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
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
            self.document.chat.anchor = ChatAnchor::nearest(center, cx0, cy0, cw, ch);
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging
    }

    /// Mouse-release handler — viewport-less variant.
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
        use openpencil_shell_core::widgets::{DropPosition, LayerPanel};
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = LayerPanel::from_document(&self.document);
        let cursor = Point2D::new(d.current_x, d.current_y);
        let Some(drop) = panel.drop_target_at(layer_rect, cursor) else {
            return true;
        };
        if drop.anchor == d.source {
            return true;
        }
        match drop.position {
            DropPosition::Before => {
                self.document.reorder_before(d.source, drop.anchor);
            }
            DropPosition::After => {
                self.document.reorder_after(d.source, drop.anchor);
            }
        }
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
        if self.document.chat.collapsed {
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
        let (x, y) = match self.document.chat.anchor {
            ChatAnchor::TopLeft => (cx0 + AICHAT_INSET_LEFT, cy0 + AICHAT_INSET_BOTTOM),
            ChatAnchor::TopRight => (
                cx0 + cw - panel_w - AICHAT_INSET_BOTTOM,
                cy0 + AICHAT_INSET_BOTTOM,
            ),
            ChatAnchor::BottomLeft => (
                cx0 + AICHAT_INSET_LEFT,
                cy0 + ch - panel_h - AICHAT_INSET_BOTTOM,
            ),
            ChatAnchor::BottomRight => (
                cx0 + cw - panel_w - AICHAT_INSET_BOTTOM,
                cy0 + ch - panel_h - AICHAT_INSET_BOTTOM,
            ),
        };
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
                self.document.ui.layer_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        }
    }

    fn toolbar_rect(&self, viewport_w: f32) -> Rect {
        // Anchor follows canvas_region (sidebar-collapse aware) so
        // hit-test matches paint regardless of sidebar state.
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_w, f32::INFINITY);
        let toolbar = Toolbar::for_document(&self.document);
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

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}
