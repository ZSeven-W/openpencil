//! Built-in shader-fill preset expansion.
//!
//! Presets are a loader-only convenience: canonical documents retain the
//! compact preset name and parameters, while the render payload receives a
//! concrete SkSL program plus runtime uniforms.

use jian_ops_schema::style::{ShaderFillBody, ShaderUniformValue};
#[cfg(test)]
use op_util::shader_preset::MAX_NUM_OCTAVES;
use op_util::shader_preset::{resolve_num_octaves, DEFAULT_NUM_OCTAVES};

use crate::payload::ShaderUniformPayload;

const DEFAULT_BASE_FREQUENCY: f32 = 0.08;
const DEFAULT_SEED: f32 = 0.0;

pub(crate) struct ExpandedShaderPreset {
    pub(crate) sksl: String,
    pub(crate) uniforms: Vec<ShaderUniformPayload>,
}

/// Expand an exactly-recognized shader preset. Unknown names intentionally
/// return `None` so callers can fall back to authored SkSL.
pub(crate) fn expand(body: &ShaderFillBody) -> Option<ExpandedShaderPreset> {
    match body.preset.as_deref() {
        Some("turbulence") => Some(expand_turbulence(body)),
        _ => None,
    }
}

fn expand_turbulence(body: &ShaderFillBody) -> ExpandedShaderPreset {
    let base_frequency = match uniform(body, "baseFrequency") {
        Some(ShaderUniformValue::Float(value)) => vec![*value, *value],
        // Preserve the authored vector arity. The existing runtime uniform
        // binder remains the single authority that accepts or rejects it.
        Some(ShaderUniformValue::Vec(values)) if !values.is_empty() => values.clone(),
        _ => vec![DEFAULT_BASE_FREQUENCY, DEFAULT_BASE_FREQUENCY],
    };
    let seed = match uniform(body, "seed") {
        Some(ShaderUniformValue::Float(value)) => *value,
        _ => DEFAULT_SEED,
    };
    let num_octaves = match uniform(body, "numOctaves") {
        Some(ShaderUniformValue::Float(value)) => resolve_num_octaves(*value).effective(),
        _ => DEFAULT_NUM_OCTAVES,
    };

    ExpandedShaderPreset {
        sksl: turbulence_sksl(num_octaves),
        uniforms: vec![
            ShaderUniformPayload {
                name: "baseFrequency".to_string(),
                values: base_frequency,
            },
            ShaderUniformPayload {
                name: "seed".to_string(),
                values: vec![seed],
            },
            // Replaced with the resolved document size when the payload becomes
            // a SceneShader. Keeping the marker here limits implicit injection
            // to programs that declare and consume the exact `size` uniform.
            ShaderUniformPayload {
                name: "size".to_string(),
                values: vec![0.0, 0.0],
            },
        ],
    }
}

fn uniform<'a>(body: &'a ShaderFillBody, name: &str) -> Option<&'a ShaderUniformValue> {
    body.uniforms.as_ref()?.get(name)
}

/// Generate loop-free FBM value-noise SkSL. Octaves are emitted as explicit
/// statements so RuntimeEffect executes only the requested amount of work and
/// the source string remains a natural compile-cache key.
fn turbulence_sksl(num_octaves: usize) -> String {
    let mut source = String::with_capacity(1_600);
    source.push_str(
        r#"uniform float2 baseFrequency;
uniform float seed;
uniform float2 size;

float hash_value(float2 point) {
    float angle = dot(point, float2(127.1, 311.7)) + seed * 74.7;
    return fract(sin(angle) * 43758.5453);
}

float value_noise(float2 point) {
    float2 cell = floor(point);
    float2 local = fract(point);
    float2 smooth_local = local * local * (float2(3.0) - 2.0 * local);
    float bottom_left = hash_value(cell);
    float bottom_right = hash_value(cell + float2(1.0, 0.0));
    float top_left = hash_value(cell + float2(0.0, 1.0));
    float top_right = hash_value(cell + float2(1.0, 1.0));
    float bottom = mix(bottom_left, bottom_right, smooth_local.x);
    float top = mix(top_left, top_right, smooth_local.x);
    return mix(bottom, top, smooth_local.y);
}

half4 main(float2 fragCoord) {
    float2 safe_size = max(size, float2(1.0));
    float2 point = (fragCoord / safe_size) * safe_size * baseFrequency;
    float amplitude = 0.5;
    float total = 0.0;
    float weight = 0.0;
"#,
    );

    for octave in 0..num_octaves {
        if octave != 0 {
            source.push_str(
                "    point = point * 2.0 + float2(17.0, 29.0);\n\
                 amplitude *= 0.5;\n",
            );
        }
        source.push_str(
            "    total += value_noise(point) * amplitude;\n\
             weight += amplitude;\n",
        );
    }

    source.push_str(
        r#"    float noise = total / max(weight, 0.0001);
    return half4(noise, noise, noise, 1.0);
}
"#,
    );
    source
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn body_with_uniforms(uniforms: BTreeMap<String, ShaderUniformValue>) -> ShaderFillBody {
        ShaderFillBody {
            preset: Some("turbulence".to_string()),
            sksl: None,
            uniforms: Some(uniforms),
            explain: None,
            opacity: None,
            blend_mode: None,
        }
    }

    fn octave_call_count(source: &str) -> usize {
        source.matches("value_noise(").count().saturating_sub(1)
    }

    #[test]
    fn generated_program_has_exact_call_count_and_no_shader_loop() {
        for octaves in 1..=MAX_NUM_OCTAVES {
            let source = turbulence_sksl(octaves);
            assert_eq!(octave_call_count(&source), octaves);
            assert!(
                source.len() <= 4 * 1024,
                "preset source must stay within a few KiB, got {} bytes",
                source.len()
            );
            assert!(!source.contains("for ("));
            assert!(!source.contains("for("));
        }
    }

    #[test]
    fn non_integer_and_non_finite_octaves_use_the_default() {
        for value in [2.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut uniforms = BTreeMap::new();
            uniforms.insert("numOctaves".to_string(), ShaderUniformValue::Float(value));
            let expanded = expand(&body_with_uniforms(uniforms)).expect("recognized preset");
            assert_eq!(octave_call_count(&expanded.sksl), DEFAULT_NUM_OCTAVES);
        }
    }

    #[test]
    fn authored_frequency_vector_keeps_its_runtime_arity() {
        let mut uniforms = BTreeMap::new();
        uniforms.insert(
            "baseFrequency".to_string(),
            ShaderUniformValue::Vec(vec![0.02, 0.04, 0.08]),
        );
        let expanded = expand(&body_with_uniforms(uniforms)).expect("recognized preset");
        let frequency = expanded
            .uniforms
            .iter()
            .find(|uniform| uniform.name == "baseFrequency")
            .expect("frequency uniform");
        assert_eq!(frequency.values, [0.02, 0.04, 0.08]);
    }
}
