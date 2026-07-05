#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

const L_LIGHT: [f64; 12] = [
    0.992, 0.977, 0.954, 0.933, 0.911, 0.885, 0.850, 0.798, 0.640, 0.590, 0.520, 0.240,
];
const L_DARK: [f64; 12] = [
    0.178, 0.213, 0.255, 0.285, 0.314, 0.353, 0.412, 0.487, 0.640, 0.680, 0.770, 0.930,
];
const C_MULT: [f64; 12] = [
    0.30, 0.45, 0.65, 0.80, 0.90, 0.95, 1.00, 1.00, 1.00, 0.95, 0.75, 0.55,
];
const NEUTRAL_CMAX: f64 = 0.006;

pub fn oklch_to_hex(o: Oklch) -> String {
    let mut chroma = o.c.max(0.0);
    if !in_gamut(oklch_to_linear(o.l, chroma, o.h)) {
        let mut lo = 0.0;
        let mut hi = chroma;
        for _ in 0..24 {
            let mid = (lo + hi) / 2.0;
            if in_gamut(oklch_to_linear(o.l, mid, o.h)) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        chroma = lo;
    }

    let (r, g, b) = oklch_to_linear(o.l, chroma, o.h);
    let r = (linear_to_srgb(r) * 255.0).round() as u8;
    let g = (linear_to_srgb(g) * 255.0).round() as u8;
    let b = (linear_to_srgb(b) * 255.0).round() as u8;
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub fn hex_to_oklch(hex: &str) -> Option<Oklch> {
    let (r, g, b) = parse_hex_rgb(hex)?;
    let r = srgb_to_linear(f64::from(r) / 255.0);
    let g = srgb_to_linear(f64::from(g) / 255.0);
    let b = srgb_to_linear(f64::from(b) / 255.0);

    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let lightness = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;
    let chroma = (a * a + b * b).sqrt();
    let hue = if chroma < 1e-12 {
        0.0
    } else {
        positive_degrees(b.atan2(a).to_degrees())
    };

    Some(Oklch {
        l: lightness,
        c: chroma,
        h: hue,
    })
}

pub fn hex_saturation(hex: &str) -> Option<f64> {
    let (r, g, b) = parse_hex_rgb(hex)?;
    let r = f64::from(r) / 255.0;
    let g = f64::from(g) / 255.0;
    let b = f64::from(b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max == 0.0 {
        Some(0.0)
    } else {
        Some((max - min) / max)
    }
}

pub fn scale12(seed_hue: f64, cmax: f64, mode: Mode, neutral: bool) -> [String; 12] {
    let lightness = match mode {
        Mode::Light => L_LIGHT,
        Mode::Dark => L_DARK,
    };
    let base_chroma = if neutral { NEUTRAL_CMAX } else { cmax };
    std::array::from_fn(|i| {
        oklch_to_hex(Oklch {
            l: lightness[i],
            c: base_chroma * C_MULT[i],
            h: seed_hue,
        })
    })
}

pub(crate) fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim();
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

pub(crate) fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn oklch_to_linear(lightness: f64, chroma: f64, hue: f64) -> (f64, f64, f64) {
    let a = chroma * hue.to_radians().cos();
    let b = chroma * hue.to_radians().sin();
    let l_ = lightness + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = lightness - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = lightness - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_.powi(3);
    let m = m_.powi(3);
    let s = s_.powi(3);

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;
    (r, g, b)
}

fn in_gamut(rgb: (f64, f64, f64)) -> bool {
    const EPS: f64 = 1e-4;
    (-EPS..=1.0 + EPS).contains(&rgb.0)
        && (-EPS..=1.0 + EPS).contains(&rgb.1)
        && (-EPS..=1.0 + EPS).contains(&rgb.2)
}

fn positive_degrees(degrees: f64) -> f64 {
    if degrees < 0.0 {
        degrees + 360.0
    } else {
        degrees
    }
}
