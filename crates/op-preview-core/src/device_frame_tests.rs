//! Pure device-frame geometry tests — `compute_frame_geometry` +
//! `device_surface_at` / `device_scene_point`. Split out of
//! `preview_frame.rs` (which also carries the `impl WidgetHostNative`
//! paint/hit-test glue) to keep that file under the repo's
//! 800-line-per-file cap; the host-lifecycle integration tests live in
//! the sibling `preview_frame_tests.rs`.

#![cfg(test)]

use super::device_frame::*;
use op_editor_core::PreviewDeviceKind;
use op_editor_ui::{Point2D, Rect};

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
        None,
    );
    assert!((frame.fit - 1.0).abs() < 1e-6);

    let frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 1000.0, 422.0),
        rect(0.0, 0.0, 390.0, 800.0),
        None,
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
        None,
    );
    assert!((frame.nav_top - 1940.0).abs() < 0.5);
    assert!((frame.viewport_h - 784.0).abs() < 0.5);
    assert!((scroll_max(&frame) - 1156.0).abs() < 0.5);

    let frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 2000.0, 2000.0),
        rect(0.0, 0.0, 390.0, 2000.0),
        None,
        None,
    );
    assert!((scroll_max(&frame) - 1156.0).abs() < 0.5);

    let frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 2000.0, 2000.0),
        rect(0.0, 0.0, 390.0, 400.0),
        None,
        None,
    );
    assert_eq!(scroll_max(&frame), 0.0);
}

#[test]
fn short_page_nav_strip_is_flush_with_device_bottom() {
    let frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 2000.0, 2000.0),
        rect(0.0, 0.0, 375.0, 816.0),
        Some(rect(0.0, 726.0, 375.0, 80.0)),
        None,
    );
    let nav = frame.pinned.as_ref().expect("nav geometry");
    let nav_bottom = nav.strip.origin.y + nav.strip.size.y;
    let device_bottom = frame.frame.origin.y + frame.frame.size.y;

    assert!((nav_bottom - device_bottom).abs() < 0.5);
    assert_eq!(scroll_max(&frame), 0.0, "short content must not scroll");
}

#[test]
fn overflowing_nav_uses_its_actual_top_for_scroll_extent() {
    let frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 2000.0, 2000.0),
        rect(0.0, 0.0, 375.0, 812.0),
        Some(rect(0.0, 841.0, 375.0, 80.0)),
        None,
    );

    assert!((frame.content_h - 921.0).abs() < 0.5);
    assert!((frame.nav_top - 841.0).abs() < 0.5);
    assert!((frame.viewport_h - 764.0).abs() < 0.5);
    assert!((scroll_max(&frame) - 77.0).abs() < 0.5);
}

#[test]
fn floating_nav_gap_is_dead_space() {
    let frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 2000.0, 2000.0),
        rect(0.0, 0.0, 390.0, 2000.0),
        Some(rect(0.0, 1930.0, 390.0, 60.0)),
        None,
    );
    assert!((frame.nav_top - 1930.0).abs() < 0.5);
    assert!((scroll_max(&frame) - (1930.0 - 784.0)).abs() < 0.5);
}

#[test]
fn status_bar_reserves_top_and_shrinks_viewport() {
    let frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 2000.0, 2000.0),
        rect(0.0, 0.0, 390.0, 2000.0),
        Some(rect(0.0, 1940.0, 390.0, 60.0)),
        Some(rect(0.0, 0.0, 390.0, 44.0)),
    );
    let pinned_top = frame.pinned_top.as_ref().expect("status bar geometry");
    assert!((pinned_top.strip.origin.y - frame.frame.origin.y).abs() < 0.5);
    assert!((pinned_top.strip.size.y - 44.0).abs() < 0.5);
    // Both strips now eat into the visible window.
    assert!((frame.viewport_h - (784.0 - 44.0)).abs() < 0.5);
    assert!((scroll_max(&frame) - (1156.0 + 44.0)).abs() < 0.5);
}

fn phone_frame_with_nav() -> DeviceFrame {
    let mut frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 1000.0, 1000.0),
        rect(0.0, 0.0, 390.0, 2000.0),
        Some(rect(0.0, 1940.0, 390.0, 60.0)),
        None,
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
        _ => panic!("expected scrolled surface"),
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
        None,
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

#[test]
fn status_bar_strip_maps_through_pinned_top() {
    let mut frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 1000.0, 1000.0),
        rect(0.0, 0.0, 390.0, 2000.0),
        Some(rect(0.0, 1940.0, 390.0, 60.0)),
        Some(rect(0.0, 0.0, 390.0, 44.0)),
    );
    if let Some(pinned) = frame.pinned.as_mut() {
        pinned.node_id = "nav".into();
    }
    if let Some(pinned_top) = frame.pinned_top.as_mut() {
        pinned_top.node_id = "status".into();
    }
    let strip = frame.pinned_top.as_ref().unwrap().strip;
    // The top strip sits flush with the frame's own top edge.
    assert!((strip.origin.y - frame.frame.origin.y).abs() < 0.5);

    let screen = Point2D::new(strip.origin.x + 10.0, strip.origin.y + 10.0);
    let surface = device_surface_at(&frame, screen, 999.0).expect("inside the status strip");
    assert!(matches!(surface, PreviewSurface::PinnedTop));
    let scene = device_scene_point(&frame, &surface, screen).expect("maps to the status bar");
    assert!((scene.y - 10.0).abs() < 0.5, "got {:?}", scene);

    // A point below the status strip (but still in the frame) is
    // the ordinary scrolled surface, unaffected by the top pin.
    let below = Point2D::new(strip.origin.x + 10.0, strip.origin.y + strip.size.y + 40.0);
    let below_surface = device_surface_at(&frame, below, 5.0).expect("inside frame");
    match below_surface {
        PreviewSurface::Scrolled { scroll_y } => assert_eq!(scroll_y, 5.0),
        _ => panic!("expected scrolled surface below the status strip"),
    }
}
