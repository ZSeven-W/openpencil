//! Backend-dispatch tests — shader and mesh-gradient fills reach their dedicated `RenderBackend` methods.
//!
//! Split out of `canvas_viewport_tests.rs` to keep every file under
//! the repository's 800-line cap. Shared fixtures (`RecordingBackend`,
//! scene builders, transform-replay helpers) stay in that spine.

use super::*;

#[test]
fn shader_fill_dispatches_through_the_shader_backend_method() {
    use crate::layout_scene::{SceneShader, SceneShaderUniform};
    let mut node = SceneNode::leaf("s1", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    node.fill = Some(Color::WHITE); // fallback colour the builder bakes in
    node.fill_type = SceneFillType::Shader;
    node.shader = Some(SceneShader {
        sksl: "half4 main(float2 p){ return half4(1.0,0.0,0.0,1.0); }".to_string(),
        uniforms: vec![SceneShaderUniform {
            name: "u_mix".to_string(),
            values: vec![0.5],
        }],
        opacity: 1.0,
        fallback: Color::WHITE,
    });

    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        crate::widgets::canvas_viewport_overlay::paint_fill_then_stroke(
            &mut cx,
            &node,
            node.bounds,
            1.0,
            node.fill,
        );
    }
    assert_eq!(
        backend.shader_fills, 1,
        "shader body must route through fill_round_rect_shader, not the solid fill"
    );
    assert_eq!(
        backend.rects, 0,
        "no plain-fill fallback when a shader is present"
    );
}

#[test]
fn mesh_gradient_dispatches_through_the_mesh_backend_method() {
    use crate::layout_scene::SceneGradient;
    let mut node = SceneNode::leaf("m1", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    node.fill = Some(Color::WHITE);
    node.fill_type = SceneFillType::MeshGradient;
    node.gradient = Some(SceneGradient::Mesh {
        rows: 2,
        cols: 2,
        colors: vec![Color::WHITE; 4],
        opacity: 1.0,
    });

    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        crate::widgets::canvas_viewport_overlay::paint_fill_then_stroke(
            &mut cx,
            &node,
            node.bounds,
            1.0,
            node.fill,
        );
    }
    assert_eq!(
        backend.mesh_fills, 1,
        "mesh body must route through fill_round_rect_mesh_gradient"
    );
    assert_eq!(
        backend.rects, 0,
        "no flat first-vertex fill at the dispatch layer"
    );
}

#[test]
fn a_shader_declaring_time_receives_the_frame_clock_over_an_authored_value() {
    use crate::layout_scene::{SceneShader, SceneShaderUniform};
    let mut node = SceneNode::leaf("t1", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    node.fill = Some(Color::WHITE);
    node.fill_type = SceneFillType::Shader;
    node.shader = Some(SceneShader {
        sksl: "uniform float time; uniform half4 tint; half4 main(float2 p){ return tint; }"
            .to_string(),
        uniforms: vec![
            SceneShaderUniform {
                name: "time".to_string(),
                values: vec![99.0],
            },
            SceneShaderUniform {
                name: "tint".to_string(),
                values: vec![1.0, 0.0, 0.0, 1.0],
            },
        ],
        opacity: 1.0,
        fallback: Color::WHITE,
    });

    crate::widgets::canvas_shader_clock::begin_frame(1_500);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        crate::widgets::canvas_viewport_overlay::paint_fill_then_stroke(
            &mut cx,
            &node,
            node.bounds,
            1.0,
            node.fill,
        );
    }
    assert_eq!(
        backend.shader_uniforms,
        vec![vec![
            ("tint".to_string(), vec![1.0, 0.0, 0.0, 1.0]),
            ("time".to_string(), vec![1.5]),
        ]],
        "the frame clock replaces the authored `time` and is bound exactly once"
    );
    assert!(crate::widgets::canvas_shader_clock::animated_shader_painted());
}
