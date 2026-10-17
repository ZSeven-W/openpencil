//! Applying a uniform scale to shape element attributes and point
//! lists.

use super::*;

/// Multiply every numeric coord on a shape element by `scale` so a
/// 24-unit viewBox shape renders at the same final size as the TS
/// app's viewBox-aware path: `width` / `height` / `x` / `y` / `r` /
/// `cx` / `cy` / `rx` / `ry` / `x1` / `y1` / `x2` / `y2` in place,
/// `points` for the point-list shapes. (`<path>` data is carried
/// through the full transform in `svg_path_data::transform_svg_path`
/// instead.)
pub(super) fn apply_scale_to_attrs(el: &mut SvgElement, scale: f64) {
    if (scale - 1.0).abs() < 1e-6 {
        return;
    }
    if el.tag == "polyline" || el.tag == "polygon" {
        if let Some(pos) = el.attrs.iter().position(|(k, _)| k == "points") {
            let scaled = scale_svg_points(&el.attrs[pos].1, scale);
            el.attrs[pos].1 = scaled;
        }
        return;
    }
    let scalable: &[&str] = &[
        "x", "y", "width", "height", "r", "rx", "ry", "cx", "cy", "x1", "y1", "x2", "y2",
    ];
    for (k, v) in &mut el.attrs {
        if !scalable.iter().any(|s| *s == k) {
            continue;
        }
        if let Ok(n) = v.trim().parse::<f64>() {
            *v = format!("{}", n * scale);
        }
    }
}

/// Scale a `points="x1,y1 x2,y2 …"` list for `<polyline>` /
/// `<polygon>`. Returns the same separator style (`x,y x,y …`).
fn scale_svg_points(s: &str, scale: f64) -> String {
    parse_point_list(s)
        .into_iter()
        .map(|(x, y)| format!("{},{}", x * scale, y * scale))
        .collect::<Vec<_>>()
        .join(" ")
}
