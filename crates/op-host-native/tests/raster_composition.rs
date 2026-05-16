//! Spec v19 acceptance #2 (chrome region) / plan v7 Task 2 Step 16d.
//!
//! Round 5 BLOCK-R5-2 fix: this test covers **chrome only** on a raster
//! surface. The chrome+stub composition lives in
//! `gpu_chrome_stub_composition.rs` because `CanvasViewportStub` needs
//! a real `glow::Context` to mutate GL state, which only the GPU smoke
//! provider has. Here we just prove that `NativeBackend` writes the
//! pixels widgets ask for at the right coordinates — independent of
//! GL state.

use op_editor_ui::{Color, Point2D, Rect};
use op_host_native::NativeBackend;

fn pixel_at(pixels: &[u8], stride: usize, x: i32, y: i32) -> [u8; 4] {
    let offset = (y as usize) * stride + (x as usize) * 4;
    [
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3],
    ]
}

#[test]
fn chrome_rects_paint_at_expected_coordinates() {
    // 500 × 400 raster surface. RGBA8888 / N32 premul matches what
    // `surfaces::wrap_backend_render_target` would set for a GL surface
    // — the per-component byte order is identical on little-endian.
    let mut surface =
        skia_safe::surfaces::raster_n32_premul((500, 400)).expect("raster_n32_premul allocated");

    // Clear to white so chrome paints stand out and "background"
    // assertions know what to expect.
    surface.canvas().clear(skia_safe::Color::WHITE);

    let mut backend = NativeBackend::with_dpi(1.0);
    let canvas = surface.canvas();

    // Chrome rect 1: red, top-left chrome.
    backend.fill_rect(
        canvas,
        Rect {
            origin: Point2D::new(50.0, 50.0),
            size: Point2D::new(100.0, 100.0),
        },
        Color::RED,
    );

    // Chrome rect 2: black, separate region — verifies multi-fill
    // isn't polluted by the first call.
    backend.fill_rect(
        canvas,
        Rect {
            origin: Point2D::new(250.0, 50.0),
            size: Point2D::new(50.0, 50.0),
        },
        Color::BLACK,
    );

    // Read pixels back via `Surface::read_pixels`; raster surfaces
    // expose CPU memory directly so the bytes match what we'd see on
    // a GL surface for the same draw.
    let stride = 500 * 4;
    let mut pixels = vec![0u8; stride * 400];
    let info = skia_safe::ImageInfo::new(
        (500, 400),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    let read_ok = surface.read_pixels(&info, &mut pixels, stride, (0, 0));
    assert!(read_ok, "raster Surface::read_pixels must succeed");

    // (a) red chrome interior — middle of the (50,50)-(150,150) rect.
    assert_eq!(
        pixel_at(&pixels, stride, 75, 75),
        [255, 0, 0, 255],
        "red chrome"
    );

    // (b) black chrome interior — middle of the (250,50)-(300,100) rect.
    assert_eq!(
        pixel_at(&pixels, stride, 275, 75),
        [0, 0, 0, 255],
        "black chrome"
    );

    // (c) untouched background pixel — well outside both rects.
    assert_eq!(
        pixel_at(&pixels, stride, 400, 300),
        [255, 255, 255, 255],
        "background unchanged"
    );
}
