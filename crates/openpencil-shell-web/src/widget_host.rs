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

use crate::backend::WebBackend;
use openpencil_shell_core::document::{ChatAnchor, Document};
use openpencil_shell_core::widgets::{
    AIChatHit, AIChatPlaceholder, CanvasViewport, LayerPanel, LayoutCx, PaintCx, PropertyPanel,
    StatusBar, Toolbar, TopBar, TopBarHit, Widget, AI_CHAT_COLLAPSED_HEIGHT,
    AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_HEIGHT, AI_CHAT_WIDTH, LAYER_PANEL_WIDTH,
    PROPERTY_PANEL_WIDTH, STATUS_BAR_HEIGHT, STATUS_BAR_WIDTH, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect, RenderBackend, Theme};

const TOOLBAR_INSET_X: f32 = 12.0;
const TOOLBAR_INSET_Y: f32 = 12.0;
const STATUS_INSET: f32 = 16.0;
const AICHAT_INSET_BOTTOM: f32 = 12.0;
const AICHAT_INSET_LEFT: f32 = 12.0;

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

pub struct WidgetHost {
    document: Document,
    theme: Theme,
    drag: Option<DragState>,
    chat_drag: Option<ChatDragState>,
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

impl WidgetHost {
    pub fn new() -> Self {
        Self {
            document: Document::sample(),
            theme: Theme::dark(),
            drag: None,
            chat_drag: None,
        }
    }

    fn canvas_region(&self, viewport_w: f32, viewport_h: f32) -> (f32, f32, f32, f32) {
        let canvas_left = if self.document.ui.sidebar_open {
            LAYER_PANEL_WIDTH
        } else {
            0.0
        };
        let has_property = self.document.selected_node().is_some();
        let canvas_right = if has_property {
            viewport_w - PROPERTY_PANEL_WIDTH
        } else {
            viewport_w
        };
        let canvas_w = (canvas_right - canvas_left).max(0.0);
        let canvas_h = (viewport_h - TOP_BAR_HEIGHT).max(0.0);
        (canvas_left, TOP_BAR_HEIGHT, canvas_w, canvas_h)
    }

    fn over_canvas(&self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        let canvas_left = if self.document.ui.sidebar_open {
            LAYER_PANEL_WIDTH
        } else {
            0.0
        };
        let has_property = self.document.selected_node().is_some();
        let canvas_right = if has_property {
            viewport_w - PROPERTY_PANEL_WIDTH
        } else {
            viewport_w
        };
        x >= canvas_left
            && x <= canvas_right
            && y >= TOP_BAR_HEIGHT
            && y <= viewport_h
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
        let cursor = Point2D::new(x - LAYER_PANEL_WIDTH, y - TOP_BAR_HEIGHT);
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
    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // 0. TopBar — sidebar toggle button. Mirrors the native
        //    host so web + native behave identically.
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        let top_bar = TopBar::untitled();
        if let Some(hit) = top_bar.hit_test(top_bar_rect, Point2D::new(x, y)) {
            match hit {
                TopBarHit::ToggleSidebar => {
                    self.document.ui.sidebar_open = !self.document.ui.sidebar_open;
                    return true;
                }
            }
        }
        if rect_contains(top_bar_rect, Point2D::new(x, y)) {
            return false;
        }

        // 1. AI chat panel — painted on top of toolbar so a
        //    click inside its rect is consumed here, even when
        //    that point lies inside the toolbar rect underneath.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel = AIChatPlaceholder::from_document(&self.document);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                if matches!(hit, AIChatHit::DragHandle) {
                    self.chat_drag = Some(ChatDragState {
                        grab_dx: x - chat_rect.origin.x,
                        grab_dy: y - chat_rect.origin.y,
                        pos_x: chat_rect.origin.x,
                        pos_y: chat_rect.origin.y,
                    });
                    self.document.chat.focused = false;
                    return true;
                }
                let _ = self.apply_click(x, y, viewport_width, viewport_height);
                return true;
            }
        }

        // 2. Toolbar — second-highest overlay. Bounding rect
        //    consumes all clicks (gaps + padding too) so it
        //    never falls through to the canvas for tool gaps
        //    that lie outside the chat panel.
        let toolbar_rect = self.toolbar_rect(viewport_width);
        let toolbar = Toolbar::for_document(&self.document);
        if rect_contains(toolbar_rect, Point2D::new(x, y)) {
            if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
                match hit {
                    openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                        self.document.tool = tool;
                        return true;
                    }
                    openpencil_shell_core::widgets::ToolbarHit::Action(_) => {
                        return false;
                    }
                }
            }
            return false;
        }

        // 3. apply_click — LayerPanel + chat-defocus.
        if self.apply_click(x, y, viewport_width, viewport_height) {
            return true;
        }

        // 4. Empty-canvas click: clear selection (collapses the
        //    PropertyPanel) + start a pan-drag, mirroring native.
        if self.over_canvas(x, y, viewport_width, viewport_height) {
            let cleared =
                self.document.selected != openpencil_shell_core::document::NodeId::NONE;
            if cleared {
                self.document.selected = openpencil_shell_core::document::NodeId::NONE;
            }
            self.drag = Some(DragState { last_x: x, last_y: y });
            return cleared;
        }
        false
    }

    /// Cursor-move handler — drives canvas pan-drag, chat drag,
    /// or no-op.
    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
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

    /// Mouse-release handler — snaps the chat panel to the
    /// nearest canvas corner if a chat drag was in flight; else
    /// ends the canvas pan-drag.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        if let Some(d) = self.chat_drag.take() {
            let center = Point2D::new(
                d.pos_x + AI_CHAT_WIDTH / 2.0,
                d.pos_y + AI_CHAT_HEIGHT / 2.0,
            );
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
        if self.chat_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging
    }

    /// Step 5 P2: push a typed character into the focused chat
    /// input. Returns true if anything changed.
    pub fn apply_text(&mut self, c: char) -> bool {
        if !self.document.chat.focused {
            return false;
        }
        self.document.chat.input.push(c);
        true
    }

    /// Backspace on the focused chat input.
    pub fn apply_backspace(&mut self) -> bool {
        if !self.document.chat.focused {
            return false;
        }
        self.document.chat.input.pop().is_some()
    }

    /// Send the focused chat input.
    pub fn apply_send(&mut self) -> bool {
        if self.document.chat.input.trim().is_empty() {
            return false;
        }
        self.document.chat.send();
        true
    }

    /// Phase C2 IME forwarding stub — Step 5+ wires per-widget focus.
    pub fn apply_ime(&mut self, _event: &openpencil_shell_core::ImeEvent) { // glue:
    }

    /// Phase C2 keyboard forwarding stub.
    pub fn apply_key(&mut self, _event: &openpencil_shell_core::KeyEvent) { // glue:
    }

    fn ai_chat_size(&self) -> (f32, f32) {
        if self.document.chat.collapsed {
            (AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_COLLAPSED_HEIGHT)
        } else {
            (AI_CHAT_WIDTH, AI_CHAT_HEIGHT)
        }
    }

    fn ai_chat_rect(&self, viewport_w: f32, viewport_h: f32) -> Option<Rect> {
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
    pub fn apply_click(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        // glue:
        // Floating chat panel sits on top — check first so its
        // clicks don't fall through to the canvas.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_w, viewport_h) {
            let panel = AIChatPlaceholder::from_document(&self.document);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                match hit {
                    AIChatHit::FocusInput => {
                        self.document.chat.focused = true;
                        return true;
                    }
                    AIChatHit::Send => {
                        self.document.chat.send();
                        return true;
                    }
                    AIChatHit::Example(text) => {
                        self.document.chat.input = text;
                        self.document.chat.focused = true;
                        return true;
                    }
                    AIChatHit::DragHandle => {
                        return false;
                    }
                    AIChatHit::ToggleCollapse => {
                        self.document.chat.collapsed =
                            !self.document.chat.collapsed;
                        return true;
                    }
                }
            }
        }
        let was_focused = self.document.chat.focused;
        self.document.chat.focused = false;

        let toolbar_rect = self.toolbar_rect(viewport_w);
        let toolbar = Toolbar::for_document(&self.document);
        if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                    self.document.tool = tool;
                    return true;
                }
                openpencil_shell_core::widgets::ToolbarHit::Action(_) => {
                    return false;
                }
            }
        }
        if !self.document.ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = LayerPanel::from_document(&self.document);
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::LayerPanelHit::Page(idx) => {
                    self.document.active_page_index = idx;
                    self.document.selected =
                        openpencil_shell_core::document::NodeId::NONE;
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::Layer(node_id) => {
                    self.document.selected = node_id;
                    return true;
                }
            }
        }
        // Defocusing the chat input itself is a visible change —
        // the caller should still repaint to drop the caret.
        was_focused
    }

    fn layer_panel_rect(&self, viewport_h: f32) -> Rect {
        Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(LAYER_PANEL_WIDTH, (viewport_h - TOP_BAR_HEIGHT).max(0.0)),
        }
    }

    fn toolbar_rect(&self, _viewport_w: f32) -> Rect {
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
            origin: Point2D::new(LAYER_PANEL_WIDTH + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, h),
        }
    }

    /// Dispatches paint to the editor-UI composition.
    pub fn paint(
        &self,
        backend: &mut WebBackend,
        viewport_width: f32,
        viewport_height: f32,
    ) { // glue:
        // 1. Background. Even before any widget paints, fill the
        //    whole viewport with `theme.background` so `<canvas>`
        //    pixels left over from a smaller previous frame don't
        //    bleed through.
        backend.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, viewport_height),
            },
            self.theme.background,
        );

        let dpi = backend.dpi_scale();

        // 2. TopBar — full width, pinned top.
        let top_bar = TopBar::untitled();
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            top_bar.paint(&mut cx, top_bar_rect);
        }

        // 3. LayerPanel — left rail (skipped when sidebar
        //    collapsed; canvas extends to the left edge).
        if self.document.ui.sidebar_open {
            let layer_panel_rect = self.layer_panel_rect(viewport_height);
            let layer_panel = LayerPanel::from_document(&self.document);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            layer_panel.paint(&mut cx, layer_panel_rect);
        }

        // 4. PropertyPanel — right rail, ONLY when selection exists.
        let property_panel = PropertyPanel::for_selection(&self.document);
        let property_rect = if property_panel.is_some() {
            Rect {
                origin: Point2D::new(viewport_width - PROPERTY_PANEL_WIDTH, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    PROPERTY_PANEL_WIDTH,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            }
        } else {
            Rect {
                origin: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
                size: Point2D::new(0.0, 0.0),
            }
        };
        if let Some(panel) = property_panel.as_ref() {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, property_rect);
        }

        // 5. CanvasViewport — fills the middle band between the
        //    rails, below the top bar. Respects the sidebar
        //    collapse state via canvas_region.
        let (canvas_left, _canvas_y, canvas_w, canvas_h) =
            self.canvas_region(viewport_width, viewport_height);
        let canvas_rect = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        let canvas = CanvasViewport::from_document(&self.document);
        if canvas_w > 0.0 && canvas_h > 0.0 {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            canvas.paint(&mut cx, canvas_rect);
        }

        // 6. Toolbar — floating column.
        let toolbar = Toolbar::for_document(&self.document);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(
                canvas_left + TOOLBAR_INSET_X,
                TOP_BAR_HEIGHT + TOOLBAR_INSET_Y,
            ),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
        if canvas_w > TOOLBAR_WIDTH + TOOLBAR_INSET_X * 2.0 {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            toolbar.paint(&mut cx, toolbar_rect);
        }

        // 7. AIChatPlaceholder — painted LAST so it sits on top
        //    of the toolbar in any overlap region.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let chat = AIChatPlaceholder::from_document(&self.document);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            chat.paint(&mut cx, chat_rect);
        }

        // 8. StatusBar — floating bottom-right of canvas.
        let canvas_right = canvas_left + canvas_w;
        if canvas_w > STATUS_BAR_WIDTH + STATUS_INSET * 2.0 {
            let status = StatusBar::new();
            let status_rect = Rect {
                origin: Point2D::new(
                    canvas_right - STATUS_BAR_WIDTH - STATUS_INSET,
                    TOP_BAR_HEIGHT + canvas_h - STATUS_BAR_HEIGHT - STATUS_INSET,
                ),
                size: Point2D::new(STATUS_BAR_WIDTH, STATUS_BAR_HEIGHT),
            };
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            status.paint(&mut cx, status_rect);
        }
    }
}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}
