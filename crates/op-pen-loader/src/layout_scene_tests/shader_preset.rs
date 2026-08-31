//! Loader expansion coverage for schema-level shader presets.

use super::*;

fn shader_fixture(shader_body: &str, width: f32, height: f32) -> String {
    format!(
        r#"{{
          "version":"1.0.0","pages":[{{"id":"p","name":"P","children":[{{
            "type":"rectangle","id":"r","width":{width},"height":{height},
            "fill":[{{"type":"shader",{shader_body}}}]
          }}]}}],"children":[]
        }}"#
    )
}

fn uniform<'a>(shader: &'a SceneShader, name: &str) -> &'a [f32] {
    shader
        .uniforms
        .iter()
        .find(|uniform| uniform.name == name)
        .unwrap_or_else(|| panic!("missing `{name}` uniform in {:?}", shader.uniforms))
        .values
        .as_slice()
}

fn octave_call_count(sksl: &str) -> usize {
    sksl.matches("value_noise(").count().saturating_sub(1)
}

#[test]
fn turbulence_preset_expands_with_defaults_and_resolved_node_size() {
    let source = shader_fixture(r#""preset":"turbulence""#, 240.0, 120.0);
    let state = state_from(&source);
    let persisted = serde_json::to_value(&state.doc).expect("document serializes");
    let persisted_fill = &persisted["pages"][0]["children"][0]["fill"][0];
    assert_eq!(persisted_fill["preset"], "turbulence");
    assert!(
        persisted_fill.get("sksl").is_none(),
        "expanded source must never be written back into the document: {persisted_fill}"
    );

    let scene = editor_state_to_layout_scene(&state);
    let node = &scene.pages[0].children[0];
    let shader = node.shader.as_ref().expect("preset expands into a shader");
    assert_eq!(
        octave_call_count(&shader.sksl),
        3,
        "default is three octaves"
    );
    assert!(!shader.sksl.contains("for ("));
    assert!(!shader.sksl.contains("for("));
    assert_eq!(uniform(shader, "baseFrequency"), [0.08, 0.08]);
    assert_eq!(uniform(shader, "seed"), [0.0]);
    assert_eq!(uniform(shader, "size"), [240.0, 120.0]);

    let layered = node
        .fill_layers
        .iter()
        .find_map(|layer| match layer {
            SceneFillLayer::Shader { shader, .. } => Some(shader),
            _ => None,
        })
        .expect("canonical fill stack carries the expanded shader");
    assert_eq!(layered.sksl, shader.sksl);
    assert_eq!(layered.uniforms, shader.uniforms);
}

#[test]
fn turbulence_preset_wins_and_bakes_clamped_octaves_into_distinct_sources() {
    let one = shader_fixture(
        r#""preset":"turbulence","sksl":"AUTHOR_SOURCE_MUST_LOSE","uniforms":{"numOctaves":0}"#,
        64.0,
        64.0,
    );
    let six = shader_fixture(
        r#""preset":"turbulence","sksl":"AUTHOR_SOURCE_MUST_LOSE","uniforms":{"numOctaves":99}"#,
        64.0,
        64.0,
    );
    let one_scene = editor_state_to_layout_scene(&state_from(&one));
    let six_scene = editor_state_to_layout_scene(&state_from(&six));
    let one_shader = one_scene.pages[0].children[0]
        .shader
        .as_ref()
        .expect("one-octave shader");
    let six_shader = six_scene.pages[0].children[0]
        .shader
        .as_ref()
        .expect("six-octave shader");

    assert!(!one_shader.sksl.contains("AUTHOR_SOURCE_MUST_LOSE"));
    assert!(!six_shader.sksl.contains("AUTHOR_SOURCE_MUST_LOSE"));
    assert_eq!(octave_call_count(&one_shader.sksl), 1);
    assert_eq!(octave_call_count(&six_shader.sksl), 6);
    assert_ne!(one_shader.sksl, six_shader.sksl);
    assert!(
        one_shader
            .uniforms
            .iter()
            .all(|uniform| uniform.name != "numOctaves"),
        "numOctaves is compile-time only"
    );
}

#[test]
fn turbulence_parameters_reuse_scalar_and_vector_uniform_shapes() {
    let scalar = shader_fixture(
        r#""preset":"turbulence","uniforms":{"baseFrequency":0.125,"seed":9}"#,
        80.0,
        40.0,
    );
    let vector = shader_fixture(
        r#""preset":"turbulence","uniforms":{"baseFrequency":[0.04,0.09],"seed":2}"#,
        80.0,
        40.0,
    );
    let scalar_scene = editor_state_to_layout_scene(&state_from(&scalar));
    let vector_scene = editor_state_to_layout_scene(&state_from(&vector));
    let scalar_shader = scalar_scene.pages[0].children[0]
        .shader
        .as_ref()
        .expect("scalar-frequency shader");
    let vector_shader = vector_scene.pages[0].children[0]
        .shader
        .as_ref()
        .expect("vector-frequency shader");

    assert_eq!(uniform(scalar_shader, "baseFrequency"), [0.125, 0.125]);
    assert_eq!(uniform(scalar_shader, "seed"), [9.0]);
    assert_eq!(uniform(vector_shader, "baseFrequency"), [0.04, 0.09]);
    assert_eq!(uniform(vector_shader, "seed"), [2.0]);
}

#[test]
fn unknown_preset_is_ignored_and_authored_sksl_remains_the_fallback() {
    let source = shader_fixture(
        r#""preset":"future_noise","sksl":"half4 main(float2 p){ return half4(1.0); }""#,
        32.0,
        32.0,
    );
    let scene = editor_state_to_layout_scene(&state_from(&source));
    let shader = scene.pages[0].children[0]
        .shader
        .as_ref()
        .expect("unknown preset falls back to authored source");
    assert_eq!(shader.sksl, "half4 main(float2 p){ return half4(1.0); }");
}
