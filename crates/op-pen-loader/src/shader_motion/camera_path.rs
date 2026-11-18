//! Shape-preserving C1 paths: continuous velocity without camera overshoot.

use super::{values, ShaderFillBody, ShaderUniformPayload, MAX_PATH_ROWS};

/// Rows: seconds + five view values, or seconds + yaw/pitch/focal/glow.
pub(super) fn expand(
    body: &ShaderFillBody,
    name: &str,
    width: usize,
    uniforms: &mut Vec<ShaderUniformPayload>,
) -> Option<String> {
    let Some(data) = values(body, name) else {
        return Some(String::new());
    };
    if data.is_empty() || !data.len().is_multiple_of(width) || data.len() / width > MAX_PATH_ROWS {
        return None;
    }
    let rows: Vec<_> = data.chunks_exact(width).collect();
    if rows.iter().any(|row| row[0] < 0.0) || rows.windows(2).any(|pair| pair[0][0] >= pair[1][0]) {
        return None;
    }
    let is_view = name == "viewPath";
    if rows.iter().any(|r| {
        if is_view {
            r[2] <= 0.0 || r[3] <= 0.0
        } else {
            r[1].abs() > 1.2 || r[2].abs() > 1.2 || r[3] < 100.0 || r[4] < 0.0
        }
    }) {
        return None;
    }
    let smooth = values(body, "pathSmoothness").is_some_and(|v| v == [1.0]);
    let components = if is_view { 8 } else { 4 };
    let mut packed = Vec::with_capacity(rows.len() * components);
    for row in &rows {
        packed.extend_from_slice(&row[1..5]);
        if is_view {
            packed.extend_from_slice(&[row[5], 0.0, 0.0, 0.0]);
        }
    }
    let slopes = if smooth { Some(tangents(&rows)?) } else { None };
    if let Some(slopes) = &slopes {
        let mut packed_slopes = Vec::with_capacity(packed.len());
        for slope in slopes {
            packed_slopes.extend_from_slice(&slope[..4]);
            if is_view {
                packed_slopes.extend_from_slice(&[slope[4], 0.0, 0.0, 0.0]);
            }
        }
        uniforms.push(ShaderUniformPayload {
            name: format!("{name}Tangents"),
            values: packed_slopes,
        });
    }
    uniforms.push(ShaderUniformPayload {
        name: name.to_owned(),
        values: packed,
    });
    let mut source = format!("uniform float4 {name}[{}];\n", rows.len() * components / 4);
    if smooth {
        source.push_str(&format!(
            "uniform float4 {name}Tangents[{}];\n",
            rows.len() * components / 4
        ));
    }
    let stride = components / 4;
    let kinds: &[(&str, usize)] = if is_view {
        &[("sampleView", 0), ("sampleFocus", 1)]
    } else {
        &[("sampleCamera", 0)]
    };
    for (function, offset) in kinds {
        source.push_str(&format!("float4 {function}(float t){{"));
        source.push_str(&format!("if(t<={})return {name}[{offset}];", rows[0][0]));
        source.push_str(&path_tree(
            &rows,
            0,
            rows.len() - 1,
            name,
            stride,
            *offset,
            smooth,
        ));
        source.push_str("}\n");
    }
    Some(source)
}

fn path_tree(
    rows: &[&[f32]],
    lo: usize,
    hi: usize,
    name: &str,
    stride: usize,
    offset: usize,
    smooth: bool,
) -> String {
    let a = lo * stride + offset;
    if lo == hi {
        return format!("return {name}[{a}];");
    }
    if hi - lo == 1 {
        let b = hi * stride + offset;
        if smooth {
            return format!("float u=clamp((t-{})/{},0.,1.);float u2=u*u,u3=u2*u;return (2.*u3-3.*u2+1.)*{name}[{a}]+(u3-2.*u2+u)*{}*{name}Tangents[{a}]+(-2.*u3+3.*u2)*{name}[{b}]+(u3-u2)*{}*{name}Tangents[{b}];", rows[lo][0], rows[hi][0]-rows[lo][0], rows[hi][0]-rows[lo][0], rows[hi][0]-rows[lo][0]);
        }
        return format!(
            "return mix({name}[{a}],{name}[{b}],clamp((t-{})/{},0.,1.));",
            rows[lo][0],
            rows[hi][0] - rows[lo][0]
        );
    }
    let mid = (lo + hi) / 2;
    format!(
        "if(t<{}){{{}}}else{{{}}}",
        rows[mid][0],
        path_tree(rows, lo, mid, name, stride, offset, smooth),
        path_tree(rows, mid, hi, name, stride, offset, smooth)
    )
}

// Weighted harmonic slopes are bounded and vanish at reversals/plateaus.
// Held endpoints have zero velocity, so entering or leaving the path is C1.
fn tangents(rows: &[&[f32]]) -> Option<Vec<Vec<f32>>> {
    let columns = rows.first()?.len() - 1;
    let mut result = vec![vec![0.0; columns]; rows.len()];
    for i in 1..rows.len().saturating_sub(1) {
        let h0 = f64::from(rows[i][0]) - f64::from(rows[i - 1][0]);
        let h1 = f64::from(rows[i + 1][0]) - f64::from(rows[i][0]);
        for j in 0..columns {
            let a = (f64::from(rows[i][j + 1]) - f64::from(rows[i - 1][j + 1])) / h0;
            let b = (f64::from(rows[i + 1][j + 1]) - f64::from(rows[i][j + 1])) / h1;
            if a * b > 0.0 {
                let w0 = 2.0 * h1 + h0;
                let w1 = h1 + 2.0 * h0;
                let slope = ((w0 + w1) / (w0 / a + w1 / b)) as f32;
                if !slope.is_finite() {
                    return None;
                }
                result[i][j] = slope;
            }
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_paths_stay_within_authored_values_and_stop_at_reversals() {
        let rows: &[&[f32]] = &[&[0., 1.], &[1., 3.], &[4., 4.], &[5., 2.], &[7., 2.]];
        let slopes = tangents(rows).unwrap();
        assert_eq!(slopes[0][0], 0.);
        assert_eq!(slopes[2][0], 0.);
        assert_eq!(slopes[3][0], 0.);
        assert_eq!(slopes[4][0], 0.);
        assert!(slopes[1][0] > 0.);
        for i in 0..rows.len() - 1 {
            let a = rows[i][1];
            let b = rows[i + 1][1];
            let h = rows[i + 1][0] - rows[i][0];
            for sample in 0..=100 {
                let u = sample as f32 / 100.;
                let u2 = u * u;
                let u3 = u2 * u;
                let value = (2. * u3 - 3. * u2 + 1.) * a
                    + (u3 - 2. * u2 + u) * h * slopes[i][0]
                    + (-2. * u3 + 3. * u2) * b
                    + (u3 - u2) * h * slopes[i + 1][0];
                assert!(value >= a.min(b) - 1e-5 && value <= a.max(b) + 1e-5);
            }
        }
    }
}
