//! Device-frame host logic: geometry, inference, scroll state, and
//! screen-to-scene input transforms.

use op_editor_core::PreviewDeviceKind;
use op_editor_ui::{Point2D, Rect};

/// Pinned-nav geometry in the device frame's screen space.
pub(crate) struct PinnedGeom {
    pub node_id: String,
    /// Nav node's scene rect (root-relative document space).
    pub node_scene: Rect,
    /// Full-width screen-space strip at the bottom of the frame.
    pub strip: Rect,
    /// Screen-space origin where the nav subtree paints.
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
    pub pinned: Option<PinnedGeom>,
    /// Framed-root height in scene-space logical pixels.
    pub content_h: f32,
    /// Logical top of the final scrollable content extent.
    pub nav_top: f32,
    /// Screen-space x span occupied by the framed root.
    pub content_span_x: (f32, f32),
    /// Visible logical height after reserving a pinned-nav strip.
    pub viewport_h: f32,
}

/// Presentation transform captured for one pointer gesture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PreviewSurface {
    Scrolled { scroll_y: f32 },
    Pinned,
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
    if let Some(pinned) = &frame.pinned {
        let strip = pinned.strip;
        let in_strip = screen.y >= strip.origin.y
            && screen.y <= strip.origin.y + strip.size.y
            && screen.x >= strip.origin.x
            && screen.x <= strip.origin.x + strip.size.x;
        if in_strip {
            let nav_left = pinned.paint_origin.x;
            let nav_right = nav_left + pinned.node_scene.size.x * frame.fit;
            if screen.x < nav_left || screen.x > nav_right {
                return None;
            }
            return Some(PreviewSurface::Pinned);
        }
    }
    Some(PreviewSurface::Scrolled { scroll_y })
}

/// Map a screen point through a previously resolved presentation surface.
/// Captured gestures never apply dead zones again during held moves.
pub(crate) fn device_scene_point(
    frame: &DeviceFrame,
    surface: &PreviewSurface,
    screen: Point2D,
) -> Option<Point2D> {
    match surface {
        PreviewSurface::Pinned => {
            let pinned = frame.pinned.as_ref()?;
            Some(Point2D::new(
                pinned.node_scene.origin.x + (screen.x - pinned.paint_origin.x) / frame.fit,
                pinned.node_scene.origin.y + (screen.y - pinned.paint_origin.y) / frame.fit,
            ))
        }
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
pub(crate) fn compute_frame_geometry(
    kind: PreviewDeviceKind,
    canvas: Rect,
    root_scene: Rect,
    nav_scene: Option<Rect>,
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
    let content_h = root_scene.size.y;
    let root_bottom = root_scene.origin.y + root_scene.size.y;
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
    let (nav_top, viewport_h) = match &pinned {
        Some(pinned) => {
            let nav_h = pinned.node_scene.size.y;
            let nav_bottom = pinned.node_scene.origin.y + nav_h;
            let gap = (root_bottom - nav_bottom).max(0.0);
            (content_h - nav_h - gap, frame_h - nav_h)
        }
        None => (content_h, frame_h),
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
        self.editor_state.editor_ui.preview_device = Some(kind);
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
                self.editor_state.editor_ui.preview_device,
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
        let Some(kind) = self.editor_state.editor_ui.preview_device else {
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
            ));
            self.preview_scroll_y = 0.0;
            return;
        };
        let nav = session.pinned_nav_candidate(kind == PreviewDeviceKind::Phone);
        let mut frame =
            compute_frame_geometry(kind, canvas, root_rect, nav.as_ref().map(|(_, rect)| *rect));
        if let (Some(pinned), Some((node_id, _))) = (frame.pinned.as_mut(), nav) {
            pinned.node_id = node_id;
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
        self.editor_state.editor_ui.preview_device = Some(kind);
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
            self.editor_state.editor_ui.preview_device = Some(kind);
        }
        self.preview_scroll_y = 0.0;
        self.preview_surface_capture = None;
        self.recompute_device_frame(viewport_w, viewport_h);
    }

    /// Paint fixed-frame content and its rounded silhouette chrome.
    pub(crate) fn paint_device_frame(
        &self,
        frame_backend: &mut dyn op_editor_ui::RenderBackend,
        canvas_rect: Rect,
    ) {
        let Some(session) = self.preview.as_ref() else {
            return;
        };
        let Some(device_frame) = self.preview_device_frame.as_ref() else {
            return;
        };
        let radius = frame_radius(device_frame.kind) * device_frame.fit;
        frame_backend.fill_round_rect(device_frame.frame, radius, op_editor_ui::Color::WHITE);

        if let Some((root_id, _)) = session.framed_root() {
            let content_clip = match &device_frame.pinned {
                Some(pinned) => Rect {
                    origin: device_frame.frame.origin,
                    size: Point2D::new(
                        device_frame.frame.size.x,
                        device_frame.frame.size.y - pinned.strip.size.y,
                    ),
                },
                None => device_frame.frame,
            };
            let content_origin = Point2D::new(
                device_frame.content_origin.x,
                device_frame.content_origin.y - self.preview_scroll_y * device_frame.fit,
            );
            let pinned_paint =
                device_frame
                    .pinned
                    .as_ref()
                    .map(|pinned| crate::preview::PinnedPaint {
                        node_id: pinned.node_id.clone(),
                        strip_clip: pinned.strip,
                        paint_origin: pinned.paint_origin,
                        nav_scene_origin: pinned.node_scene.origin,
                    });
            session.paint_framed(
                frame_backend,
                &root_id,
                content_clip,
                content_origin,
                device_frame.fit,
                pinned_paint.as_ref(),
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
        frame_backend.stroke_round_rect(device_frame.frame, radius, self.theme.border, 1.0);
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
            selected: self.editor_state.editor_ui.preview_device,
            hover: self.editor_state.editor_ui.preview_switcher_hover,
            pressed: self.editor_state.editor_ui.preview_switcher_pressed,
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
        self.editor_state.editor_ui.preview_switcher_pressed = hit;
        hit.is_some()
    }

    /// Activate on release when the maintained hover matches the press.
    pub(crate) fn preview_switcher_release(&mut self) -> bool {
        let pressed = self.editor_state.editor_ui.preview_switcher_pressed.take();
        let Some(pressed) = pressed else {
            return false;
        };
        if self.editor_state.editor_ui.preview_switcher_hover == Some(pressed) {
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
        if self.editor_state.editor_ui.preview_switcher_hover != hit {
            self.editor_state.editor_ui.preview_switcher_hover = hit;
            self.mark_dirty();
        }
    }

    fn preview_switcher_labels(&self) -> [&'static str; 3] {
        let locale = self.editor_state.editor_ui.locale;
        [
            op_i18n::translate(locale, "preview.device.phone"),
            op_i18n::translate(locale, "preview.device.desktop"),
            op_i18n::translate(locale, "preview.device.canvas"),
        ]
    }

    /// Canvas rect shared by switcher paint and hit-testing.
    pub(crate) fn preview_canvas_rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(w, h),
        }
    }

    #[test]
    fn infer_by_root_width() {
        assert_eq!(infer_kind_for_width(Some(390.0)), PreviewDeviceKind::Phone);
        assert_eq!(infer_kind_for_width(Some(500.0)), PreviewDeviceKind::Phone);
        assert_eq!(
            infer_kind_for_width(Some(501.0)),
            PreviewDeviceKind::Desktop
        );
        assert_eq!(
            infer_kind_for_width(Some(1440.0)),
            PreviewDeviceKind::Desktop
        );
        assert_eq!(infer_kind_for_width(None), PreviewDeviceKind::Desktop);
    }

    #[test]
    fn fit_caps_at_one_and_shrinks_to_region() {
        let frame = compute_frame_geometry(
            PreviewDeviceKind::Phone,
            rect(0.0, 0.0, 1000.0, 1000.0),
            rect(0.0, 0.0, 390.0, 800.0),
            None,
        );
        assert!((frame.fit - 1.0).abs() < 1e-6);

        let frame = compute_frame_geometry(
            PreviewDeviceKind::Phone,
            rect(0.0, 0.0, 1000.0, 422.0),
            rect(0.0, 0.0, 390.0, 800.0),
            None,
        );
        assert!((frame.fit - 0.5).abs() < 1e-3);
        assert!((frame.frame.size.y - 422.0).abs() < 1.0);
    }

    #[test]
    fn content_origin_centers_horizontally() {
        let frame = compute_frame_geometry(
            PreviewDeviceKind::Phone,
            rect(0.0, 0.0, 2000.0, 2000.0),
            rect(0.0, 0.0, 300.0, 800.0),
            None,
        );
        let expected_x = frame.frame.origin.x + (frame.frame.size.x - 300.0) / 2.0;
        assert!((frame.content_origin.x - expected_x).abs() < 0.5);
    }

    #[test]
    fn scroll_max_uses_nav_top() {
        let frame = compute_frame_geometry(
            PreviewDeviceKind::Phone,
            rect(0.0, 0.0, 2000.0, 2000.0),
            rect(0.0, 0.0, 390.0, 2000.0),
            Some(rect(0.0, 1940.0, 390.0, 60.0)),
        );
        assert!((frame.nav_top - 1940.0).abs() < 0.5);
        assert!((frame.viewport_h - 784.0).abs() < 0.5);
        assert!((scroll_max(&frame) - 1156.0).abs() < 0.5);

        let frame = compute_frame_geometry(
            PreviewDeviceKind::Phone,
            rect(0.0, 0.0, 2000.0, 2000.0),
            rect(0.0, 0.0, 390.0, 2000.0),
            None,
        );
        assert!((scroll_max(&frame) - 1156.0).abs() < 0.5);

        let frame = compute_frame_geometry(
            PreviewDeviceKind::Phone,
            rect(0.0, 0.0, 2000.0, 2000.0),
            rect(0.0, 0.0, 390.0, 400.0),
            None,
        );
        assert_eq!(scroll_max(&frame), 0.0);
    }

    #[test]
    fn floating_nav_gap_is_dead_space() {
        let frame = compute_frame_geometry(
            PreviewDeviceKind::Phone,
            rect(0.0, 0.0, 2000.0, 2000.0),
            rect(0.0, 0.0, 390.0, 2000.0),
            Some(rect(0.0, 1930.0, 390.0, 60.0)),
        );
        assert!((frame.nav_top - 1930.0).abs() < 0.5);
        assert!((scroll_max(&frame) - (1930.0 - 784.0)).abs() < 0.5);
    }

    fn phone_frame_with_nav() -> DeviceFrame {
        let mut frame = compute_frame_geometry(
            PreviewDeviceKind::Phone,
            rect(0.0, 0.0, 1000.0, 1000.0),
            rect(0.0, 0.0, 390.0, 2000.0),
            Some(rect(0.0, 1940.0, 390.0, 60.0)),
        );
        if let Some(pinned) = frame.pinned.as_mut() {
            pinned.node_id = "nav".into();
        }
        frame
    }

    #[test]
    fn surface_and_scene_point_scrolled_region() {
        let frame = phone_frame_with_nav();
        let screen = Point2D::new(frame.frame.origin.x + 100.0, frame.frame.origin.y + 100.0);
        let surface = device_surface_at(&frame, screen, 50.0).expect("inside frame");
        match surface {
            PreviewSurface::Scrolled { scroll_y } => assert_eq!(scroll_y, 50.0),
            PreviewSurface::Pinned => panic!("expected scrolled surface"),
        }
        let scene = device_scene_point(&frame, &surface, screen).expect("maps");
        assert!((scene.x - 100.0).abs() < 0.5);
        assert!((scene.y - 150.0).abs() < 0.5);
    }

    #[test]
    fn strip_maps_through_pinned_inverse_and_dead_zone() {
        let frame = phone_frame_with_nav();
        let strip = frame.pinned.as_ref().unwrap().strip;
        let screen = Point2D::new(strip.origin.x + 10.0, strip.origin.y + 30.0);
        let surface = device_surface_at(&frame, screen, 999.0).expect("inside strip");
        assert!(matches!(surface, PreviewSurface::Pinned));
        let scene = device_scene_point(&frame, &surface, screen).expect("maps to nav");
        assert!((scene.y - 1970.0).abs() < 0.5);

        let mut narrow = compute_frame_geometry(
            PreviewDeviceKind::Phone,
            rect(0.0, 0.0, 1000.0, 1000.0),
            rect(0.0, 0.0, 390.0, 2000.0),
            Some(rect(95.0, 1940.0, 200.0, 60.0)),
        );
        if let Some(pinned) = narrow.pinned.as_mut() {
            pinned.node_id = "nav".into();
        }
        let beside = Point2D::new(
            narrow.frame.origin.x + 5.0,
            narrow.pinned.as_ref().unwrap().strip.origin.y + 30.0,
        );
        assert!(
            device_surface_at(&narrow, beside, 0.0).is_none(),
            "strip point beside a narrower nav resolves to no surface"
        );
        let over_nav = Point2D::new(
            narrow.pinned.as_ref().unwrap().paint_origin.x + 5.0,
            narrow.pinned.as_ref().unwrap().strip.origin.y + 30.0,
        );
        let captured = device_surface_at(&narrow, over_nav, 0.0).expect("over the nav");
        assert!(
            device_scene_point(&narrow, &captured, beside).is_some(),
            "captured drags never dead-zone mid-gesture"
        );
    }

    #[test]
    fn outside_frame_is_none_and_capture_freezes_surface() {
        let frame = phone_frame_with_nav();
        let outside = Point2D::new(frame.frame.origin.x - 10.0, frame.frame.origin.y + 10.0);
        assert!(device_surface_at(&frame, outside, 0.0).is_none());

        let strip = frame.pinned.as_ref().unwrap().strip;
        let down = Point2D::new(strip.origin.x + 10.0, strip.origin.y + 30.0);
        let captured = device_surface_at(&frame, down, 0.0).expect("down in strip");
        let moved = Point2D::new(strip.origin.x + 10.0, strip.origin.y - 100.0);
        let scene = device_scene_point(&frame, &captured, moved).expect("still pinned space");
        assert!((scene.y - 1840.0).abs() < 0.5);
    }
}
