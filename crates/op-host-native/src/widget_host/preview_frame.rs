//! Device-frame host logic: geometry, inference, scroll state, and
//! screen-to-scene input transforms.

use op_editor_core::PreviewDeviceKind;
use op_editor_ui::widgets::TOP_BAR_HEIGHT;
use op_editor_ui::{Point2D, Rect};

/// Pinned-strip geometry in the device frame's screen space — shared by
/// the bottom nav and the top status bar (the two are otherwise
/// symmetric: one node id, one screen-space strip, one paint origin).
pub(crate) struct PinnedGeom {
    pub node_id: String,
    /// Strip node's scene rect (root-relative document space).
    pub node_scene: Rect,
    /// Full-width screen-space strip at the top or bottom of the frame.
    pub strip: Rect,
    /// Screen-space origin where the strip's subtree paints.
    pub paint_origin: Point2D,
}

/// Device-frame geometry shared by paint and hit-testing.
pub(crate) struct DeviceFrame {
    pub kind: PreviewDeviceKind,
    /// Screen-space frame rect after fit and centering.
    pub frame: Rect,
    pub fit: f32,
    /// Screen-space page origin before applying the scroll offset.
    pub content_origin: Point2D,
    /// Pinned bottom nav, if detected.
    pub pinned: Option<PinnedGeom>,
    /// Pinned top status bar, if detected.
    pub pinned_top: Option<PinnedGeom>,
    /// Framed-root height in scene-space logical pixels.
    pub content_h: f32,
    /// Logical top of the final scrollable content extent.
    pub nav_top: f32,
    /// Screen-space x span occupied by the framed root.
    pub content_span_x: (f32, f32),
    /// Visible logical height after reserving the pinned nav / status
    /// bar strips.
    pub viewport_h: f32,
}

/// Presentation transform captured for one pointer gesture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PreviewSurface {
    Scrolled { scroll_y: f32 },
    Pinned,
    PinnedTop,
}

/// Resolve the presentation surface under a screen-space point.
/// Dead zones apply only while resolving the surface at pointer down.
pub(crate) fn device_surface_at(
    frame: &DeviceFrame,
    screen: Point2D,
    scroll_y: f32,
) -> Option<PreviewSurface> {
    let frame_rect = frame.frame;
    let inside = screen.x >= frame_rect.origin.x
        && screen.x <= frame_rect.origin.x + frame_rect.size.x
        && screen.y >= frame_rect.origin.y
        && screen.y <= frame_rect.origin.y + frame_rect.size.y;
    if !inside {
        return None;
    }
    if screen.x < frame.content_span_x.0 || screen.x > frame.content_span_x.1 {
        return None;
    }
    // Once the point falls in a strip's screen-space band, the result
    // is COMMITTED to that strip: either the pinned surface, or a dead
    // zone (`None`) if it's beside a strip narrower than the frame —
    // it must NOT fall through to `Scrolled` the way a miss on the
    // strip's band itself does (that's a real, if rare, case: a
    // narrower-than-frame bottom nav or top status bar still leaves
    // its own flanks live for the scrolled content underneath).
    if strip_band_contains(&frame.pinned, screen) {
        return pinned_surface_or_dead_zone(
            frame.pinned.as_ref().expect("checked Some above"),
            frame.fit,
            screen,
            PreviewSurface::Pinned,
        );
    }
    if strip_band_contains(&frame.pinned_top, screen) {
        return pinned_surface_or_dead_zone(
            frame.pinned_top.as_ref().expect("checked Some above"),
            frame.fit,
            screen,
            PreviewSurface::PinnedTop,
        );
    }
    Some(PreviewSurface::Scrolled { scroll_y })
}

/// Whether `screen` falls inside this (optional) strip's screen-space
/// band at all — the OUTER gate `device_surface_at` commits on before
/// checking the strip node's own (possibly narrower) horizontal span.
fn strip_band_contains(pinned: &Option<PinnedGeom>, screen: Point2D) -> bool {
    let Some(pinned) = pinned else {
        return false;
    };
    let strip = pinned.strip;
    screen.y >= strip.origin.y
        && screen.y <= strip.origin.y + strip.size.y
        && screen.x >= strip.origin.x
        && screen.x <= strip.origin.x + strip.size.x
}

/// Inside the strip's band (already checked by the caller): resolve to
/// the pinned surface, or `None` (dead zone) when the point is beside
/// the strip node's own narrower horizontal span.
fn pinned_surface_or_dead_zone(
    pinned: &PinnedGeom,
    fit: f32,
    screen: Point2D,
    surface: PreviewSurface,
) -> Option<PreviewSurface> {
    let left = pinned.paint_origin.x;
    let right = left + pinned.node_scene.size.x * fit;
    if screen.x < left || screen.x > right {
        return None;
    }
    Some(surface)
}

/// Map a screen point through a previously resolved presentation surface.
/// Captured gestures never apply dead zones again during held moves.
pub(crate) fn device_scene_point(
    frame: &DeviceFrame,
    surface: &PreviewSurface,
    screen: Point2D,
) -> Option<Point2D> {
    let through = |pinned: &PinnedGeom| {
        Point2D::new(
            pinned.node_scene.origin.x + (screen.x - pinned.paint_origin.x) / frame.fit,
            pinned.node_scene.origin.y + (screen.y - pinned.paint_origin.y) / frame.fit,
        )
    };
    match surface {
        PreviewSurface::Pinned => frame.pinned.as_ref().map(through),
        PreviewSurface::PinnedTop => frame.pinned_top.as_ref().map(through),
        PreviewSurface::Scrolled { scroll_y } => Some(Point2D::new(
            (screen.x - frame.content_origin.x) / frame.fit,
            (screen.y - frame.content_origin.y) / frame.fit + scroll_y,
        )),
    }
}

pub(crate) fn frame_size(kind: PreviewDeviceKind) -> (f32, f32) {
    match kind {
        PreviewDeviceKind::Phone => (390.0, 844.0),
        PreviewDeviceKind::Desktop | PreviewDeviceKind::Canvas => (1440.0, 900.0),
    }
}

pub(crate) fn frame_radius(kind: PreviewDeviceKind) -> f32 {
    match kind {
        PreviewDeviceKind::Phone => 24.0,
        PreviewDeviceKind::Desktop | PreviewDeviceKind::Canvas => 8.0,
    }
}

/// Infer Phone at or below 500 logical pixels; otherwise Desktop.
pub(crate) fn infer_kind_for_width(root_w: Option<f32>) -> PreviewDeviceKind {
    match root_w {
        Some(w) if w <= 500.0 => PreviewDeviceKind::Phone,
        _ => PreviewDeviceKind::Desktop,
    }
}

/// Compute all device-frame geometry from a canvas and framed root.
/// `status_scene` mirrors `nav_scene` for the pinned top status bar.
pub(crate) fn compute_frame_geometry(
    kind: PreviewDeviceKind,
    canvas: Rect,
    root_scene: Rect,
    nav_scene: Option<Rect>,
    status_scene: Option<Rect>,
) -> DeviceFrame {
    let (frame_w, frame_h) = frame_size(kind);
    let fit = (canvas.size.x / frame_w)
        .min(canvas.size.y / frame_h)
        .min(1.0);
    let frame = Rect {
        origin: Point2D::new(
            canvas.origin.x + (canvas.size.x - frame_w * fit) / 2.0,
            canvas.origin.y + (canvas.size.y - frame_h * fit) / 2.0,
        ),
        size: Point2D::new(frame_w * fit, frame_h * fit),
    };
    let content_origin = Point2D::new(
        frame.origin.x + (frame.size.x - root_scene.size.x * fit) / 2.0 - root_scene.origin.x * fit,
        frame.origin.y - root_scene.origin.y * fit,
    );
    // A generated flex root may retain its authored height even when its
    // bottom tab bar resolves below that edge. Track the effective scene
    // extent so the pinned-nav scroll range can still expose every item
    // before the bar.
    let content_h = nav_scene.map_or(root_scene.size.y, |nav| {
        root_scene
            .size
            .y
            .max(nav.origin.y + nav.size.y - root_scene.origin.y)
    });
    let pinned = nav_scene.map(|nav| {
        let strip = Rect {
            origin: Point2D::new(
                frame.origin.x,
                frame.origin.y + frame.size.y - nav.size.y * fit,
            ),
            size: Point2D::new(frame.size.x, nav.size.y * fit),
        };
        PinnedGeom {
            node_id: String::new(),
            node_scene: nav,
            strip,
            paint_origin: Point2D::new(content_origin.x + nav.origin.x * fit, strip.origin.y),
        }
    });
    let pinned_top = status_scene.map(|status| {
        let strip = Rect {
            origin: frame.origin,
            size: Point2D::new(frame.size.x, status.size.y * fit),
        };
        PinnedGeom {
            node_id: String::new(),
            node_scene: status,
            strip,
            paint_origin: Point2D::new(content_origin.x + status.origin.x * fit, strip.origin.y),
        }
    });
    let status_h = pinned_top.as_ref().map_or(0.0, |p| p.node_scene.size.y);
    let (nav_top, viewport_h) = match &pinned {
        Some(pinned) => {
            let nav_h = pinned.node_scene.size.y;
            (
                (pinned.node_scene.origin.y - root_scene.origin.y).max(0.0),
                frame_h - nav_h - status_h,
            )
        }
        None => (content_h, frame_h - status_h),
    };
    let content_span_x = (
        content_origin.x + root_scene.origin.x * fit,
        content_origin.x + (root_scene.origin.x + root_scene.size.x) * fit,
    );
    DeviceFrame {
        kind,
        frame,
        fit,
        content_origin,
        pinned,
        pinned_top,
        content_h,
        nav_top,
        content_span_x,
        viewport_h,
    }
}

pub(crate) fn scroll_max(frame: &DeviceFrame) -> f32 {
    debug_assert!(frame.nav_top <= frame.content_h + f32::EPSILON);
    (frame.nav_top - frame.viewport_h).max(0.0)
}

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
        self.preview_scroll_y = self.preview_scroll_y.clamp(0.0, scroll_max(&frame));
        self.preview_device_frame = Some(frame);
    }

    /// Apply a screen-pixel scroll delta to logical frame content.
    pub(crate) fn apply_device_scroll(&mut self, screen_delta_y: f32) {
        let Some(frame) = self.preview_device_frame.as_ref() else {
            return;
        };
        let next =
            (self.preview_scroll_y - screen_delta_y / frame.fit).clamp(0.0, scroll_max(frame));
        if (next - self.preview_scroll_y).abs() > f32::EPSILON {
            self.preview_scroll_y = next;
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

fn paint_corner_notches(
    backend: &mut dyn op_editor_ui::RenderBackend,
    frame: Rect,
    radius: f32,
    mask: op_editor_ui::Color,
) {
    let half = radius / 2.0;
    let inflated = Rect {
        origin: Point2D::new(frame.origin.x - half, frame.origin.y - half),
        size: Point2D::new(frame.size.x + radius, frame.size.y + radius),
    };
    backend.stroke_round_rect(inflated, radius + half, mask, radius);
}

// Pure geometry tests (`compute_frame_geometry` / `device_surface_at` /
// `device_scene_point`) live in the sibling `preview_frame_geometry_tests.rs`
// — this file was at the 800-line cap with them inline.
