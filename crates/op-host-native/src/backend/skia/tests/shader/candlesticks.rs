//! Pixel regressions for integer-boundary jumps and face-wide highlights.

use super::*;
use op_editor_ui::layout_scene::SceneShader;

fn scene_shader(uniforms: serde_json::Value) -> SceneShader {
    let doc = jian_ops_schema::load_str(
        &serde_json::json!({
            "version":"1.0.0","children":[{"type":"rectangle","id":"c",
            "width":192,"height":342,"fill":[{"type":"shader",
            "preset":"candlesticks_3d","uniforms":uniforms}]}]
        })
        .to_string(),
    )
    .unwrap()
    .value;
    op_pen_loader::pen_document_to_layout_scene(&doc, &Default::default(), 0).pages[0].children[0]
        .shader
        .clone()
        .unwrap()
}

fn render(shader: &SceneShader, progress: f32, spacing: f32) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((192, 342)).unwrap();
    let mut backend = NativeBackend::with_dpi(1.0);
    let view = [progress, spacing, 3., 430.];
    let mut uniforms: Vec<_> = shader
        .uniforms
        .iter()
        .filter(|u| u.name != "view")
        .map(|u| (u.name.as_str(), u.values.as_slice()))
        .collect();
    uniforms.push(("view", &view));
    backend.fill_round_rect_shader(
        surface.canvas(),
        Rect::xywh(0., 0., 192., 342.),
        0.,
        &shader.sksl,
        &uniforms,
        1.,
        Color::GREEN,
    );
    let mut pixels = vec![0; 192 * 342 * 4];
    let info = skia_safe::ImageInfo::new(
        (192, 342),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    assert!(surface.read_pixels(&info, &mut pixels, 192 * 4, (0, 0)));
    pixels
}

#[test]
fn advancing_across_a_candle_boundary_does_not_jump_the_history() {
    let shader = scene_shader(serde_json::json!({
        "bars":[100,120,90,110,110,180,100,170,170,190,120,130],
        "sampleTime":0,"lens":[850,0.34,0.2,1]
    }));
    let a = render(&shader, 0.999, 80.);
    let b = render(&shader, 1.001, 80.);
    let mean = a
        .iter()
        .zip(&b)
        .map(|(a, b)| f64::from(a.abs_diff(*b)))
        .sum::<f64>()
        / a.len() as f64;
    assert!(
        mean < 0.15,
        "history jumped at an integer candle index: mean pixel change={mean}"
    );
    let distant = render(&shader, 1.5, 80.);
    let moved = b
        .iter()
        .zip(&distant)
        .filter(|(a, b)| a.abs_diff(**b) > 20)
        .count();
    assert!(moved > 1000, "continuity must not freeze the scene");
}

#[test]
fn flat_front_faces_do_not_receive_the_edge_highlight() {
    let shader = scene_shader(serde_json::json!({
        "bars":[100,225,95,220],"baseline":100,"sampleTime":0,
        "lens":[700,0.34,0.2,1]
    }));
    let pixels = render(&shader, 0., 150.);
    let center = (235 * 192 + 114) * 4;
    let rgb = &pixels[center..center + 3];
    assert!(
        rgb[0] > 140 && rgb[1] < 40 && rgb[2] < 80,
        "the red face should retain its material, not a white edge overlay: {rgb:?}"
    );
}
