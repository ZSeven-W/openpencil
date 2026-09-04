//! Device-frame presentation geometry — shared by every preview host.
//!
//! Preview presents a screen inside a fixed device silhouette (phone /
//! desktop), fits and centres it in the canvas region, and pins the
//! screen's bottom nav / top status bar out of the scroll flow. All of
//! that is pure math over `Rect`s, so it lives here rather than in a
//! host: the native and web hosts MUST agree on it pixel-for-pixel or
//! preview taps stop landing where preview paints.
//!
//! Hosts own the state (which device is selected, current scroll offset)
//! and the decoration; this module owns the geometry and the screen ↔
//! scene transforms that go with it.

use op_editor_core::PreviewDeviceKind;
use op_editor_ui::{Point2D, Rect};

/// Pinned-strip geometry in the device frame's screen space — shared by
/// the bottom nav and the top status bar (the two are otherwise
/// symmetric: one node id, one screen-space strip, one paint origin).
pub struct PinnedGeom {
    pub node_id: String,
    /// Strip node's scene rect (root-relative document space).
    pub node_scene: Rect,
    /// Full-width screen-space strip at the top or bottom of the frame.
    pub strip: Rect,
    /// Screen-space origin where the strip's subtree paints.
    pub paint_origin: Point2D,
}

/// Device-frame geometry shared by paint and hit-testing.
pub struct DeviceFrame {
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
pub enum PreviewSurface {
    Scrolled { scroll_y: f32 },
    Pinned,
    PinnedTop,
}

/// Resolve the presentation surface under a screen-space point.
/// Dead zones apply only while resolving the surface at pointer down.
pub fn device_surface_at(
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
pub fn device_scene_point(
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

pub fn frame_size(kind: PreviewDeviceKind) -> (f32, f32) {
    match kind {
        PreviewDeviceKind::Phone => (390.0, 844.0),
        PreviewDeviceKind::Desktop | PreviewDeviceKind::Canvas => (1440.0, 900.0),
    }
}

pub fn frame_radius(kind: PreviewDeviceKind) -> f32 {
    match kind {
        PreviewDeviceKind::Phone => 24.0,
        PreviewDeviceKind::Desktop | PreviewDeviceKind::Canvas => 8.0,
    }
}

/// Infer Phone at or below 500 logical pixels; otherwise Desktop.
pub fn infer_kind_for_width(root_w: Option<f32>) -> PreviewDeviceKind {
    match root_w {
        Some(w) if w <= 500.0 => PreviewDeviceKind::Phone,
        _ => PreviewDeviceKind::Desktop,
    }
}

/// Compute all device-frame geometry from a canvas and framed root.
/// `status_scene` mirrors `nav_scene` for the pinned top status bar.
pub fn compute_frame_geometry(
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

pub fn scroll_max(frame: &DeviceFrame) -> f32 {
    debug_assert!(frame.nav_top <= frame.content_h + f32::EPSILON);
    (frame.nav_top - frame.viewport_h).max(0.0)
}
pub fn paint_corner_notches(
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
