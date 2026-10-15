//! Opt-in visual evidence for saved vector prototypes after real Play taps.
//! This uses the native painter on a raster surface, not native-window input.

#![cfg(all(feature = "gl-host", not(target_os = "windows")))]

use op_editor_ui::{Point2D, Rect};
use op_host_native::{NativeBackend, NativeFrameBackend, PreviewSession};
use op_preview_core::{device_frame::compute_frame_geometry, PinnedPaint};
use std::{collections::BTreeMap, path::Path, rc::Rc};

fn capture(session: &PreviewSession, path: &Path, now: u64) -> Vec<u8> {
    let (root_id, bounds) = session.framed_root().unwrap();
    let canvas = Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(480.0, 960.0),
    };
    let nav = session.pinned_nav_candidate(true);
    let status = session.pinned_status_bar_candidate(true);
    let geometry = compute_frame_geometry(
        op_editor_core::PreviewDeviceKind::Phone,
        canvas,
        bounds,
        nav.as_ref().map(|(_, rect)| *rect),
        status.as_ref().map(|(_, rect)| *rect),
    );
    let pinned = geometry
        .pinned
        .as_ref()
        .zip(nav.as_ref())
        .map(|(geom, (id, _))| PinnedPaint {
            node_id: id.clone(),
            strip_clip: geom.strip,
            paint_origin: geom.paint_origin,
            nav_scene_origin: geom.node_scene.origin,
        });
    let pinned_top = geometry
        .pinned_top
        .as_ref()
        .zip(status.as_ref())
        .map(|(geom, (id, _))| PinnedPaint {
            node_id: id.clone(),
            strip_clip: geom.strip,
            paint_origin: geom.paint_origin,
            nav_scene_origin: geom.node_scene.origin,
        });
    let top = pinned_top.as_ref().map_or(0.0, |p| p.strip_clip.size.y);
    let bottom = pinned.as_ref().map_or(0.0, |p| p.strip_clip.size.y);
    let mut surface = skia_safe::surfaces::raster_n32_premul((480, 960)).unwrap();
    let mut backend = NativeBackend::with_dpi(1.0);
    let mut frame = NativeFrameBackend::new(&mut backend, surface.canvas());
    session.paint_framed(
        &mut frame,
        &root_id,
        Rect {
            origin: Point2D::new(geometry.frame.origin.x, geometry.frame.origin.y + top),
            size: Point2D::new(geometry.frame.size.x, geometry.frame.size.y - top - bottom),
        },
        geometry.content_origin,
        geometry.fit,
        pinned.as_ref(),
        pinned_top.as_ref(),
        now,
    );
    let data = surface
        .image_snapshot()
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .unwrap();
    let bytes = data.as_bytes().to_vec();
    std::fs::write(path, &bytes).unwrap();
    bytes
}

#[test]
#[ignore = "requires OPENPENCIL_PREVIEW_AUDIT_INPUT, OPENPENCIL_PREVIEW_TAP_IDS and OPENPENCIL_PREVIEW_SHOT_DIR"]
fn capture_saved_vector_preview_controls() {
    let input = std::env::var("OPENPENCIL_PREVIEW_AUDIT_INPUT").unwrap();
    let dir = std::env::var("OPENPENCIL_PREVIEW_SHOT_DIR").unwrap();
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    let ids: Vec<String> =
        serde_json::from_str(&std::env::var("OPENPENCIL_PREVIEW_TAP_IDS").unwrap()).unwrap();
    assert!(!ids.is_empty());
    let source = std::fs::read_to_string(&input).unwrap();
    let doc = jian_ops_schema::load_str(&source).unwrap().value;
    for id in ids {
        let mut session = PreviewSession::enter(
            &doc,
            (1440.0, 1000.0),
            &BTreeMap::new(),
            0,
            false,
            false,
            Rc::new(jian_skia::SkiaMeasure::new()),
            0,
        )
        .unwrap();
        let initial = capture(&session, &dir.join(format!("{id}-initial.png")), 0);
        for attempt in 0..2 {
            let scene = session.preview_scene_for_test();
            let node = scene.active_page().unwrap().find(&id).unwrap();
            let rect = node.bounds;
            let now = 100 + attempt * 600;
            session.set_now_ms(now);
            assert!(session.dispatch_tap(
                rect.origin.x + rect.size.x / 2.0,
                rect.origin.y + rect.size.y / 2.0,
            ));
            session.pump(now + 500);
            let after = capture(
                &session,
                &dir.join(format!("{id}-tap{}.png", attempt + 1)),
                now + 500,
            );
            if attempt == 0 {
                assert_ne!(initial, after, "tap must change painted pixels: {id}");
            }
        }
    }
    assert_eq!(source, std::fs::read_to_string(input).unwrap());
}
