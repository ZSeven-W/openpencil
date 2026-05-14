//! Custom rotate cursor bitmap. Lives in its own module so the
//! main winit event-loop file (`main.rs`) stays under the 800-line
//! cap that all OP modules honor.

/// Build a 32 × 32 RGBA rotate-cursor bitmap by rendering the
/// lucide `RotateCw` icon through skia's stroke-path pipeline —
/// proper Skia AA + round caps/joins, matches the rest of the
/// icon set's visual language. Two passes: white halo for
/// legibility on any background, then a dark core on top.
pub(crate) fn make_rotate_cursor_rgba() -> (Vec<u8>, u16, u16, u16, u16) {
    const SIZE: i32 = 16;
    // Lucide rotate-cw d-strings (one per <path> element).
    const LUCIDE_PATHS: &[&str] = &[
        "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8",
        "M21 3v5h-5",
    ];

    // Off-screen raster surface; skia handles all AA + stroking.
    let mut surface = skia_safe::surfaces::raster_n32_premul((SIZE, SIZE))
        .expect("raster_n32_premul should produce a CPU surface for cursor");
    {
        let canvas = surface.canvas();
        canvas.clear(skia_safe::Color::TRANSPARENT);
        let scale = (SIZE as f32) / 24.0; // lucide viewBox is 24 × 24
        let mut paths = Vec::with_capacity(LUCIDE_PATHS.len());
        for d in LUCIDE_PATHS {
            if let Some(path) = skia_safe::utils::parse_path::from_svg(d) {
                let mut m = skia_safe::Matrix::new_identity();
                m.set_scale((scale, scale), None);
                paths.push(path.with_transform(&m));
            }
        }

        // Halo (white, slightly wider).
        let mut halo = skia_safe::Paint::new(skia_safe::Color4f::new(1.0, 1.0, 1.0, 1.0), None);
        halo.set_anti_alias(true);
        halo.set_stroke(true);
        halo.set_stroke_width(2.4);
        halo.set_stroke_cap(skia_safe::PaintCap::Round);
        halo.set_stroke_join(skia_safe::PaintJoin::Round);
        for p in &paths {
            canvas.draw_path(p, &halo);
        }

        // Core stroke — near-black for contrast.
        let mut core = skia_safe::Paint::new(skia_safe::Color4f::new(0.06, 0.06, 0.06, 1.0), None);
        core.set_anti_alias(true);
        core.set_stroke(true);
        core.set_stroke_width(1.2);
        core.set_stroke_cap(skia_safe::PaintCap::Round);
        core.set_stroke_join(skia_safe::PaintJoin::Round);
        for p in &paths {
            canvas.draw_path(p, &core);
        }
    }

    let row_bytes = (SIZE as usize) * 4;
    let mut bgra = vec![0u8; row_bytes * (SIZE as usize)];
    let info = skia_safe::ImageInfo::new(
        (SIZE, SIZE),
        skia_safe::ColorType::BGRA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    surface.read_pixels(&info, &mut bgra, row_bytes, (0, 0));
    // winit wants straight RGBA — swap BGRA → RGBA in place.
    let mut rgba = bgra;
    for px in rgba.chunks_exact_mut(4) {
        px.swap(0, 2);
    }

    let hotspot = (SIZE as u16) / 2;
    (rgba, SIZE as u16, SIZE as u16, hotspot, hotspot)
}
