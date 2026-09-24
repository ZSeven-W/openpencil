//! Tailwind design-token scales for the `react-tailwind` target.
//!
//! Every lookup is exact: a document value lands on a named utility only
//! when it equals a step of the stock scale, otherwise it becomes an
//! arbitrary value (`[13px]`). The tables deliberately stick to steps
//! that mean the same thing in Tailwind v3 and v4 so the generated
//! markup renders identically on either major version.

use crate::fmt_num;

/// Stock spacing scale (`p-4`, `gap-2`, `w-10`, `left-6`, …) as
/// `(px, suffix)` pairs. Tailwind v3's fixed steps — v4 accepts any
/// multiple of 0.25rem, but emitting only the shared subset keeps the
/// output portable.
const SPACING: &[(f64, &str)] = &[
    (0.0, "0"),
    (1.0, "px"),
    (2.0, "0.5"),
    (4.0, "1"),
    (6.0, "1.5"),
    (8.0, "2"),
    (10.0, "2.5"),
    (12.0, "3"),
    (14.0, "3.5"),
    (16.0, "4"),
    (20.0, "5"),
    (24.0, "6"),
    (28.0, "7"),
    (32.0, "8"),
    (36.0, "9"),
    (40.0, "10"),
    (44.0, "11"),
    (48.0, "12"),
    (56.0, "14"),
    (64.0, "16"),
    (80.0, "20"),
    (96.0, "24"),
    (112.0, "28"),
    (128.0, "32"),
    (144.0, "36"),
    (160.0, "40"),
    (176.0, "44"),
    (192.0, "48"),
    (208.0, "52"),
    (224.0, "56"),
    (240.0, "60"),
    (256.0, "64"),
    (288.0, "72"),
    (320.0, "80"),
    (384.0, "96"),
];

/// Font-size scale (`text-sm`, …) keyed by px.
const FONT_SIZE: &[(f64, &str)] = &[
    (12.0, "xs"),
    (14.0, "sm"),
    (16.0, "base"),
    (18.0, "lg"),
    (20.0, "xl"),
    (24.0, "2xl"),
    (30.0, "3xl"),
    (36.0, "4xl"),
    (48.0, "5xl"),
    (60.0, "6xl"),
    (72.0, "7xl"),
    (96.0, "8xl"),
    (128.0, "9xl"),
];

/// Unitless line-height scale (`leading-tight`, …).
const LEADING: &[(f64, &str)] = &[
    (1.0, "none"),
    (1.25, "tight"),
    (1.375, "snug"),
    (1.5, "normal"),
    (1.625, "relaxed"),
    (2.0, "loose"),
];

/// Opacity steps shared by v3.0+ and v4.
const OPACITY: &[u32] = &[0, 5, 10, 20, 25, 30, 40, 50, 60, 70, 75, 80, 90, 95, 100];

/// Rotation steps (`rotate-45`, …).
const ROTATE: &[f64] = &[0.0, 1.0, 2.0, 3.0, 6.0, 12.0, 45.0, 90.0, 180.0];

/// Values closer than this are treated as equal (float noise from
/// layout math must not push `16.0000001` off the scale).
const EPSILON: f64 = 1e-6;

fn lookup(table: &[(f64, &'static str)], value: f64) -> Option<&'static str> {
    table
        .iter()
        .find(|(step, _)| (step - value).abs() < EPSILON)
        .map(|(_, name)| *name)
}

/// Arbitrary-value body for a px length: `13px`, `12.5px`.
pub(super) fn px(value: f64) -> String {
    format!("{}px", fmt_num(round_px(value)))
}

/// Round to 2 decimals so layout noise cannot leak into class names.
pub(super) fn round_px(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// A spacing utility: `{prefix}-{step}` on-scale, `{prefix}-[Npx]`
/// off-scale. Negative values use Tailwind's leading-dash form
/// (`-left-4`, `left-[-13px]`).
pub(super) fn spacing(prefix: &str, value: f64) -> String {
    let value = round_px(value);
    if value < 0.0 {
        return match lookup(SPACING, -value) {
            Some(step) => format!("-{prefix}-{step}"),
            None => format!("{prefix}-[{}]", px(value)),
        };
    }
    match lookup(SPACING, value) {
        Some(step) => format!("{prefix}-{step}"),
        None => format!("{prefix}-[{}]", px(value)),
    }
}

/// `text-sm` / `text-[15px]`.
pub(super) fn font_size(value: f64) -> String {
    match lookup(FONT_SIZE, round_px(value)) {
        Some(step) => format!("text-{step}"),
        None => format!("text-[{}]", px(value)),
    }
}

/// `leading-tight` / `leading-[1.4]` for a multiplier, `leading-[17px]`
/// for a pixel line height (values above 4 are pixel heights, matching
/// the schema's `canonical_line_height_multiplier` rule).
pub(super) fn leading(value: f64) -> Option<String> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    if value > 4.0 {
        return Some(format!("leading-[{}]", px(value)));
    }
    let rounded = (value * 1000.0).round() / 1000.0;
    Some(match lookup(LEADING, rounded) {
        Some(step) => format!("leading-{step}"),
        None => format!("leading-[{}]", fmt_num(rounded)),
    })
}

/// `font-semibold` for numeric CSS weights / common keywords.
pub(super) fn font_weight(weight: u32) -> Option<&'static str> {
    Some(match weight {
        100 => "font-thin",
        200 => "font-extralight",
        300 => "font-light",
        400 => "font-normal",
        500 => "font-medium",
        600 => "font-semibold",
        700 => "font-bold",
        800 => "font-extrabold",
        900 => "font-black",
        _ => return None,
    })
}

/// Weight keywords the schema accepts as strings (`"bold"`).
pub(super) fn font_weight_keyword(keyword: &str) -> Option<&'static str> {
    let weight = match keyword.trim().to_ascii_lowercase().as_str() {
        "thin" | "hairline" => 100,
        "extralight" | "extra-light" | "ultralight" => 200,
        "light" => 300,
        "normal" | "regular" => 400,
        "medium" => 500,
        "semibold" | "semi-bold" | "demibold" => 600,
        "bold" => 700,
        "extrabold" | "extra-bold" | "ultrabold" => 800,
        "black" | "heavy" => 900,
        other => other.parse::<u32>().ok()?,
    };
    font_weight(weight)
}

/// `opacity-50` / `opacity-[0.37]` for a 0..=1 opacity. Full opacity
/// emits nothing.
pub(super) fn opacity(value: f64) -> Option<String> {
    if !value.is_finite() || value >= 1.0 - EPSILON {
        return None;
    }
    let value = value.max(0.0);
    let percent = (value * 100.0).round() as u32;
    if (value * 100.0 - f64::from(percent)).abs() < 1e-3 && OPACITY.contains(&percent) {
        Some(format!("opacity-{percent}"))
    } else {
        Some(format!(
            "opacity-[{}]",
            fmt_num((value * 1000.0).round() / 1000.0)
        ))
    }
}

/// Colour opacity modifier suffix: `/50`, `/[0.37]`, or empty at 100%.
pub(super) fn color_alpha_suffix(alpha: f32) -> String {
    let alpha = f64::from(alpha);
    if !alpha.is_finite() || alpha >= 1.0 - EPSILON {
        return String::new();
    }
    let alpha = alpha.max(0.0);
    let percent = (alpha * 100.0).round() as u32;
    if (alpha * 100.0 - f64::from(percent)).abs() < 1e-3 && percent.is_multiple_of(5) {
        format!("/{percent}")
    } else {
        format!("/[{}]", fmt_num((alpha * 1000.0).round() / 1000.0))
    }
}

/// `rotate-45` / `-rotate-90` / `rotate-[13deg]`.
pub(super) fn rotate(degrees: f64) -> Option<String> {
    let degrees = (degrees * 100.0).round() / 100.0;
    if degrees.abs() < EPSILON {
        return None;
    }
    let magnitude = degrees.abs();
    if ROTATE.iter().any(|step| (step - magnitude).abs() < EPSILON) {
        let sign = if degrees < 0.0 { "-" } else { "" };
        return Some(format!("{sign}rotate-{}", fmt_num(magnitude)));
    }
    Some(format!("rotate-[{}deg]", fmt_num(degrees)))
}

/// Border width utility for one side (`border`, `border-t-2`,
/// `border-[3px]`). `side` is `""`, `"t"`, `"r"`, `"b"` or `"l"`.
pub(super) fn border_width(side: &str, width: f64) -> Option<String> {
    let width = round_px(width);
    if width <= 0.0 {
        return None;
    }
    let prefix = if side.is_empty() {
        "border".to_string()
    } else {
        format!("border-{side}")
    };
    Some(match width {
        w if (w - 1.0).abs() < EPSILON => prefix,
        w if [2.0, 4.0, 8.0].iter().any(|s| (s - w).abs() < EPSILON) => {
            format!("{prefix}-{}", fmt_num(w))
        }
        w => format!("{prefix}-[{}]", px(w)),
    })
}

/// Radius steps the generated theme leaves untouched, and — when the
/// document defines `--radius` — the shadcn `sm`/`md`/`lg`/`xl` steps
/// re-derived from it (mirroring the `@theme` block the target emits).
#[derive(Debug, Clone)]
pub(super) struct RadiusScale {
    steps: Vec<(f64, &'static str)>,
}

impl RadiusScale {
    /// `base` is the document's `--radius` in px, when it has one.
    pub(super) fn new(base: Option<f64>) -> Self {
        let mut steps: Vec<(f64, &'static str)> = vec![(0.0, "none")];
        match base {
            Some(r) if r.is_finite() && r > 0.0 => {
                steps.push((r, "lg"));
                steps.push((r - 2.0, "md"));
                steps.push((r - 4.0, "sm"));
                steps.push((r + 4.0, "xl"));
            }
            _ => {
                steps.push((6.0, "md"));
                steps.push((8.0, "lg"));
                steps.push((12.0, "xl"));
            }
        }
        for (px, name) in [(16.0, "2xl"), (24.0, "3xl")] {
            if !steps.iter().any(|(v, _)| (v - px).abs() < EPSILON) {
                steps.push((px, name));
            }
        }
        // Earlier entries win on collisions (e.g. `--radius: 4` makes
        // `sm` 0, which must not shadow `none`).
        Self { steps }
    }

    /// `rounded-lg` / `rounded-t-lg` / `rounded-[10px]` / `rounded-full`.
    /// `corner` is `""` or a Tailwind corner key (`tl`, `tr`, `br`, `bl`).
    pub(super) fn class(&self, corner: &str, value: f64) -> Option<String> {
        let value = round_px(value);
        if value <= 0.0 {
            return None;
        }
        let prefix = if corner.is_empty() {
            "rounded".to_string()
        } else {
            format!("rounded-{corner}")
        };
        if value >= 9999.0 {
            return Some(format!("{prefix}-full"));
        }
        let hit = self
            .steps
            .iter()
            .find(|(step, _)| *step > 0.0 && (step - value).abs() < EPSILON);
        Some(match hit {
            Some((_, name)) => format!("{prefix}-{name}"),
            None => format!("{prefix}-[{}]", px(value)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_hits_scale_and_falls_back_to_arbitrary_px() {
        assert_eq!(spacing("p", 16.0), "p-4");
        assert_eq!(spacing("gap", 2.0), "gap-0.5");
        assert_eq!(spacing("w", 1.0), "w-px");
        assert_eq!(spacing("p", 13.0), "p-[13px]");
        assert_eq!(spacing("left", -16.0), "-left-4");
        assert_eq!(spacing("left", -13.0), "left-[-13px]");
        assert_eq!(spacing("h", 12.5), "h-[12.5px]");
    }

    #[test]
    fn typography_scales_are_exact() {
        assert_eq!(font_size(14.0), "text-sm");
        assert_eq!(font_size(15.0), "text-[15px]");
        assert_eq!(leading(1.5).as_deref(), Some("leading-normal"));
        assert_eq!(leading(1.4).as_deref(), Some("leading-[1.4]"));
        assert_eq!(leading(20.0).as_deref(), Some("leading-[20px]"));
        assert_eq!(font_weight(600), Some("font-semibold"));
        assert_eq!(font_weight_keyword("Bold"), Some("font-bold"));
        assert_eq!(font_weight(450), None);
    }

    #[test]
    fn opacity_rotation_and_border_steps() {
        assert_eq!(opacity(0.5).as_deref(), Some("opacity-50"));
        assert_eq!(opacity(0.37).as_deref(), Some("opacity-[0.37]"));
        assert_eq!(opacity(1.0), None);
        assert_eq!(color_alpha_suffix(0.5), "/50");
        assert_eq!(color_alpha_suffix(0.33), "/[0.33]");
        assert_eq!(rotate(-45.0).as_deref(), Some("-rotate-45"));
        assert_eq!(rotate(13.0).as_deref(), Some("rotate-[13deg]"));
        assert_eq!(border_width("", 1.0).as_deref(), Some("border"));
        assert_eq!(border_width("t", 2.0).as_deref(), Some("border-t-2"));
        assert_eq!(border_width("", 3.0).as_deref(), Some("border-[3px]"));
    }

    #[test]
    fn radius_scale_tracks_the_documents_radius_token() {
        let stock = RadiusScale::new(None);
        assert_eq!(stock.class("", 8.0).as_deref(), Some("rounded-lg"));
        assert_eq!(stock.class("", 12.0).as_deref(), Some("rounded-xl"));
        assert_eq!(stock.class("", 10.0).as_deref(), Some("rounded-[10px]"));
        assert_eq!(stock.class("tl", 16.0).as_deref(), Some("rounded-tl-2xl"));
        assert_eq!(stock.class("", 99999.0).as_deref(), Some("rounded-full"));
        let themed = RadiusScale::new(Some(10.0));
        assert_eq!(themed.class("", 10.0).as_deref(), Some("rounded-lg"));
        assert_eq!(themed.class("", 8.0).as_deref(), Some("rounded-md"));
        assert_eq!(themed.class("", 6.0).as_deref(), Some("rounded-sm"));
        assert_eq!(themed.class("", 14.0).as_deref(), Some("rounded-xl"));
    }
}
