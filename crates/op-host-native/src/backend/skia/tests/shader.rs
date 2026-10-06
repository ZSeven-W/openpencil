//! Real Skia RuntimeEffect coverage for loader-expanded shader presets.

use super::*;

fn turbulence_shader() -> op_editor_ui::layout_scene::SceneShader {
    let source = r#"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":64,"height":64,
        "fill":[{"type":"shader","preset":"turbulence",
          "sksl":"AUTHOR_SOURCE_MUST_LOSE",
          "uniforms":{"baseFrequency":[0.08,0.11],"seed":7,"numOctaves":3}}]
      }]}],"children":[]
    }"#;
    let document = jian_ops_schema::load_str(source)
        .expect("preset document parses")
        .value;
    let scene = op_pen_loader::pen_document_to_layout_scene(
        &document,
        &std::collections::BTreeMap::new(),
        0,
    );
    scene.pages[0].children[0]
        .shader
        .clone()
        .expect("loader expands the preset")
}

fn raster_bytes(
    width: i32,
    height: i32,
    paint: impl FnOnce(&mut NativeBackend, &skia_safe::Canvas),
) -> Vec<u8> {
    let mut surface =
        skia_safe::surfaces::raster_n32_premul((width, height)).expect("raster surface");
    let mut backend = NativeBackend::with_dpi(1.0);
    paint(&mut backend, surface.canvas());
    surface
        .image_snapshot()
        .peek_pixels()
        .expect("raster pixels")
        .bytes()
        .expect("pixel bytes")
        .to_vec()
}

#[test]
fn turbulence_preset_source_compiles_in_real_skia_runtime_effect() {
    let shader = turbulence_shader();
    assert!(!shader.sksl.contains("AUTHOR_SOURCE_MUST_LOSE"));
    skia_safe::RuntimeEffect::make_for_shader(&shader.sksl, None)
        .unwrap_or_else(|error| panic!("expanded turbulence SkSL must compile: {error}"));

    let uniforms: Vec<(&str, &[f32])> = shader
        .uniforms
        .iter()
        .map(|uniform| (uniform.name.as_str(), uniform.values.as_slice()))
        .collect();
    let pixels = raster_bytes(64, 64, |backend, canvas| {
        backend.fill_round_rect_shader(
            canvas,
            Rect::xywh(0.0, 0.0, 64.0, 64.0),
            0.0,
            &shader.sksl,
            &uniforms,
            1.0,
            Color::GREEN,
        );
    });
    let (minimum, maximum) = pixels
        .chunks_exact(4)
        .map(|pixel| pixel[0])
        .fold((u8::MAX, u8::MIN), |(minimum, maximum), value| {
            (minimum.min(value), maximum.max(value))
        });
    assert!(
        maximum.saturating_sub(minimum) > 12,
        "compiled turbulence should vary across the raster, got {minimum}..={maximum}"
    );
}

fn translated_shader_crop(origin: Point2D, rendered_side: usize) -> Vec<u8> {
    const DOCUMENT_SIDE: usize = 32;
    const SURFACE_SIDE: usize = 160;
    let size = [DOCUMENT_SIDE as f32, DOCUMENT_SIDE as f32];
    let uniforms = [("size", size.as_slice())];
    let pixels = raster_bytes(
        SURFACE_SIDE as i32,
        SURFACE_SIDE as i32,
        |backend, canvas| {
            backend.fill_round_rect_shader(
                canvas,
                Rect::xywh(
                    origin.x,
                    origin.y,
                    rendered_side as f32,
                    rendered_side as f32,
                ),
                0.0,
                "uniform float2 size; half4 main(float2 p) { float2 cell = floor(p); float v = fract(sin(dot(cell, float2(12.9898, 78.233))) * 43758.5453); return half4(v, v, v, 1.0); }",
                &uniforms,
                1.0,
                Color::RED,
            );
        },
    );
    let mut crop = Vec::with_capacity(rendered_side * rendered_side * 4);
    let left = origin.x as usize;
    let top = origin.y as usize;
    for y in top..top + rendered_side {
        let start = (y * SURFACE_SIDE + left) * 4;
        crop.extend_from_slice(&pixels[start..start + rendered_side * 4]);
    }
    crop
}

#[test]
fn shader_pixels_stay_node_local_across_viewport_translation() {
    // The shape painter bakes viewport pan into the world-space rect origin.
    // Exercise the same 32×32 node at 1× and 2× viewport zoom, each under two
    // translations. This covers both halves of the scale+translate matrix.
    for rendered_side in [32, 64] {
        let first = translated_shader_crop(Point2D::new(5.0, 7.0), rendered_side);
        let second = translated_shader_crop(Point2D::new(80.0, 80.0), rendered_side);
        let (minimum, maximum) = first
            .chunks_exact(4)
            .map(|pixel| pixel[0])
            .fold((u8::MAX, u8::MIN), |(minimum, maximum), value| {
                (minimum.min(value), maximum.max(value))
            });
        assert!(
            maximum.saturating_sub(minimum) > 12,
            "translation probe must contain shader variation, not a flat fallback"
        );
        assert_eq!(
            first, second,
            "RuntimeEffect coordinates must stay node-local at rendered side {rendered_side}"
        );
    }

    let zoom_one = translated_shader_crop(Point2D::new(5.0, 7.0), 32);
    let zoom_two = translated_shader_crop(Point2D::new(5.0, 7.0), 64);
    for y in 1..31 {
        for x in 1..31 {
            let document_pixel = &zoom_one[(y * 32 + x) * 4..(y * 32 + x + 1) * 4];
            for (zoom_x, zoom_y) in [
                (x * 2, y * 2),
                (x * 2 + 1, y * 2),
                (x * 2, y * 2 + 1),
                (x * 2 + 1, y * 2 + 1),
            ] {
                let start = (zoom_y * 64 + zoom_x) * 4;
                assert_eq!(
                    document_pixel,
                    &zoom_two[start..start + 4],
                    "2× zoom must preserve document-local cell ({x}, {y})"
                );
            }
        }
    }
}
