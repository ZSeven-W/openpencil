//! Fill payload coverage: authored fills, fill-type switches, linear /
//! mesh / shader gradients and image-fill mode, tile scale and
//! adjustments.

use super::*;

#[test]
fn no_variable_ref_keeps_authored_fill() {
    // Without a registered `$ref`, the node keeps its authored fill.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r1","width":100,"height":50,
         "fill":[{"type":"solid","color":"#112233"}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let fill = scene.pages[0].children[0].fill.expect("authored fill");
    assert!((fill.r - 0.0667).abs() < 0.02);
    assert!((fill.g - 0.1333).abs() < 0.02);
    assert!((fill.b - 0.2).abs() < 0.02);
}

#[test]
fn text_node_carries_content_and_text_style() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"text","id":"hd","content":"Welcome Back",
         "fontSize":28,"fontWeight":700,
         "fill":[{"type":"solid","color":"#0F172A"}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    assert_eq!(n.text.as_deref(), Some("Welcome Back"));
    assert_eq!(n.font_size, 28.0);
    assert_eq!(n.font_weight, 700);
}

#[test]
fn property_panel_fill_type_switch_produces_scene_gradient() {
    // Reproduce what the property-panel does on
    // "Solid → LinearGradient": mutate `EditorState.doc` via the
    // same code path the host calls, then re-build the scene and
    // assert that `SceneNode.gradient` is populated. Catches the
    // case where the live mutation path bypasses the scene's
    // gradient field — e.g. if the adapter only reads .op files
    // and not in-memory `PenDocument` mutations.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":100,"height":50,
        "fill":[{"type":"solid","color":"#FF0000"}]
      }]}],"children":[]
    }"##;
    let mut state = state_from(src);
    let node_id = op_editor_core::NodeId::new("r");
    state.selection.set = vec![node_id.clone()];
    state.selection.anchor = node_id;
    assert!(state.set_selected_fill_type(op_editor_core::FillType::LinearGradient));
    let scene = editor_state_to_layout_scene(&state);
    let n = &scene.pages[0].children[0];
    assert!(
        n.gradient.is_some(),
        "scene gradient must populate after Solid → LinearGradient toggle; \
         live-mutated PenDocument is being silently dropped"
    );
}

#[test]
fn linear_gradient_payload_threads_into_scene_node() {
    // A LinearGradient first-fill must come out of the scene builder
    // as `SceneNode.gradient = Some(Linear { ... })` — otherwise the
    // canvas painter falls back to the first-stop solid and the
    // user's "switch fill to LinearGradient" intent doesn't render.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":100,"height":50,
        "fill":[{"type":"linear_gradient","angle":90,
                 "stops":[{"color":"#FF0000","offset":0},
                          {"color":"#0000FF","offset":1}]}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    let g = n.gradient.as_ref().expect("scene gradient must populate");
    match g {
        jian_scene::layout_scene::SceneGradient::Linear { stops, .. } => {
            assert_eq!(stops.len(), 2);
            assert!(stops[0].color.r > 0.99 && stops[0].color.b < 0.01);
        }
        other => panic!("expected linear, got {other:?}"),
    }
}

#[test]
fn mesh_gradient_payload_threads_into_scene_node_with_first_vertex_fallback() {
    // A mesh_gradient first-fill must come out of the scene builder as
    // `SceneNode.gradient = Some(Mesh { .. })` (so the native
    // draw_vertices path renders the Gouraud blend) AND populate
    // `SceneNode.fill` with the first-vertex colour as the documented
    // solid fallback (backends without per-vertex support paint flat).
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":100,"height":100,
        "fill":[{"type":"mesh_gradient","rows":2,"cols":2,
                 "stops":[{"row":0,"col":0,"color":"#FF0000"},
                          {"row":0,"col":1,"color":"#00FF00"},
                          {"row":1,"col":0,"color":"#0000FF"},
                          {"row":1,"col":1,"color":"#FFFF00"}]}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    let g = n.gradient.as_ref().expect("scene gradient must populate");
    match g {
        jian_scene::layout_scene::SceneGradient::Mesh {
            rows, cols, colors, ..
        } => {
            assert_eq!(*rows, 2);
            assert_eq!(*cols, 2);
            assert_eq!(colors.len(), 4);
            // Row-major: (0,0) red, (0,1) green, (1,0) blue, (1,1) yellow.
            assert!(colors[0].r > 0.99 && colors[0].g < 0.01 && colors[0].b < 0.01);
            assert!(colors[1].g > 0.99 && colors[1].r < 0.01);
            assert!(colors[2].b > 0.99 && colors[2].r < 0.01);
        }
        other => panic!("expected mesh, got {other:?}"),
    }
    // First-vertex solid fallback baked into node.fill.
    let fill = n.fill.expect("mesh fill must carry first-vertex fallback");
    assert!(fill.r > 0.99 && fill.g < 0.01 && fill.b < 0.01);
}

#[test]
fn shader_payload_threads_into_scene_node_with_uniforms_and_fallback() {
    // A `shader` first-fill must come out of the scene builder as
    // `SceneNode.shader = Some(..)` (so the native RuntimeEffect path
    // runs the program) AND bake the first colour-uniform into
    // `SceneNode.fill` as the documented solid fallback.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":240,"height":240,
        "fill":[{"type":"shader",
                 "sksl":"half4 main(float2 p){ return half4(p.x/240.0, p.y/240.0, 0.5, 1.0); }",
                 "uniforms":{"glow":0.5,"tint":"#FF0000"}}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    let s = n.shader.as_ref().expect("scene shader must populate");
    assert!(s.sksl.contains("half4 main(float2 p)"));
    // `glow` (float) + `tint` (color → premultiplied vec4) bind through.
    let glow = s.uniforms.iter().find(|u| u.name == "glow").expect("glow");
    assert_eq!(glow.values, vec![0.5]);
    let tint = s.uniforms.iter().find(|u| u.name == "tint").expect("tint");
    assert_eq!(tint.values.len(), 4);
    // Premultiplied opaque red → (1,0,0,1).
    assert!(tint.values[0] > 0.99 && tint.values[1] < 0.01 && tint.values[3] > 0.99);
    // fill_type is "shader".
    assert_eq!(n.fill_type, jian_scene::layout_scene::SceneFillType::Shader);
    // First colour-uniform solid fallback baked into node.fill (red).
    let fill = n
        .fill
        .expect("shader fill must carry colour-uniform fallback");
    assert!(fill.r > 0.99 && fill.g < 0.01 && fill.b < 0.01);
    // Fallback colour on the scene shader itself is red too.
    assert!(s.fallback.r > 0.99 && s.fallback.g < 0.01);
}

#[test]
fn shader_payload_without_color_uniform_falls_back_mid_gray() {
    // A shader with no colour uniform must still bake a visible fallback
    // (mid-gray) into both node.fill and the scene shader, so a host
    // that can't compile the program shows something.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":100,"height":100,
        "fill":[{"type":"shader",
                 "sksl":"half4 main(float2 p){ return half4(0.0,0.0,0.0,1.0); }"}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    let s = n.shader.as_ref().expect("scene shader must populate");
    assert!(s.uniforms.is_empty());
    // Mid-gray fallback (0.5, 0.5, 0.5).
    assert!((s.fallback.r - 0.5).abs() < 0.02 && (s.fallback.g - 0.5).abs() < 0.02);
    let fill = n.fill.expect("shader fill must carry mid-gray fallback");
    assert!((fill.r - 0.5).abs() < 0.02);
}

#[test]
fn image_fill_mode_threads_into_scene_node() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":360,"height":240,
        "fill":[{"type":"image","url":"data:image/png;base64,AA==","mode":"tile",
          "originalSize":{"width":4096,"height":2048},"tileScale":0.38618907}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    assert_eq!(n.image_src.as_deref(), Some("data:image/png;base64,AA=="));
    assert_eq!(n.image_fit, jian_scene::layout_scene::SceneImageFit::Tile);
    assert_eq!(n.image_original_size, Some([4096.0, 2048.0]));
    assert_eq!(n.image_tile_scale, 0.38618907);
    assert!(matches!(
        n.fill_layers.as_slice(),
        [SceneFillLayer::Image {
            fit: SceneImageFit::Tile,
            original_size: Some([4096.0, 2048.0]),
            tile_scale,
            ..
        }] if *tile_scale == 0.38618907
    ));
}

#[test]
fn image_video_metadata_threads_into_scene_node() {
    let src = r#"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"image","id":"hero","width":360,"height":240,
        "src":"data:image/png;base64,AA==",
        "video":{"src":"https://cdn.example.com/hero.mp4","autoplay":true,
          "loop":true,"muted":true,"holdLastFrame":true,"clickToReplay":true}
      }]}],"children":[]
    }"#;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let video = scene.pages[0].children[0]
        .video
        .as_ref()
        .expect("video metadata reaches the scene");
    assert_eq!(video.src.as_ref(), "https://cdn.example.com/hero.mp4");
    assert!(video.autoplay && video.r#loop && video.muted);
    assert!(video.hold_last_frame && video.click_to_replay);
}

#[test]
fn invalid_or_missing_tile_scale_defaults_to_one_in_scene() {
    for tile_scale in ["", ",\"tileScale\":0", ",\"tileScale\":-2"] {
        let src = format!(
            r#"{{
              "version":"1.0.0","pages":[{{"id":"p","name":"P","children":[{{
                "type":"rectangle","id":"r","width":220,"height":220,
                "fill":[{{"type":"image","url":"data:image/png;base64,AA==","mode":"tile"{tile_scale}}}]
              }}]}}],"children":[]
            }}"#
        );
        let scene = editor_state_to_layout_scene(&state_from(&src));
        let node = &scene.pages[0].children[0];
        assert_eq!(node.image_tile_scale, 1.0);
        assert!(matches!(
            node.fill_layers.as_slice(),
            [SceneFillLayer::Image { tile_scale, .. }] if *tile_scale == 1.0
        ));
    }
}

#[test]
fn image_fill_adjustments_thread_into_scene_node() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":360,"height":240,
        "fill":[{"type":"image","url":"data:image/png;base64,AA==",
          "mode":"fit","exposure":100,"contrast":-100,"saturation":50,
          "temperature":25,"tint":-25,"highlights":75,"shadows":-75}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let a = scene.pages[0].children[0].image_adjustments;
    assert_eq!(a.exposure, 100.0);
    assert_eq!(a.contrast, -100.0);
    assert_eq!(a.saturation, 50.0);
    assert_eq!(a.temperature, 25.0);
    assert_eq!(a.tint, -25.0);
    assert_eq!(a.highlights, 75.0);
    assert_eq!(a.shadows, -75.0);
}
