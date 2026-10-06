//! Device-frame host logic: state, paint, and input glue.
//!
//! The pure geometry (frame fit/centring, pinned-strip detection, the
//! screen <-> scene transforms) lives in `op_preview_core::device_frame`
//! so the web host solves it identically. This file is the native
//! host's state + decoration on top of it.

use op_editor_core::PreviewDeviceKind;
use op_editor_ui::widgets::TOP_BAR_HEIGHT;
use op_editor_ui::{Point2D, Rect};

pub(crate) use op_preview_core::device_frame::{
    compute_frame_geometry, device_scene_point, device_surface_at, frame_radius,
    infer_kind_for_width, paint_corner_notches, scroll_max, DeviceFrame, PinnedGeom,
    PreviewSurface,
};

impl super::WidgetHostNative {
    pub(crate) fn initialize_device_preview(&mut self) {
        let kind = self.infer_device_kind();
        self.editor_state.editor_ui.preview.device = Some(kind);
        self.preview_scroll_y = 0.0;
        self.recompute_device_frame(self.last_viewport_w, self.last_viewport_h);
    }

    pub(crate) fn center_preview_entry_if_canvas(&mut self, canvas_size: (f32, f32)) {
        if self.device_mode_active() {
            return;
        }
        if let Some(rect) = self
            .preview
            .as_ref()
            .and_then(|preview| preview.current_screen_scene_rect())
        {
            self.center_canvas_on(rect, canvas_size.0, canvas_size.1);
        }
    }

    pub(crate) fn clear_device_preview_state(&mut self) {
        self.preview_device_frame = None;
        self.preview_scroll_y = 0.0;
        self.preview_manual_pick = None;
        self.preview_surface_capture = None;
    }

    pub(crate) fn cache_preview_viewport(&mut self, viewport_w: f32, viewport_h: f32) {
        self.last_viewport_w = viewport_w;
        self.last_viewport_h = viewport_h;
    }

    pub(crate) fn device_preview_doc_point(&self, screen_x: f32, screen_y: f32) -> Option<Point2D> {
        let frame = self.preview_device_frame.as_ref()?;
        let screen = Point2D::new(screen_x, screen_y);
        let surface = match self.preview_surface_capture {
            Some(surface) => surface,
            None => device_surface_at(frame, screen, self.preview_scroll_y)?,
        };
        device_scene_point(frame, &surface, screen)
    }

    pub(crate) fn capture_device_preview_surface(&mut self, screen_x: f32, screen_y: f32) {
        if let Some(frame) = self.preview_device_frame.as_ref() {
            self.preview_surface_capture = device_surface_at(
                frame,
                Point2D::new(screen_x, screen_y),
                self.preview_scroll_y,
            );
        }
    }

    /// Whether preview is currently presented through a device frame.
    pub(crate) fn device_mode_active(&self) -> bool {
        self.preview.is_some()
            && matches!(
                self.editor_state.editor_ui.preview.device,
                Some(PreviewDeviceKind::Phone) | Some(PreviewDeviceKind::Desktop)
            )
    }

    /// Resolve the manual pick or infer from the framed root width.
    pub(crate) fn infer_device_kind(&self) -> PreviewDeviceKind {
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

    /// Rebuild the cached frame from the current session and viewport.
    pub(crate) fn recompute_device_frame(&mut self, viewport_w: f32, viewport_h: f32) {
        if !self.device_mode_active() {
            self.preview_device_frame = None;
            return;
        }
        let Some(kind) = self.editor_state.editor_ui.preview.device else {
            self.preview_device_frame = None;
            return;
        };
        let (canvas_x, canvas_y, canvas_w, canvas_h) = self.canvas_region(viewport_w, viewport_h);
        if canvas_w <= 0.0 || canvas_h <= 0.0 {
            self.preview_device_frame = None;
            return;
        }
        let canvas = Rect {
            origin: Point2D::new(canvas_x, canvas_y),
            size: Point2D::new(canvas_w, canvas_h),
        };
        let Some(session) = self.preview.as_ref() else {
            self.preview_device_frame = None;
            return;
        };
        let Some((_root_id, root_rect)) = session.framed_root() else {
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
        let max = scroll_max(&frame);
        self.preview_scroll_y = self.preview_scroll_y.clamp(0.0, max);
        self.preview_device_frame = Some(frame);
        self.sync_page_scroll(max, 0.0);
    }

    /// Apply a screen-pixel scroll delta to logical frame content.
    pub(crate) fn apply_device_scroll(&mut self, screen_delta_y: f32) {
        let Some(frame) = self.preview_device_frame.as_ref() else {
            return;
        };
        let max = scroll_max(frame);
        let next = (self.preview_scroll_y - screen_delta_y / frame.fit).clamp(0.0, max);
        if (next - self.preview_scroll_y).abs() > f32::EPSILON {
            self.preview_scroll_y = next;
            self.mark_dirty();
        }
        self.sync_page_scroll(max, screen_delta_y);
    }

    /// Feed the device frame's scroll position into the session so
    /// `$scroll` bindings under the framed root track the page scroll
    /// (page-scroll contract). Repaints when a bound value moved even
    /// if the offset itself did not (a new `max_offset` changes
    /// `$scroll.progress`).
    fn sync_page_scroll(&mut self, max_offset: f32, delta_y: f32) {
        let offset = self.preview_scroll_y;
        let changed = self
            .preview
            .as_mut()
            .is_some_and(|session| session.set_page_scroll(offset, max_offset, delta_y));
        if changed {
            self.mark_dirty();
        }
    }

    /// Activate one switcher segment and rebuild its presentation.
    pub(crate) fn set_preview_device(
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
            if let Some(rect) = self
                .preview
                .as_ref()
                .and_then(|preview| preview.current_screen_scene_rect())
            {
                self.center_canvas_on(rect, viewport_w, viewport_h);
            }
        } else {
            self.recompute_device_frame(viewport_w, viewport_h);
        }
        self.mark_dirty();
    }

    /// Re-infer after an app-mode screen switch and reset gesture state.
    pub(crate) fn on_preview_screen_switched(&mut self, viewport_w: f32, viewport_h: f32) {
        if self.preview_manual_pick.is_none() {
            let kind = self.infer_device_kind();
            self.editor_state.editor_ui.preview.device = Some(kind);
        }
        self.preview_scroll_y = 0.0;
        self.preview_surface_capture = None;
        self.recompute_device_frame(viewport_w, viewport_h);
    }

    /// Paint fixed-frame content and its rounded silhouette chrome.
    ///
    /// Track M-1: while a canvas ↔ device-frame merge animation is
    /// active, the geometry used for this paint is NOT the settled
    /// `self.preview_device_frame` — it is re-derived (same root / nav
    /// / status inputs) against the transition's interpolated rect, so
    /// the silhouette + content visibly move/scale between the
    /// screen's canvas position and the settled frame. The bezel /
    /// border colours blend toward the canvas backdrop at the
    /// un-settled end of the animation (`chrome_blend`) — see
    /// `preview::ModeTransition`'s module doc for why content itself
    /// needs no separate fade.
    pub(crate) fn paint_device_frame(
        &self,
        frame_backend: &mut dyn op_editor_ui::RenderBackend,
        canvas_rect: Rect,
    ) {
        let Some(session) = self.preview.as_ref() else {
            return;
        };
        let Some(steady_frame) = self.preview_device_frame.as_ref() else {
            return;
        };

        let active_mode_transition = self
            .preview_mode_transition
            .as_ref()
            .filter(|t| t.is_active(self.now_ms));
        let interpolated;
        let (device_frame, chrome_blend): (&DeviceFrame, f32) = match active_mode_transition {
            Some(transition) => {
                let is_phone = steady_frame.kind == PreviewDeviceKind::Phone;
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
                if let (Some(pinned_top), Some((node_id, _))) = (frame.pinned_top.as_mut(), status)
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
        // white: the device silhouette is a fixed size (`frame_size`)
        // that doesn't always match the screen's authored width, so a
        // narrower design shows a thin strip of bezel on each side —
        // this keeps that strip blending with the design instead of
        // reading as a stray light seam against a dark theme.
        let settled_bezel_fill = session
            .framed_root_fill()
            .unwrap_or(self.theme.canvas_surface);
        // Blend toward the ALREADY-PAINTED canvas backdrop
        // (`theme.canvas_surface`, filled by the caller before this
        // runs) at the un-settled end of a Track M-1 merge — equivalent
        // to true alpha blending since the backdrop underneath is a
        // known solid colour.
        let bezel_fill =
            crate::preview::lerp_color(self.theme.canvas_surface, settled_bezel_fill, chrome_blend);
        frame_backend.fill_round_rect(device_frame.frame, radius, bezel_fill);

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
            let to_pinned_paint = |pinned: &PinnedGeom| crate::preview::PinnedPaint {
                node_id: pinned.node_id.clone(),
                strip_clip: pinned.strip,
                paint_origin: pinned.paint_origin,
                nav_scene_origin: pinned.node_scene.origin,
            };
            let pinned_paint = device_frame.pinned.as_ref().map(to_pinned_paint);
            let pinned_top_paint = device_frame.pinned_top.as_ref().map(to_pinned_paint);
            // Track C-3: routes to the plain single-layer `paint_framed`
            // when idle, or composites the outgoing/entering screens for
            // an in-flight push/pop/replace transition.
            session.paint_framed_animated(
                frame_backend,
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
        // rounded-frame interior. Clip it to the canvas so it cannot touch
        // the TopBar or rails painted earlier.
        frame_backend.save();
        frame_backend.clip_rect(canvas_rect);
        paint_corner_notches(
            frame_backend,
            device_frame.frame,
            radius,
            self.theme.canvas_surface,
        );
        let border_color =
            crate::preview::lerp_color(self.theme.canvas_surface, self.theme.border, chrome_blend);
        frame_backend.stroke_round_rect(device_frame.frame, radius, border_color, 1.0);
        frame_backend.restore();
    }

    /// Paint the switcher after Preview content in every Preview mode.
    pub(crate) fn paint_preview_switcher(
        &self,
        frame_backend: &mut dyn op_editor_ui::RenderBackend,
        canvas_rect: Rect,
    ) {
        use op_editor_ui::widgets::{PaintCx, PreviewDeviceSwitcher};
        let switcher = PreviewDeviceSwitcher {
            labels: self.preview_switcher_labels(),
            selected: self.editor_state.editor_ui.preview.device,
            hover: self.editor_state.editor_ui.preview.switcher_hover,
            pressed: self.editor_state.editor_ui.preview.switcher_pressed,
        };
        let mut cx = PaintCx {
            backend: frame_backend,
        };
        switcher.paint(&mut cx, canvas_rect, &self.theme);
    }

    pub(crate) fn preview_switcher_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        use op_editor_ui::widgets::PreviewDeviceSwitcher;
        if self.preview.is_none() {
            return false;
        }
        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        let hit = PreviewDeviceSwitcher::hit_test(canvas, Point2D::new(x, y));
        self.editor_state.editor_ui.preview.switcher_pressed = hit;
        hit.is_some()
    }

    /// Activate on release when the maintained hover matches the press.
    pub(crate) fn preview_switcher_release(&mut self) -> bool {
        let pressed = self.editor_state.editor_ui.preview.switcher_pressed.take();
        let Some(pressed) = pressed else {
            return false;
        };
        if self.editor_state.editor_ui.preview.switcher_hover == Some(pressed) {
            let (viewport_w, viewport_h) = (self.last_viewport_w, self.last_viewport_h);
            self.set_preview_device(pressed, viewport_w, viewport_h);
        }
        self.mark_dirty();
        true
    }

    pub(crate) fn preview_switcher_hover(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        use op_editor_ui::widgets::PreviewDeviceSwitcher;
        if self.preview.is_none() {
            return;
        }
        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        let hit = PreviewDeviceSwitcher::hit_test(canvas, Point2D::new(x, y));
        if self.editor_state.editor_ui.preview.switcher_hover != hit {
            self.editor_state.editor_ui.preview.switcher_hover = hit;
            self.mark_dirty();
        }
    }

    fn preview_switcher_labels(&self) -> [&'static str; 3] {
        let locale = self.editor_state.editor_ui.effective_locale();
        [
            op_i18n::translate(locale, "preview.device.phone"),
            op_i18n::translate(locale, "preview.device.desktop"),
            op_i18n::translate(locale, "preview.device.canvas"),
        ]
    }

    /// Canvas rect shared by switcher paint and hit-testing.
    pub(crate) fn preview_canvas_rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        if self.preview_slideshow_active() {
            // Presenting hides the rails. Desktop keeps its TopBar because
            // it remains painted; touch layouts hide their app bar and dock
            // in favour of presenter controls, so their safe-area-local
            // viewport is the whole stage with no unexplained top band.
            let top = if self.editor_state.editor_ui.touch_chrome() {
                0.0
            } else {
                TOP_BAR_HEIGHT
            };
            return Rect {
                origin: Point2D::new(0.0, top),
                size: Point2D::new(viewport_w, (viewport_h - top).max(0.0)),
            };
        }
        let (canvas_x, canvas_y, canvas_w, canvas_h) = self.canvas_region(viewport_w, viewport_h);
        Rect {
            origin: Point2D::new(canvas_x, canvas_y),
            size: Point2D::new(canvas_w, canvas_h),
        }
    }
}

// Pure geometry tests (`compute_frame_geometry` / `device_surface_at` /
// `device_scene_point`) live in the sibling `preview_frame_geometry_tests.rs`
// — this file was at the 800-line cap with them inline.
