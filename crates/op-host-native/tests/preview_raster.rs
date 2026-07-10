#![cfg(feature = "gl-host")]
//! Pixel-level proof that Canvas Preview (Play) renders through the
//! design-canvas scene painter — the "orange renders grey / layout
//! scattered" bug, verified end-to-end on a real raster surface.
//!
//! Unit tests assert the overlaid scene DATA; this test paints the
//! preview for real (`PreviewSession::paint_scene` → `NativeFrameBackend`
//! → skia raster) and reads the pixels back, so it catches paint-path
//! regressions the data tests can't (wrong colour, scattered layout,
//! grey-placeholder fallback).

use op_editor_ui::{Point2D, Rect};
use op_host_native::{NativeBackend, NativeFrameBackend, PreviewSession};

fn pixel_at(pixels: &[u8], stride: usize, x: i32, y: i32) -> [u8; 4] {
    let offset = (y as usize) * stride + (x as usize) * 4;
    [
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3],
    ]
}

/// `|a - b| <= tol` per channel (raster premul + any edge blending).
fn near(actual: [u8; 4], expect: [u8; 4], tol: i32) -> bool {
    actual
        .iter()
        .zip(expect.iter())
        .all(|(a, b)| (*a as i32 - *b as i32).abs() <= tol)
}

#[test]
fn preview_paints_resolved_orange_at_the_swatch() {
    // A 100×100 swatch filled via the `$color-brand` (#ff8800) ref,
    // top-left inside a 200×200 screen frame with NO fill. The old jian
    // MVP walker dropped the `$ref` to grey; the design-canvas renderer
    // resolves it. The screen frame stays unpainted, so a pixel outside
    // the swatch must remain the cleared background — guarding against
    // the "scattered / fill-everything" failure mode.
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "variables": { "color-brand": { "type": "color", "value": "#ff8800" } },
        "children": [
            {
                "type": "frame", "id": "screen", "width": 200, "height": 200,
                "children": [
                    {
                        "type": "rectangle", "id": "swatch",
                        "x": 0, "y": 0, "width": 100, "height": 100,
                        "fill": [{ "type": "solid", "color": "$color-brand" }]
                    }
                ]
            }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src)
        .expect("parse var-color doc")
        .value;
    let session = PreviewSession::enter(&doc, (200.0, 200.0), &Default::default(), 0, false)
        .expect("enter preview");

    // Raster surface cleared to a sentinel BLUE so "unpainted" pixels
    // are unmistakable (white would collide with common widget fills).
    let mut surface = skia_safe::surfaces::raster_n32_premul((200, 200)).expect("raster surface");
    surface
        .canvas()
        .clear(skia_safe::Color::from_argb(255, 0, 0, 255));

    {
        let mut inner = NativeBackend::with_dpi(1.0);
        let canvas = surface.canvas();
        let mut frame = NativeFrameBackend::new(&mut inner, canvas);
        session.paint_scene(
            &mut frame,
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(200.0, 200.0),
            },
            (0.0, 0.0),
            1.0,
            0,
        );
    }

    let stride = 200 * 4;
    let mut pixels = vec![0u8; stride * 200];
    let info = skia_safe::ImageInfo::new(
        (200, 200),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    assert!(
        surface.read_pixels(&info, &mut pixels, stride, (0, 0)),
        "raster read_pixels must succeed"
    );

    // (a) swatch interior (25,25) — resolved orange #ff8800, NOT grey.
    let swatch = pixel_at(&pixels, stride, 25, 25);
    assert!(
        near(swatch, [0xff, 0x88, 0x00, 0xff], 4),
        "swatch should paint resolved orange #ff8800, got {swatch:?} \
         (a grey/dropped fill would be the regression)"
    );

    // (b) inside the screen frame but OUTSIDE the swatch (150,150) — the
    // unfilled frame must leave the sentinel background, proving the
    // swatch is bounded to 100×100 (not scattered / fill-everything).
    let bg = pixel_at(&pixels, stride, 150, 150);
    assert!(
        near(bg, [0, 0, 255, 255], 4),
        "area outside the 100×100 swatch should stay background, got {bg:?}"
    );
}
