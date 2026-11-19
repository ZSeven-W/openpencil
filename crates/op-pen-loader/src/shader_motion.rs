//! Bounded motion-graphics presets shared by native and CanvasKit loaders.

use jian_ops_schema::style::{ShaderFillBody, ShaderUniformValue};

use crate::payload::ShaderUniformPayload;
use crate::shader_preset::ExpandedShaderPreset;

const MAX_CANDLES: usize = 256;
const MAX_PATH_ROWS: usize = 256;
mod camera_path;

pub(crate) fn expand(body: &ShaderFillBody) -> Option<ExpandedShaderPreset> {
    match body.preset.as_deref()? {
        "candlesticks_3d" => candlesticks(body),
        "spark_burst" => effect(
            body,
            include_str!("shader_motion/spark_burst.sksl"),
            &[
                ("origin", &[0.5, 0.65]),
                ("burst", &[0.0, 1.5, 220.0, 90.0]),
                ("tint", &[0.2, 1.0, 0.65]),
            ],
        ),
        "iridescent_glass" => effect(
            body,
            include_str!("shader_motion/iridescent_glass.sksl"),
            &[("sphere", &[0.5, 0.5, 0.4, 1.0])],
        ),
        _ => None,
    }
}

fn values(body: &ShaderFillBody, name: &str) -> Option<Vec<f32>> {
    let values = match body.uniforms.as_ref()?.get(name)? {
        ShaderUniformValue::Float(value) => vec![*value],
        ShaderUniformValue::Vec(values) => values.clone(),
        ShaderUniformValue::Color(_) => return None,
    };
    values.iter().all(|v| v.is_finite()).then_some(values)
}

fn parameter(body: &ShaderFillBody, name: &str, default: &[f32]) -> ShaderUniformPayload {
    ShaderUniformPayload {
        name: name.to_owned(),
        values: values(body, name)
            .filter(|v| v.len() == default.len())
            .unwrap_or_else(|| default.to_vec()),
    }
}

fn effect(
    body: &ShaderFillBody,
    source: &str,
    parameters: &[(&str, &[f32])],
) -> Option<ExpandedShaderPreset> {
    let mut uniforms = vec![
        parameter(body, "size", &[0.0, 0.0]),
        parameter(body, "sampleTime", &[-1.0]),
    ];
    uniforms.extend(
        parameters
            .iter()
            .map(|(name, value)| parameter(body, name, value)),
    );
    Some(ExpandedShaderPreset {
        sksl: source.to_owned(),
        uniforms,
    })
}

fn candlesticks(body: &ShaderFillBody) -> Option<ExpandedShaderPreset> {
    let bars = values(body, "bars").unwrap_or_else(|| vec![100.0, 110.0, 95.0, 108.0]);
    if bars.is_empty() || !bars.len().is_multiple_of(4) || bars.len() / 4 > MAX_CANDLES {
        return None;
    }
    if bars
        .chunks_exact(4)
        .any(|b| b[1] < b[0].max(b[3]) || b[2] > b[0].min(b[3]))
    {
        return None;
    }
    let count = bars.len() / 4;
    let mut preset = effect(
        body,
        "",
        &[
            ("view", &[(count - 1) as f32, 14.0, 2.0, 500.0]),
            ("lens", &[900.0, 0.34, 0.2, 1.0]),
            ("focal", &[1250.0]),
            ("baseline", &[bars[0]]),
            ("edgeSoftness", &[0.8]),
            ("stage", &[90.0, 0.0, 0.0, 0.0]),
        ],
    )?;
    preset.uniforms.push(ShaderUniformPayload {
        name: "bars".into(),
        values: bars,
    });
    let mut source = format!("uniform float4 bars[{count}];\n");
    source.push_str("float4 candleAt(int i){");
    source.push_str(&lookup(0, count));
    source.push_str("}\n");
    source.push_str(&camera_path::expand(
        body,
        "viewPath",
        6,
        &mut preset.uniforms,
    )?);
    source.push_str(&camera_path::expand(
        body,
        "cameraPath",
        5,
        &mut preset.uniforms,
    )?);
    let view_sample = if source.contains("sampleView(") {
        "currentView=sampleView(t); endpoint=sampleFocus(t).x;"
    } else {
        ""
    };
    let camera_sample = if source.contains("sampleCamera(") {
        "currentCamera=sampleCamera(t);"
    } else {
        ""
    };
    source.push_str(
        &include_str!("shader_motion/candlesticks.sksl")
            .replace("VIEW_SAMPLE", view_sample)
            .replace("CAMERA_SAMPLE", camera_sample),
    );
    preset.sksl = source.replace("CANDLE_COUNT", &count.to_string());
    Some(preset)
}

// SkSL ES2 requires constant array indices. A bounded decision tree retains
// runtime data uniforms without an O(N) scan for every fragment.
fn lookup(lo: usize, hi: usize) -> String {
    if hi - lo == 1 {
        return format!("return bars[{lo}];");
    }
    let mid = (lo + hi) / 2;
    format!(
        "if(i<{mid}){{{}}}else{{{}}}",
        lookup(lo, mid),
        lookup(mid, hi)
    )
}

#[cfg(test)]
mod tests;
