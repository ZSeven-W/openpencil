//! Device-frame preview host glue for the web shell.
//!
//! Mirrors the native host's `preview_frame.rs` / `screen_switcher.rs`,
//! but solves NO geometry of its own: the frame fit/centring, pinned
//! bottom-nav and top-status-bar detection, scroll extent, and the
//! screen <-> scene transforms all come from
//! `op_preview_core::device_frame`, the same module the native host
//! uses. Preview's runtime layout only serves hit-testing, so the two
//! hosts must agree on this math exactly or taps stop landing where
//! preview paints.
//!
//! Not ported yet (deliberately): the canvas <-> frame merge animation
//! (`ModeTransition`), slideshow presentation, and edge-swipe back.

use op_editor_core::PreviewDeviceKind;
use op_editor_ui::widgets::{PaintCx, PreviewDeviceSwitcher, ScreenSwitcherPills};
use op_editor_ui::{Point2D, Rect};

use op_preview_core::device_frame::{
    compute_frame_geometry, device_scene_point, device_surface_at, frame_radius,
    infer_kind_for_width, paint_corner_notches, scroll_max, PinnedGeom,
};
use op_preview_core::PinnedPaint;

impl super::WidgetHost {
    /// Canvas region preview presents into. While presenting a deck slideshow,
    /// returns the full viewport. Otherwise returns the device-frame canvas rect.
    pub(in crate::widget_host) fn preview_canvas_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Rect {
        // A slideshow presentation takes the full stage (both on native and web)
        // rather than being constrained to the canvas region.
        if self.preview_slideshow_active() {
            return Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_w, viewport_h),
            };
        }
        let (canvas_x, canvas_y, canvas_w, canvas_h) = self.canvas_region(viewport_w, viewport_h);
        Rect {
            origin: Point2D::new(canvas_x, canvas_y),
            size: Point2D::new(canvas_w, canvas_h),
        }
    }

    /// Whether preview is currently presented through a device frame
    /// (as opposed to the Canvas segment, which paints the raw scene at
    /// the editor's own pan/zoom).
    pub(in crate::widget_host) fn device_mode_active(&self) -> bool {
        self.preview.is_some()
            && matches!(
                self.editor_state.editor_ui.preview.device,
                Some(PreviewDeviceKind::Phone) | Some(PreviewDeviceKind::Desktop)
            )
    }

    /// Resolve the manual switcher pick, or infer from the framed root's
    /// width (<= 500 logical px reads as a phone).
    pub(in crate::widget_host) fn infer_device_kind(&self) -> PreviewDeviceKind {
        if let Some(pick) = self.preview_manual_pick {
            return pick;
        }
        let width = self
            .preview
            .as_ref()
            .and_then(|preview| preview.framed_root())
            .map(|(_, rect)| rect.size.x);
        infer_kind_for_width(width)
    }

    /// Pick the entry device and build its frame. Called once on enter.
    pub(in crate::widget_host) fn initialize_device_preview(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let kind = self.infer_device_kind();
        self.editor_state.editor_ui.preview.device = Some(kind);
        self.preview_scroll_y = 0.0;
        self.recompute_device_frame(viewport_w, viewport_h);
    }

    pub(in crate::widget_host) fn clear_device_preview_state(&mut self) {
        self.preview_device_frame = None;
        self.preview_scroll_y = 0.0;
        self.preview_manual_pick = None;
        self.preview_surface_capture = None;
        self.preview_edge_swipe_start_x = None;
        self.slideshow_cursor = None;
        self.slideshow_press_screen = None;
        self.editor_state.editor_ui.preview.device = None;
        self.editor_state.editor_ui.preview.screen_switcher_hover = None;
        self.editor_state.editor_ui.preview.screen_switcher_pressed = None;
    }

    /// Rebuild the cached frame from the current session and viewport.
    /// Re-run on every resize and screen switch — the frame is derived
    /// state, never edited in place.
    pub(in crate::widget_host) fn recompute_device_frame(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        if !self.device_mode_active() {
            self.preview_device_frame = None;
            return;
        }
        let Some(kind) = self.editor_state.editor_ui.preview.device else {
            self.preview_device_frame = None;
            return;
        };
        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        if canvas.size.x <= 0.0 || canvas.size.y <= 0.0 {
            self.preview_device_frame = None;
            return;
        }
        let Some(session) = self.preview.as_ref() else {
            self.preview_device_frame = None;
            return;
        };
        let Some((_root_id, root_rect)) = session.framed_root() else {
            // No framed root yet: keep an empty silhouette so the bezel
            // still paints rather than flashing raw canvas.
            self.preview_device_frame = Some(compute_frame_geometry(
                kind,
                canvas,
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(0.0, 0.0),
                },
                None,
                None,
            ));
            self.preview_scroll_y = 0.0;
            return;
        };
        let is_phone = kind == PreviewDeviceKind::Phone;
        let nav = session.pinned_nav_candidate(is_phone);
        let status = session.pinned_status_bar_candidate(is_phone);
        let mut frame = compute_frame_geometry(
            kind,
            canvas,
            root_rect,
            nav.as_ref().map(|(_, rect)| *rect),
            status.as_ref().map(|(_, rect)| *rect),
        );
        if let (Some(pinned), Some((node_id, _))) = (frame.pinned.as_mut(), nav) {
            pinned.node_id = node_id;
        }
        if let (Some(pinned_top), Some((node_id, _))) = (frame.pinned_top.as_mut(), status) {
            pinned_top.node_id = node_id;
        }
        self.preview_scroll_y = self.preview_scroll_y.clamp(0.0, scroll_max(&frame));
        self.preview_device_frame = Some(frame);
    }

    /// Screen -> scene through the device frame. Honours a gesture's
    /// captured surface so a drag that started on the scrolled content
    /// does not remap through the pinned nav when it crosses into it.
    pub(in crate::widget_host) fn device_preview_doc_point(
        &self,
        screen_x: f32,
        screen_y: f32,
    ) -> Option<Point2D> {
        let frame = self.preview_device_frame.as_ref()?;
        let screen = Point2D::new(screen_x, screen_y);
        let surface = match self.preview_surface_capture {
            Some(surface) => surface,
            None => device_surface_at(frame, screen, self.preview_scroll_y)?,
        };
        device_scene_point(frame, &surface, screen)
    }

    pub(in crate::widget_host) fn capture_device_preview_surface(
        &mut self,
        screen_x: f32,
        screen_y: f32,
    ) {
        if let Some(frame) = self.preview_device_frame.as_ref() {
            self.preview_surface_capture = device_surface_at(
                frame,
                Point2D::new(screen_x, screen_y),
                self.preview_scroll_y,
            );
        }
    }

    /// Apply a screen-pixel wheel delta to the framed content's logical
    /// scroll offset.
    pub(in crate::widget_host) fn apply_device_scroll(&mut self, screen_delta_y: f32) -> bool {
        let Some(frame) = self.preview_device_frame.as_ref() else {
            return false;
        };
        let next =
            (self.preview_scroll_y - screen_delta_y / frame.fit).clamp(0.0, scroll_max(frame));
        if (next - self.preview_scroll_y).abs() > f32::EPSILON {
            self.preview_scroll_y = next;
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Activate one switcher segment and rebuild its presentation.
    pub(in crate::widget_host) fn set_preview_device(
        &mut self,
        kind: PreviewDeviceKind,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        self.preview_manual_pick = Some(kind);
        self.editor_state.editor_ui.preview.device = Some(kind);
        self.preview_scroll_y = 0.0;
        self.preview_surface_capture = None;
        if kind == PreviewDeviceKind::Canvas {
            self.preview_device_frame = None;
            self.center_canvas_on_preview_root(viewport_w, viewport_h);
        } else {
            self.recompute_device_frame(viewport_w, viewport_h);
        }
        self.mark_dirty();
    }

    /// Re-infer after an app-mode screen switch and reset gesture state.
    pub(in crate::widget_host) fn on_preview_screen_switched(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        if self.preview_manual_pick.is_none() {
            let kind = self.infer_device_kind();
            self.editor_state.editor_ui.preview.device = Some(kind);
        }
        self.preview_scroll_y = 0.0;
        self.preview_surface_capture = None;
        self.recompute_device_frame(viewport_w, viewport_h);
    }

    /// Centre the editor viewport on the preview root — the Canvas
    /// segment's equivalent of fitting a device frame.
    pub(in crate::widget_host) fn center_canvas_on_preview_root(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let Some((_id, rect)) = self.preview.as_ref().and_then(|p| p.framed_root()) else {
            return;
        };
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let vp = &mut self.editor_state.viewport;
        vp.pan_x = cw / 2.0 - (rect.origin.x + rect.size.x / 2.0) * vp.zoom;
        vp.pan_y = ch / 2.0 - (rect.origin.y + rect.size.y / 2.0) * vp.zoom;
        self.mark_dirty();
    }

    /// Map a framed-root DOC-space rect through the canvas's current
    /// pan/zoom into SCREEN space. Used by Track M-1 merge animation
    /// to capture the source rect at enter time.
    pub(in crate::widget_host) fn framed_root_to_screen_rect(
        &self,
        rect: Rect,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Rect {
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let vp = &self.editor_state.viewport;
        Rect {
            origin: Point2D::new(
                cx0 + vp.pan_x + rect.origin.x * vp.zoom,
                cy0 + vp.pan_y + rect.origin.y * vp.zoom,
            ),
            size: Point2D::new(rect.size.x * vp.zoom, rect.size.y * vp.zoom),
        }
    }

    /// Paint the framed content plus its rounded silhouette.
    ///
    /// Track M-1: while a canvas ↔ device-frame merge animation is
    /// active, the geometry used for this paint is NOT the settled
    /// `self.preview_device_frame` — it is re-derived against the
    /// transition's interpolated rect, so the silhouette + content
    /// visibly move/scale between the screen's canvas position and
    /// the settled frame. The bezel/border colours blend toward the
    /// canvas backdrop via `chrome_blend`.
    pub(in crate::widget_host) fn paint_device_frame(
        &self,
        backend: &mut dyn op_editor_ui::RenderBackend,
        canvas_rect: Rect,
    ) {
        let Some(session) = self.preview.as_ref() else {
            return;
        };
        let Some(steady_frame) = self.preview_device_frame.as_ref() else {
            return;
        };

        #[cfg(feature = "canvaskit")]
        let active_mode_transition = self
            .preview_mode_transition
            .as_ref()
            .filter(|t| t.is_active(self.now_ms));
        #[cfg(not(feature = "canvaskit"))]
        let active_mode_transition: Option<&op_preview_core::ModeTransition> = None;

        let interpolated;
        let (device_frame, chrome_blend): (&op_preview_core::device_frame::DeviceFrame, f32) =
            match active_mode_transition {
                Some(transition) => {
                    let is_phone = steady_frame.kind == op_editor_core::PreviewDeviceKind::Phone;
                    let nav = session.pinned_nav_candidate(is_phone);
                    let status = session.pinned_status_bar_candidate(is_phone);
                    let root_rect = session
                        .framed_root()
                        .map(|(_, rect)| rect)
                        .unwrap_or(steady_frame.frame);
                    let mut frame = compute_frame_geometry(
                        steady_frame.kind,
                        transition.canvas_rect_for_frame(self.now_ms),
                        root_rect,
                        nav.as_ref().map(|(_, rect)| *rect),
                        status.as_ref().map(|(_, rect)| *rect),
                    );
                    if let (Some(pinned), Some((node_id, _))) = (frame.pinned.as_mut(), nav) {
                        pinned.node_id = node_id;
                    }
                    if let (Some(pinned_top), Some((node_id, _))) =
                        (frame.pinned_top.as_mut(), status)
                    {
                        pinned_top.node_id = node_id;
                    }
                    interpolated = frame;
                    (&interpolated, transition.chrome_blend(self.now_ms))
                }
                None => (steady_frame, 1.0),
            };

        let radius = frame_radius(device_frame.kind) * device_frame.fit;
        // Line the bezel with the screen's OWN background (falling back
        // to the host's canvas-surface tone) rather than a hardcoded
        // white: the device silhouette is a fixed size that doesn't
        // always match the screen's authored width, so a narrower design
        // shows a thin strip of bezel on each side — this keeps that
        // strip blending with the design instead of reading as a stray seam.
        let settled_bezel_fill = session
            .framed_root_fill()
            .unwrap_or(self.theme.canvas_surface);
        // Blend toward the ALREADY-PAINTED canvas backdrop
        // (`theme.canvas_surface`, filled by the caller before this
        // runs) at the un-settled end of a Track M-1 merge — equivalent
        // to true alpha blending since the backdrop underneath is a
        // known solid colour.
        let bezel_fill = op_preview_core::lerp_color(
            self.theme.canvas_surface,
            settled_bezel_fill,
            chrome_blend,
        );
        backend.fill_round_rect(device_frame.frame, radius, bezel_fill);

        if let Some((root_id, _)) = session.framed_root() {
            let top_inset = device_frame
                .pinned_top
                .as_ref()
                .map_or(0.0, |status| status.strip.size.y);
            let bottom_inset = device_frame
                .pinned
                .as_ref()
                .map_or(0.0, |pinned| pinned.strip.size.y);
            let content_clip = Rect {
                origin: Point2D::new(
                    device_frame.frame.origin.x,
                    device_frame.frame.origin.y + top_inset,
                ),
                size: Point2D::new(
                    device_frame.frame.size.x,
                    device_frame.frame.size.y - top_inset - bottom_inset,
                ),
            };
            let content_origin = Point2D::new(
                device_frame.content_origin.x,
                device_frame.content_origin.y - self.preview_scroll_y * device_frame.fit,
            );
            let to_pinned_paint = |pinned: &PinnedGeom| PinnedPaint {
                node_id: pinned.node_id.clone(),
                strip_clip: pinned.strip,
                paint_origin: pinned.paint_origin,
                nav_scene_origin: pinned.node_scene.origin,
            };
            let pinned_paint = device_frame.pinned.as_ref().map(to_pinned_paint);
            let pinned_top_paint = device_frame.pinned_top.as_ref().map(to_pinned_paint);
            // Route to the plain single-layer `paint_framed` when idle, or
            // composites the outgoing/entering screens for an in-flight
            // push/pop/replace transition.
            session.paint_framed_animated(
                backend,
                &root_id,
                content_clip,
                content_origin,
                device_frame.fit,
                pinned_paint.as_ref(),
                pinned_top_paint.as_ref(),
                self.now_ms,
            );
        }

        // The outward ring masks square clip bleed without covering the
        // rounded interior; clip it so it cannot touch the TopBar.
        backend.save();
        backend.clip_rect(canvas_rect);
        paint_corner_notches(
            backend,
            device_frame.frame,
            radius,
            self.theme.canvas_surface,
        );
        let border_color =
            op_preview_core::lerp_color(self.theme.canvas_surface, self.theme.border, chrome_blend);
        backend.stroke_round_rect(device_frame.frame, radius, border_color, 1.0);
        backend.restore();
    }

    /// Segment labels, localized through the same keys the native host
    /// uses so both shells read identically.
    fn preview_switcher_labels(&self) -> [&'static str; 3] {
        let locale = self.editor_state.editor_ui.effective_locale();
        [
            op_i18n::translate(locale, "preview.device.phone"),
            op_i18n::translate(locale, "preview.device.desktop"),
            op_i18n::translate(locale, "preview.device.canvas"),
        ]
    }

    /// Phone / Desktop / Canvas segmented control, painted in every
    /// preview mode.
    pub(in crate::widget_host) fn paint_preview_switcher(
        &self,
        backend: &mut dyn op_editor_ui::RenderBackend,
        canvas_rect: Rect,
    ) {
        if self.preview.is_none() {
            return;
        }
        let labels = self.preview_switcher_labels();
        let switcher = PreviewDeviceSwitcher {
            labels,
            selected: self.editor_state.editor_ui.preview.device,
            hover: self.editor_state.editor_ui.preview.switcher_hover,
            pressed: self.editor_state.editor_ui.preview.switcher_pressed,
        };
        let mut cx = PaintCx { backend };
        switcher.paint(&mut cx, canvas_rect, &self.theme);
    }

    /// App-mode screen pills (one per routed screen).
    pub(in crate::widget_host) fn paint_screen_switcher(
        &self,
        backend: &mut dyn op_editor_ui::RenderBackend,
        canvas_rect: Rect,
    ) {
        let Some(session) = self.preview.as_ref() else {
            return;
        };
        if !session.is_app_mode() {
            return;
        }
        let labels: Vec<String> = session
            .screen_switcher_entries()
            .into_iter()
            .map(|(_, label)| label)
            .collect();
        let switcher = ScreenSwitcherPills {
            labels: &labels,
            active: session.current_screen_index(),
            hover: self.editor_state.editor_ui.preview.screen_switcher_hover,
        };
        let mut cx = PaintCx { backend };
        switcher.paint(&mut cx, canvas_rect, &self.theme);
    }

    /// Press on the device switcher — arms a segment for release.
    pub(in crate::widget_host) fn preview_switcher_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if self.preview.is_none() {
            return false;
        }
        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        let hit = PreviewDeviceSwitcher::hit_test(canvas, Point2D::new(x, y));
        self.editor_state.editor_ui.preview.switcher_pressed = hit;
        hit.is_some()
    }

    /// Release — activates only when the pointer is still over the armed
    /// segment, mirroring the native host.
    pub(in crate::widget_host) fn preview_switcher_release(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let pressed = self.editor_state.editor_ui.preview.switcher_pressed.take();
        let Some(pressed) = pressed else {
            return false;
        };
        if self.editor_state.editor_ui.preview.switcher_hover == Some(pressed) {
            self.set_preview_device(pressed, viewport_w, viewport_h);
        }
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn preview_switcher_hover(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        if self.preview.is_none() {
            return;
        }
        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        let hover = PreviewDeviceSwitcher::hit_test(canvas, Point2D::new(x, y));
        if self.editor_state.editor_ui.preview.switcher_hover != hover {
            self.editor_state.editor_ui.preview.switcher_hover = hover;
            self.mark_dirty();
        }
    }

    pub(in crate::widget_host) fn screen_switcher_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let Some(session) = self.preview.as_ref() else {
            return false;
        };
        if !session.is_app_mode() {
            return false;
        }
        let count = session.screen_switcher_entries().len();
        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        let hit = ScreenSwitcherPills::hit_test(canvas, count, Point2D::new(x, y));
        self.editor_state.editor_ui.preview.screen_switcher_pressed = hit;
        hit.is_some()
    }

    pub(in crate::widget_host) fn screen_switcher_release(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let pressed = self
            .editor_state
            .editor_ui
            .preview
            .screen_switcher_pressed
            .take();
        let Some(pressed) = pressed else {
            return false;
        };
        if self.editor_state.editor_ui.preview.screen_switcher_hover == Some(pressed) {
            if let Some(session) = self.preview.as_ref() {
                if let Some((path, _)) = session.screen_switcher_entries().get(pressed) {
                    // Queues the route; the paint-time `reconcile` pump is
                    // what swaps the mounted screen and rebuilds the frame.
                    session.navigate_to_screen(path);
                }
            }
        }
        let _ = (viewport_w, viewport_h);
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn screen_switcher_hover(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let Some(session) = self.preview.as_ref() else {
            return;
        };
        if !session.is_app_mode() {
            return;
        }
        let count = session.screen_switcher_entries().len();
        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        let hover = ScreenSwitcherPills::hit_test(canvas, count, Point2D::new(x, y));
        if self.editor_state.editor_ui.preview.screen_switcher_hover != hover {
            self.editor_state.editor_ui.preview.screen_switcher_hover = hover;
            self.mark_dirty();
        }
    }

    /// Route a printable character into the live preview runtime.
    /// Returns `true` when consumed by a focused widget. No-op (false)
    /// when not in preview.
    pub(in crate::widget_host) fn preview_dispatch_text(&mut self, text: &str) -> bool {
        let consumed = self.preview.as_mut().is_some_and(|p| p.dispatch_text(text));
        if consumed {
            self.mark_dirty();
        }
        consumed
    }

    /// Advance focus to the next (`shift=false`) / previous
    /// (`shift=true`) focusable widget in the live preview runtime —
    /// Tab / Shift+Tab. Returns `true` while in preview.
    pub(in crate::widget_host) fn preview_focus(&mut self, shift: bool) -> bool {
        let Some(preview) = self.preview.as_mut() else {
            return false;
        };
        if shift {
            preview.focus_previous();
        } else {
            preview.focus_next();
        }
        self.mark_dirty();
        true
    }

    /// Route a named key into the live preview runtime. `shift` drives
    /// Shift+Tab focus traversal. Returns `true` when the runtime emitted
    /// any semantic event.
    pub(in crate::widget_host) fn preview_dispatch_key(&mut self, key: &str, shift: bool) -> bool {
        // Presenting a deck: the arrow keys move through the slides. They
        // are checked before the runtime sees them because during a
        // presentation that IS what they mean — a slide's widgets are being
        // shown, not filled in.
        if self.preview_slideshow_active() {
            match key {
                // Keynote's conventions: either axis steps the deck, so a
                // presenter's muscle memory works whichever key they reach
                // for.
                "ArrowRight" | "ArrowDown" => {
                    self.preview_slideshow_step(1);
                    return true;
                }
                "ArrowLeft" | "ArrowUp" => {
                    self.preview_slideshow_step(-1);
                    return true;
                }
                "Home" => {
                    self.preview_slideshow_to_end(false);
                    return true;
                }
                "End" => {
                    self.preview_slideshow_to_end(true);
                    return true;
                }
                _ => {}
            }
        }
        use jian_core::gesture::pointer::Modifiers;
        let mods = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::empty()
        };
        let handled = self
            .preview
            .as_mut()
            .is_some_and(|p| p.dispatch_key(key, mods));
        // Tab traversal / text edits change focus or content even when
        // no semantic event fires, so always repaint while in preview.
        if self.preview.is_some() {
            self.mark_dirty();
        }
        handled
    }

    /// Screen-space distance from the framed content's left edge a press
    /// must start within to arm an edge-swipe candidate.
    const EDGE_ZONE_PX: f32 = 24.0;
    /// Cumulative rightward drag distance that fires the pop.
    const POP_THRESHOLD_PX: f32 = 60.0;

    /// Arm (or clear) an edge-swipe candidate for a fresh press at
    /// `screen_x`. Called from preview press right after the
    /// underlying pointer Down is forwarded.
    pub(in crate::widget_host) fn arm_edge_swipe_candidate(&mut self, screen_x: f32) {
        // Clear FIRST, exactly as native does: a fresh press outside the
        // edge zone must disarm any stale candidate, or a leftover armed
        // value from an undisarmed gesture fires a pop on an ordinary drag.
        self.preview_edge_swipe_start_x = None;
        // Edge swipe only works in device-frame preview with a pop-capable stack
        if !self.device_mode_active() {
            return;
        }
        let Some(frame) = self.preview_device_frame.as_ref() else {
            return;
        };
        let edge = frame.content_span_x.0;
        if screen_x < edge || screen_x - edge >= Self::EDGE_ZONE_PX {
            return;
        }
        if !self.preview.as_ref().is_some_and(|p| p.can_pop()) {
            return;
        }
        self.preview_edge_swipe_start_x = Some(screen_x);
    }

    /// Check an armed candidate against the held drag's current `screen_x`.
    /// Returns `true` exactly once per gesture when the threshold is crossed
    /// and fires the pop.
    pub(in crate::widget_host) fn maybe_fire_edge_swipe(&mut self, screen_x: f32) -> bool {
        if self.preview_edge_swipe_start_x.is_none() {
            return false;
        }
        if screen_x - self.preview_edge_swipe_start_x.unwrap() < Self::POP_THRESHOLD_PX {
            return false;
        }
        self.preview_edge_swipe_start_x = None;
        if let Some(preview) = self.preview.as_ref() {
            preview.pop_screen();
            self.on_preview_screen_switched(self.last_viewport_w, self.last_viewport_h);
        }
        true
    }

    /// Clear any armed candidate — called on release so a gesture that
    /// never crossed the threshold doesn't leak into the NEXT press.
    pub(in crate::widget_host) fn disarm_edge_swipe(&mut self) {
        self.preview_edge_swipe_start_x = None;
    }

    /// Cancel the in-flight preview pointer gesture after an edge-swipe
    /// fires. Per-pointer (R4): every held id's arena timers settle and
    /// its capture anchor releases.
    pub(in crate::widget_host) fn cancel_preview_gesture_for_edge_swipe(&mut self) {
        let pids = std::mem::take(&mut self.preview_pressed_pids);
        for pid in pids {
            if let Some(p) = self.preview.as_mut() {
                p.cancel_pointer(pid, self.now_ms);
            }
        }
    }
}
