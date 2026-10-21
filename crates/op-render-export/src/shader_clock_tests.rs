//! Exports pin time-driven shader fills to `time = 0`. A live screenshot runs
//! on the UI thread between canvas frames, so without the pin its pixels would
//! depend on the canvas clock at the moment it was taken.

use super::test_support::filled_rect;
use super::*;
use op_editor_ui::layout_scene::{SceneFillType, SceneShader};
use op_editor_ui::widgets::canvas_shader_clock;
use op_editor_ui::{Color, Rect};

fn time_shader_png() -> Vec<u8> {
    let mut node = filled_rect("t", 0.0, 0.0, 8.0, 8.0, Color::WHITE);
    node.fill_type = SceneFillType::Shader;
    node.shader = Some(SceneShader {
        sksl:
            "uniform float time; half4 main(float2 p){ return half4(fract(time), 0.0, 0.0, 1.0); }"
                .to_string(),
        uniforms: Vec::new(),
        opacity: 1.0,
        fallback: Color::WHITE,
    });
    render_raster_bytes(
        Rect::xywh(0.0, 0.0, 8.0, 8.0),
        RasterFormat::Png,
        1.0,
        0.0,
        |canvas| scene_painter::paint_nodes(canvas, std::slice::from_ref(&node)),
    )
    .expect("shader node renders")
}

#[test]
fn a_screenshot_between_canvas_frames_renders_the_shader_at_time_zero() {
    canvas_shader_clock::begin_frame(250);
    let early = time_shader_png();
    canvas_shader_clock::begin_frame(750);
    let late = time_shader_png();
    assert_eq!(
        early, late,
        "the canvas clock leaked into the export: pixels depend on capture time"
    );
    assert!(
        !canvas_shader_clock::animated_shader_painted(),
        "an export must not mark the canvas frame as animated"
    );
}
