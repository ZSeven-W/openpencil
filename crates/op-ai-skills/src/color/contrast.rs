use super::oklch::{hex_saturation, hex_to_oklch, parse_hex_rgb, srgb_to_linear};

pub fn wcag(fg_hex: &str, bg_hex: &str) -> Option<f64> {
    let fg = rel_lum(fg_hex)?;
    let bg = rel_lum(bg_hex)?;
    let hi = fg.max(bg);
    let lo = fg.min(bg);
    Some((hi + 0.05) / (lo + 0.05))
}

pub fn on_color(bg_hex: &str) -> &'static str {
    let Some(lightness) = hex_to_oklch(bg_hex).map(|oklch| oklch.l) else {
        return "#0F172A";
    };
    let Some(saturation) = hex_saturation(bg_hex) else {
        return "#0F172A";
    };

    if lightness < 0.5 || (saturation >= 0.5 && lightness <= 0.72) {
        "#FFFFFF"
    } else {
        "#0F172A"
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContrastViolation {
    pub fg: String,
    pub bg: String,
    pub ratio: f64,
    pub target: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContrastReport {
    pub violations: Vec<ContrastViolation>,
}

pub fn scan_pairs(pairs: &[(String, String, f64)]) -> ContrastReport {
    let violations = pairs
        .iter()
        .filter_map(|(fg, bg, target)| {
            let ratio = wcag(fg, bg)?;
            (ratio < *target).then(|| ContrastViolation {
                fg: fg.clone(),
                bg: bg.clone(),
                ratio,
                target: *target,
            })
        })
        .collect();

    ContrastReport { violations }
}

fn rel_lum(hex: &str) -> Option<f64> {
    let (r, g, b) = parse_hex_rgb(hex)?;
    let r = srgb_to_linear(f64::from(r) / 255.0);
    let g = srgb_to_linear(f64::from(g) / 255.0);
    let b = srgb_to_linear(f64::from(b) / 255.0);
    Some(0.2126 * r + 0.7152 * g + 0.0722 * b)
}
