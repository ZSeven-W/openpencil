//! Step 4 native widget glue — the only file in shell-native allowed
//! to call into `openpencil_shell_core::widgets`. Mirrors the
//! shell-web `widget_host.rs` so the editor-UI composition is
//! cross-platform: same widget code, same paint output.
//!
//! Layout matches `apps/web/src/components/editor/editor-layout.tsx`
//! — TopBar / LayerPanel / Toolbar (vertical floating) / Canvas
//! (fills) / RightPanel (only with selection) / StatusBar (floating
//! bottom-right) / AIChatPlaceholder (floating bottom-center).
//!
//! ### Mobile (iOS / Android) — Step 1f path
//!
//! Spec §11 — shell-native is desktop-gated (`backend` /
//! `widget_host` modules cfg-gated to `macos | linux | windows`).
//! Mobile widget rendering lands in Step 1f via `context::EaglProvider`
//! / `context::AndroidEglProvider`. Per the 2026-05-10 directive
//! ("安卓和ios 不需要 ipc / 本地 cli — 只需要 custom provider"):
//! mobile rendering is a custom-provider plugin point on the
//! `GlContextProvider` trait, not a separate IPC / CLI pipeline.

use crate::backend::NativeBackend;
use openpencil_shell_core::document::{ChatAnchor, Document, PropertyFocus};
use openpencil_shell_core::widgets::{
    AIChatHit, AIChatPlaceholder, CanvasViewport, LayerPanel, LayoutCx, LocalePicker, PaintCx,
    PropertyPanel, StatusBar, Toolbar, TopBar, TopBarHit, Widget, AI_CHAT_COLLAPSED_HEIGHT,
    AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_HEIGHT, AI_CHAT_WIDTH, LOCALE_PICKER_WIDTH, STATUS_BAR_HEIGHT,
    STATUS_BAR_WIDTH, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend, TextLayout, Theme};

const TOOLBAR_INSET_X: f32 = 12.0;
const TOOLBAR_INSET_Y: f32 = 12.0;
const STATUS_INSET: f32 = 16.0;

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}
/// Small breathing room from the canvas corner so the chat pill
/// doesn't visually touch the canvas edge (per 2026-05-10 user
/// note "稍微加一点上下偶有的间距，一点点").
const AICHAT_INSET_BOTTOM: f32 = 12.0;
const AICHAT_INSET_LEFT: f32 = 12.0;

/// Frame-scoped `RenderBackend` adapter over `NativeBackend` +
/// `&Canvas`. Lifetime-bound to the `SharedSkiaContext::with_frame`
/// closure body so widget code never sees the canvas borrow directly.
pub struct NativeFrameBackend<'a> {
    inner: &'a mut NativeBackend,
    canvas: &'a skia_safe::Canvas,
}

impl<'a> NativeFrameBackend<'a> {
    pub fn new(inner: &'a mut NativeBackend, canvas: &'a skia_safe::Canvas) -> Self {
        Self { inner, canvas }
    }
}

impl<'a> RenderBackend for NativeFrameBackend<'a> {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.inner.fill_rect(self.canvas, rect, color);
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        self.inner.stroke_rect(self.canvas, rect, color, width);
    }

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        self.inner.draw_text(self.canvas, layout, origin);
    }

    fn clip_rect(&mut self, rect: Rect) {
        self.inner.clip_rect(self.canvas, rect);
    }

    fn save(&mut self) {
        let _ = self.inner.save(self.canvas);
    }

    fn restore(&mut self) {
        self.inner.restore(self.canvas);
    }

    fn translate(&mut self, offset: Point2D) {
        self.inner.translate(self.canvas, offset);
    }

    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
        self.inner.stroke_line(self.canvas, from, to, color, width);
    }

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.inner.fill_round_rect(self.canvas, rect, radius, color);
    }

    fn stroke_round_rect(&mut self, rect: Rect, radius: f32, color: Color, width: f32) {
        self.inner
            .stroke_round_rect(self.canvas, rect, radius, color, width);
    }

    fn stroke_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, color: Color, width: f32) {
        self.inner
            .stroke_svg_path(self.canvas, d, top_left, size, color, width);
    }

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn dpi_scale(&self) -> f32 {
        self.inner.dpi_scale()
    }

    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        self.inner.measure_text(text, font_size)
    }
}

/// Native counterpart of shell-web's `WidgetHost`. Owns a
/// `Document` + composes the editor UI per frame in the
/// TS-equivalent layout (Step 4 visual lift).
pub struct WidgetHostNative {
    document: Document,
    theme: Theme,
    /// Active canvas pan-drag state — left-button press → motion
    /// → release.
    drag: Option<DragState>,
    /// Active chat-panel drag state — present while the user
    /// drags the floating AI chat panel by its header. Holds the
    /// transient panel top-left position so paint can place the
    /// panel at the cursor instead of its anchor; on release the
    /// host computes the nearest corner via `ChatAnchor::nearest`
    /// and snaps.
    chat_drag: Option<ChatDragState>,
    /// Active panel-resize drag — set when the cursor is pressed
    /// within the resize gutter of LayerPanel's right edge or
    /// PropertyPanel's left edge.
    panel_resize: Option<PanelResize>,
    /// Host-supplied frame timestamp in milliseconds. Drives the
    /// caret blink via `jian_core::anim::blink_visible`. The
    /// inspector_window runner refreshes this once per
    /// `RedrawRequested` from a single `Instant` start anchor;
    /// any other host (mobile / browser) installs its own clock.
    now_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct DragState {
    last_x: f32,
    last_y: f32,
}

/// Which panel edge is being dragged, plus the press anchor +
/// the panel width at press time. Live width is computed as
/// `start_width + (live_x - start_x) * sign`.
#[derive(Debug, Clone, Copy)]
struct PanelResize {
    kind: PanelResizeKind,
    start_x: f32,
    start_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelResizeKind {
    LayerRight,
    PropertyLeft,
}

/// Pixel half-thickness of the resize gutter on each panel edge —
/// click within this distance of the edge to begin a resize drag.
const PANEL_RESIZE_GUTTER: f32 = 4.0;
/// Hard floor / ceiling for resizable panels (TS app uses similar
/// limits — left/right rails can't shrink below ~180 or grow past
/// half the viewport).
const PANEL_MIN_WIDTH: f32 = 180.0;
const PANEL_MAX_WIDTH: f32 = 480.0;

#[derive(Debug, Clone, Copy)]
struct ChatDragState {
    /// Pointer offset within the panel rect when the drag began.
    /// Subtracting from the live cursor position gives the panel
    /// top-left, so the panel doesn't visually jump on press.
    grab_dx: f32,
    grab_dy: f32,
    /// Live panel top-left (logical px, viewport-relative).
    pos_x: f32,
    pos_y: f32,
}

impl WidgetHostNative {
    pub fn new() -> Self {
        Self {
            document: Document::sample(),
            theme: Theme::dark(),
            drag: None,
            chat_drag: None,
            panel_resize: None,
            now_ms: 0,
        }
    }

    /// Push the host's monotonic millisecond timestamp into the
    /// host. Drives caret blink + any future time-based
    /// animations via `jian_core::anim`.
    pub fn set_now_ms(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    /// Whether the chat input is focused — runner uses this to
    /// decide whether to schedule a periodic wake-up for caret
    /// blink.
    pub fn chat_focused(&self) -> bool {
        self.document.chat.focused
    }

    /// Next millisecond at which the host should wake to repaint
    /// the caret blink phase. `None` = no animation pending.
    pub fn next_animation_deadline_ms(&self) -> Option<u64> {
        if self.document.ui.property_focus.is_some() {
            return Some(jian_core::anim::next_blink_flip_ms(
                self.now_ms,
                self.document.ui.property_caret_anchor_ms,
                500,
            ));
        }
        if self.document.chat.focused {
            return Some(jian_core::anim::next_blink_flip_ms(
                self.now_ms,
                self.document.chat.caret_anchor_ms,
                500,
            ));
        }
        None
    }

    /// Hit-test which screen region the cursor is over. Used by
    /// the wheel + drag handlers so wheel-zoom + Hand-pan only
    /// fire when the cursor is over the canvas (not over a panel).
    /// Uses `canvas_region` so it stays in sync with paint when
    /// the sidebar is collapsed (codex Step 6 stop-hook fix:
    /// "native collapsed-sidebar canvas input still uses the old
    /// left offset").
    fn over_canvas(&self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        x >= cx0 && x <= cx0 + cw && y >= cy0 && y <= cy0 + ch
    }

    /// Apply a wheel event — zoom centered at `(x, y)` when over
    /// the canvas. Returns true if a redraw is needed.
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
        // Cursor in canvas-local coords — use canvas_region's
        // dynamic left edge so cursor-centered zoom stays anchored
        // when the sidebar is collapsed.
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let cursor = Point2D::new(x - cx0, y - cy0);
        self.document.viewport.zoom_at(cursor, delta_y);
        true
    }

    /// Apply a 2-finger trackpad pan gesture — translate the
    /// canvas viewport by `(dx, dy)` directly. Step 5 makes
    /// trackpad swipes feel native (Figma convention: 2-finger
    /// swipe pans, pinch / Cmd+swipe / mouse-wheel zoom). Returns
    /// true if a redraw is needed.
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

    /// Mouse-press handler. Hit-test order:
    ///   0. TopBar (panel-left toggles sidebar, etc.)
    ///   1. AI chat panel — DragHandle starts a chat drag,
    ///      everything else defers to apply_click
    ///   2. Toolbar (gaps + buttons consume clicks)
    ///   3. apply_click — LayerPanel + chat-defocus
    ///   4. Otherwise: clear selection (collapses RightPanel) +
    ///      start canvas pan-drag.
    /// Returns whether anything visible changed.
    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // 0aa. Commit-on-blur for property-panel inputs — if a
        //      click lands outside the property panel while an
        //      input is focused, commit the draft (parse + write to
        //      node) before processing the new click. The
        //      property-panel hit-test below replaces it with the
        //      new focus when the click was inside the panel.
        if self.document.ui.property_focus.is_some() {
            let property_left = if self.document.selected_node().is_some() {
                viewport_width - self.document.ui.property_panel_width
            } else {
                viewport_width
            };
            if x < property_left {
                self.commit_property_focus_if_any();
            }
        }

        // 0z. Panel-resize gutter — clicks within ±4 px of the
        //     LayerPanel right edge or PropertyPanel left edge
        //     start a resize drag. Below TopBar so the gutter
        //     doesn't eat title-bar clicks.
        if y >= TOP_BAR_HEIGHT {
            if let Some(kind) = self.panel_resize_hover(x, y, viewport_width) {
                let start_width = match kind {
                    PanelResizeKind::LayerRight => self.document.ui.layer_panel_width,
                    PanelResizeKind::PropertyLeft => self.document.ui.property_panel_width,
                };
                self.panel_resize = Some(PanelResize {
                    kind,
                    start_x: x,
                    start_width,
                });
                return true;
            }
        }

        // 0a. Locale picker overlay — when open, it sits on top of
        //     everything. Row click sets locale + closes; ANY
        //     other click (including the Globe button itself, the
        //     canvas, the toolbar, the chip) just closes the
        //     picker. The click is swallowed so the same press
        //     doesn't simultaneously re-toggle the picker open
        //     via the Globe button hit-test below.
        if self.document.ui.locale_picker_open {
            let panel_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_document(&self.document);
            if let Some(locale) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
                self.document.ui.locale = locale;
                self.document.ui.locale_picker_open = false;
                return true;
            }
            self.document.ui.locale_picker_open = false;
            return true;
        }

        // 0b. TopBar — sidebar toggle button + theme + locale picker.
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        let top_bar = TopBar::for_document(&self.document);
        if let Some(hit) = top_bar.hit_test(top_bar_rect, Point2D::new(x, y)) {
            match hit {
                TopBarHit::ToggleSidebar => {
                    self.document.ui.sidebar_open = !self.document.ui.sidebar_open;
                    return true;
                }
                TopBarHit::ToggleTheme => {
                    self.document.ui.theme_mode = self.document.ui.theme_mode.flipped();
                    return true;
                }
                TopBarHit::ToggleLocale => {
                    self.document.ui.locale_picker_open = !self.document.ui.locale_picker_open;
                    return true;
                }
            }
        }
        if rect_contains(top_bar_rect, Point2D::new(x, y)) {
            // Other top-bar gaps eat clicks but don't act.
            return false;
        }

        // 0c. PropertyPanel input row — focus the row + seed the
        //     edit draft from the snapshot value. Any other click
        //     (canvas, chat, toolbar, layer panel) commits + clears
        //     the focused input via the catch-all branches below.
        if let Some(panel) = PropertyPanel::for_selection(&self.document) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.document.ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.document.ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            if let Some(focus) = panel.hit_test(property_rect, Point2D::new(x, y)) {
                self.commit_property_focus_if_any();
                let initial = match focus {
                    openpencil_shell_core::document::PropertyFocus::PositionX => {
                        panel.snapshot.x.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::PositionY => {
                        panel.snapshot.y.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::SizeW => {
                        panel.snapshot.width.to_string()
                    }
                    openpencil_shell_core::document::PropertyFocus::SizeH => {
                        panel.snapshot.height.to_string()
                    }
                    _ => String::new(),
                };
                self.document.ui.property_focus = Some(focus);
                self.document.ui.property_input_draft = initial;
                self.document.ui.property_caret_anchor_ms = self.now_ms;
                self.document.chat.focused = false;
                return true;
            }
        }

        // 1. AI chat panel — sits on top of the toolbar in paint
        //    order, so any click inside its rect is consumed
        //    here. DragHandle starts a chat drag; other AI hits
        //    defer to apply_click for focus/send/example/toggle.
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
                // Non-drag chat hit: route through apply_click +
                // short-circuit so we never fall through to the
                // toolbar (which is below the chat panel).
                let _ = self.apply_click(x, y, viewport_width, viewport_height);
                return true;
            }
        }

        // 2. Toolbar — second-highest overlay. A click anywhere
        //    inside its bounding rect is consumed (gaps + padding
        //    too) so it never falls through to the canvas. The
        //    toolbar's x anchor follows `canvas_region`, so when
        //    the sidebar is collapsed it shifts left along with
        //    the canvas (codex stop-hook fix: "collapsed-sidebar
        //    interactions break").
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let toolbar = Toolbar::for_document(&self.document);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
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
        let consumed = self.apply_click(x, y, viewport_width, viewport_height);
        if consumed {
            return true;
        }

        // 4. Empty-canvas click: clear selection (collapses the
        //    PropertyPanel) + start a pan-drag. Selection clear
        //    is the "click blank to deselect" UX the user
        //    requested.
        if self.over_canvas(x, y, viewport_width, viewport_height) {
            let cleared = self.document.selected != openpencil_shell_core::document::NodeId::NONE;
            if cleared {
                self.document.selected = openpencil_shell_core::document::NodeId::NONE;
            }
            self.drag = Some(DragState {
                last_x: x,
                last_y: y,
            });
            return cleared;
        }
        false
    }

    /// Cursor-move handler. Drives canvas pan-drag, chat-panel
    /// drag, or no-op. Returns whether the host should repaint.
    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        if let Some(resize) = self.panel_resize {
            let dx = x - resize.start_x;
            match resize.kind {
                PanelResizeKind::LayerRight => {
                    let new_w = (resize.start_width + dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
                    self.document.ui.layer_panel_width = new_w;
                }
                PanelResizeKind::PropertyLeft => {
                    let new_w = (resize.start_width - dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
                    self.document.ui.property_panel_width = new_w;
                }
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

    /// Mouse-release handler. Ends the active drag (if any). For
    /// chat-panel drag, snaps the panel to the nearest canvas
    /// corner via `ChatAnchor::nearest`. Returns true if anything
    /// visible changed.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        if self.panel_resize.take().is_some() {
            return true;
        }
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

    /// Mouse-release handler — viewport-less variant kept for
    /// backwards compatibility with existing call sites.
    pub fn apply_release(&mut self) -> bool {
        if self.panel_resize.take().is_some() {
            return true;
        }
        // If a chat drag was in flight without a known viewport,
        // we can't snap; just drop it (best effort).
        if self.chat_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging
    }

    /// Push a typed character into the focused chat input.
    /// Returns true if anything changed.
    pub fn apply_text(&mut self, c: char) -> bool {
        if let Some(focus) = self.document.ui.property_focus {
            // Only accept characters that fit a number input.
            // Position / size only need digits + a leading minus.
            let allowed = c.is_ascii_digit()
                || (c == '-' && self.document.ui.property_input_draft.is_empty())
                || (c == '.'
                    && matches!(
                        focus,
                        PropertyFocus::Opacity | PropertyFocus::Rotation | PropertyFocus::PositionR
                    )
                    && !self.document.ui.property_input_draft.contains('.'));
            if !allowed {
                return false;
            }
            self.document.ui.property_input_draft.push(c);
            self.document.ui.property_caret_anchor_ms = self.now_ms;
            return true;
        }
        if !self.document.chat.focused {
            return false;
        }
        self.document.chat.input.push(c);
        // Reset blink so the caret is solid right after the
        // keystroke instead of mid-fade.
        self.document.chat.caret_anchor_ms = self.now_ms;
        true
    }

    /// Backspace — routes to whichever input is currently focused
    /// (property edit field if any, else chat).
    pub fn apply_backspace(&mut self) -> bool {
        if self.document.ui.property_focus.is_some() {
            if self.document.ui.property_input_draft.pop().is_some() {
                self.document.ui.property_caret_anchor_ms = self.now_ms;
                return true;
            }
            return false;
        }
        if !self.document.chat.focused {
            return false;
        }
        if self.document.chat.input.pop().is_some() {
            self.document.chat.caret_anchor_ms = self.now_ms;
            return true;
        }
        false
    }

    /// Enter — commits the focused property edit (parses the draft
    /// as f32, writes to the selected node, clears focus) or
    /// sends the focused chat input.
    pub fn apply_send(&mut self) -> bool {
        if self.document.ui.property_focus.is_some() {
            self.commit_property_focus_if_any();
            return true;
        }
        if self.document.chat.input.trim().is_empty() {
            return false;
        }
        self.document.chat.send();
        true
    }

    /// Escape — drops the focused property edit without committing.
    pub fn apply_escape(&mut self) -> bool {
        if self.document.ui.property_focus.take().is_some() {
            self.document.ui.property_input_draft.clear();
            return true;
        }
        false
    }

    /// Parse `property_input_draft` as f32 and apply it to the
    /// selected node via `Document::commit_property_edit`. Always
    /// clears focus + draft. No-op when nothing is focused.
    fn commit_property_focus_if_any(&mut self) {
        let Some(focus) = self.document.ui.property_focus.take() else {
            return;
        };
        let draft = std::mem::take(&mut self.document.ui.property_input_draft);
        if let Ok(value) = draft.trim().parse::<f32>() {
            let _ = self.document.commit_property_edit(focus, value);
        }
    }

    /// Canvas region (logical px, viewport-relative). Reflects
    /// the LayerPanel sidebar collapse state — when sidebar is
    /// hidden the canvas stretches to the left edge.
    /// True when the cursor is over either resize gutter — used by
    /// the runner to set `CursorIcon::EwResize`. None = no gutter.
    pub fn panel_resize_hover(&self, x: f32, y: f32, viewport_w: f32) -> Option<PanelResizeKind> {
        if y < TOP_BAR_HEIGHT {
            return None;
        }
        if self.document.ui.sidebar_open {
            let edge = self.document.ui.layer_panel_width;
            if (x - edge).abs() <= PANEL_RESIZE_GUTTER {
                return Some(PanelResizeKind::LayerRight);
            }
        }
        if self.document.selected_node().is_some() {
            let edge = viewport_w - self.document.ui.property_panel_width;
            if (x - edge).abs() <= PANEL_RESIZE_GUTTER {
                return Some(PanelResizeKind::PropertyLeft);
            }
        }
        None
    }

    /// Whether a panel resize is in progress. Runner uses this to
    /// keep the resize cursor active even when the cursor briefly
    /// leaves the gutter mid-drag.
    pub fn is_resizing_panel(&self) -> bool {
        self.panel_resize.is_some()
    }

    fn canvas_region(&self, viewport_w: f32, viewport_h: f32) -> (f32, f32, f32, f32) {
        let canvas_left = if self.document.ui.sidebar_open {
            self.document.ui.layer_panel_width
        } else {
            0.0
        };
        let has_property = self.document.selected_node().is_some();
        let canvas_right = if has_property {
            viewport_w - self.document.ui.property_panel_width
        } else {
            viewport_w
        };
        let canvas_w = (canvas_right - canvas_left).max(0.0);
        let canvas_h = (viewport_h - TOP_BAR_HEIGHT).max(0.0);
        (canvas_left, TOP_BAR_HEIGHT, canvas_w, canvas_h)
    }

    fn locale_picker_rect(&self, viewport_w: f32) -> Rect {
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        };
        let globe = TopBar::globe_rect(top_bar_rect);
        let panel_h = LocalePicker::panel_height();
        // Anchor under the globe icon, right-aligned to its center
        // so the panel doesn't run off the right edge.
        let x = (globe.origin.x + globe.size.x / 2.0 - LOCALE_PICKER_WIDTH / 2.0)
            .max(8.0)
            .min(viewport_w - LOCALE_PICKER_WIDTH - 8.0);
        let y = globe.origin.y + globe.size.y + 6.0;
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(LOCALE_PICKER_WIDTH, panel_h),
        }
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

    /// Apply a primary-button mouse click — routes to Toolbar /
    /// LayerPanel / AI chat hit-test. Returns whether anything was
    /// consumed (caller should request a redraw if so).
    pub fn apply_click(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // AI chat panel sits ABOVE the canvas — check it first so
        // clicks on the floating panel don't fall through to the
        // canvas / Hand-tool drag.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel = AIChatPlaceholder::from_document(&self.document);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                match hit {
                    AIChatHit::FocusInput => {
                        self.document.chat.focused = true;
                        self.document.chat.caret_anchor_ms = self.now_ms;
                        return true;
                    }
                    AIChatHit::Send => {
                        self.document.chat.send();
                        return true;
                    }
                    AIChatHit::Example(text) => {
                        self.document.chat.input = text;
                        self.document.chat.focused = true;
                        self.document.chat.caret_anchor_ms = self.now_ms;
                        return true;
                    }
                    AIChatHit::DragHandle => {
                        // Drag handle is handled in apply_press
                        // ahead of this; reaching here is a path
                        // bypass — ignore.
                        return false;
                    }
                    AIChatHit::ToggleCollapse => {
                        self.document.chat.collapsed = !self.document.chat.collapsed;
                        return true;
                    }
                }
            }
        }
        // Click outside chat panel — defocus the input.
        let was_focused = self.document.chat.focused;
        self.document.chat.focused = false;
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let toolbar = Toolbar::for_document(&self.document);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
        if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                    self.document.tool = tool;
                    return true;
                }
                openpencil_shell_core::widgets::ToolbarHit::Action(_) => return false,
            }
        }
        // LayerPanel hits only land when the sidebar is open —
        // when collapsed the panel isn't painted (codex stop-hook
        // fix: native collapsed-sidebar input was still resolving
        // canvas clicks to the LayerPanel rect underneath).
        if !self.document.ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.document.ui.layer_panel_width,
                (viewport_height - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        let panel = LayerPanel::from_document(&self.document);
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::LayerPanelHit::Page(idx) => {
                    self.document.active_page_index = idx;
                    self.document.selected = openpencil_shell_core::document::NodeId::NONE;
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::Layer(node_id) => {
                    self.document.selected = node_id;
                    return true;
                }
            }
        }
        // Click hit no chrome — return true if the prior focus
        // state changed so the chrome repaints to drop the caret.
        was_focused
    }

    /// Paint the editor-UI composition.
    pub fn paint(
        &self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // glue:
        // 1. Background fill so previous-frame pixels never bleed.
        frame.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, viewport_height),
            },
            self.theme.background,
        );

        let dpi = frame.dpi_scale();

        // 2. TopBar.
        let top_bar = TopBar::for_document(&self.document);
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            top_bar.paint(&mut cx, top_bar_rect);
        }

        // 3. LayerPanel — skipped when the sidebar is collapsed.
        if self.document.ui.sidebar_open {
            let layer_panel = LayerPanel::from_document(&self.document);
            let layer_panel_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    self.document.ui.layer_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            layer_panel.paint(&mut cx, layer_panel_rect);
        }

        // 4. PropertyPanel — only when selection.
        let property_panel = PropertyPanel::for_selection_at(&self.document, self.now_ms);
        let has_property = property_panel.is_some();
        if let Some(panel) = property_panel.as_ref() {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.document.ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.document.ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            panel.paint(&mut cx, property_rect);
        }

        // 5. CanvasViewport — middle band, respects sidebar
        //    collapse state.
        let (canvas_left, _canvas_y, canvas_w, canvas_h) =
            self.canvas_region(viewport_width, viewport_height);
        let _ = has_property;
        let canvas_rect = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        if canvas_w > 0.0 && canvas_h > 0.0 {
            let canvas = CanvasViewport::from_document(&self.document);
            let mut cx = PaintCx {
                backend: &mut *frame,
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
                backend: &mut *frame,
            };
            toolbar.paint(&mut cx, toolbar_rect);
        }

        // 7. AIChatPlaceholder — painted LAST so it sits on top
        //    of the toolbar in any overlap region (matches the
        //    user's requested z-order: chat above toolbar).
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let chat = AIChatPlaceholder::from_document_at(&self.document, self.now_ms);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            chat.paint(&mut cx, chat_rect);
        }

        // 8. StatusBar — floating bottom-right.
        let canvas_right = canvas_left + canvas_w;
        if canvas_w > STATUS_BAR_WIDTH + STATUS_INSET * 2.0 {
            let status = StatusBar::for_document(&self.document);
            let status_rect = Rect {
                origin: Point2D::new(
                    canvas_right - STATUS_BAR_WIDTH - STATUS_INSET,
                    TOP_BAR_HEIGHT + canvas_h - STATUS_BAR_HEIGHT - STATUS_INSET,
                ),
                size: Point2D::new(STATUS_BAR_WIDTH, STATUS_BAR_HEIGHT),
            };
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            status.paint(&mut cx, status_rect);
        }

        // 9. LocalePicker — top-most overlay so it covers chat /
        //    toolbar / status when open.
        if self.document.ui.locale_picker_open {
            let picker_rect = self.locale_picker_rect(viewport_width);
            let picker = LocalePicker::for_document(&self.document);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            picker.paint(&mut cx, picker_rect);
        }
    }
}

impl Default for WidgetHostNative {
    fn default() -> Self {
        Self::new()
    }
}
