//! Colour arithmetic for brand extraction: sRGB triples, WCAG contrast,
//! HSL round-trips, and the CSS value forms design systems store in
//! custom properties.

/// An opaque sRGB colour. Alpha is resolved by the parsers (mostly
/// transparent values are dropped) so everything downstream is opaque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Pure white — the light end of every "black or white" contrast fix.
pub const WHITE: Rgb = Rgb::new(255, 255, 255);
/// Near-black ink — the dark end of every "black or white" contrast fix.
/// Not `#000`: the default palette's darkest ink is a soft near-black.
pub const INK: Rgb = Rgb::new(10, 10, 10);

/// WCAG AA for body text — the floor every `*-foreground` pair must meet.
pub const AA_TEXT: f64 = 4.5;

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// `#RGB`, `#RRGGBB`, or `#RRGGBBAA` (alpha ignored). `None` otherwise.
    pub fn parse_hex(input: &str) -> Option<Self> {
        let hex = input.trim().strip_prefix('#')?;
        if !hex.is_ascii() {
            return None;
        }
        let byte = |s: &str| u8::from_str_radix(s, 16).ok();
        match hex.len() {
            3 | 4 => {
                let digits: Vec<u8> = hex
                    .chars()
                    .take(3)
                    .map(|c| byte(&format!("{c}{c}")))
                    .collect::<Option<_>>()?;
                Some(Self::new(digits[0], digits[1], digits[2]))
            }
            6 | 8 => Some(Self::new(
                byte(&hex[0..2])?,
                byte(&hex[2..4])?,
                byte(&hex[4..6])?,
            )),
            _ => None,
        }
    }

    /// `#RRGGBB`, upper-case — the spelling the bundled palette uses.
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// WCAG 2.x relative luminance.
    pub fn luminance(self) -> f64 {
        fn channel(v: u8) -> f64 {
            let c = f64::from(v) / 255.0;
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// WCAG contrast ratio, `1.0..=21.0`.
    pub fn contrast(self, other: Rgb) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// Linear sRGB mix: `t = 0` is `self`, `t = 1` is `other`.
    pub fn mix(self, other: Rgb, t: f64) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| {
            (f64::from(a) + (f64::from(b) - f64::from(a)) * t)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Rgb::new(
            lerp(self.r, other.r),
            lerp(self.g, other.g),
            lerp(self.b, other.b),
        )
    }

    /// `(hue degrees 0..360, saturation 0..1, lightness 0..1)`.
    pub fn to_hsl(self) -> (f64, f64, f64) {
        let (r, g, b) = (
            f64::from(self.r) / 255.0,
            f64::from(self.g) / 255.0,
            f64::from(self.b) / 255.0,
        );
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        let d = max - min;
        if d < 1e-9 {
            return (0.0, 0.0, l);
        }
        let s = d / (1.0 - (2.0 * l - 1.0).abs());
        let h = if (max - r).abs() < 1e-9 {
            60.0 * (((g - b) / d).rem_euclid(6.0))
        } else if (max - g).abs() < 1e-9 {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        (h.rem_euclid(360.0), s.clamp(0.0, 1.0), l)
    }

    pub fn from_hsl(h: f64, s: f64, l: f64) -> Rgb {
        let s = s.clamp(0.0, 1.0);
        let l = l.clamp(0.0, 1.0);
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let hp = h.rem_euclid(360.0) / 60.0;
        let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
        let (r1, g1, b1) = match hp as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = l - c / 2.0;
        let to = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        Rgb::new(to(r1), to(g1), to(b1))
    }

    /// Colourfulness `(max - min) / 255` — independent of lightness, so a
    /// dark navy and a pale sky blue both read as chromatic.
    pub fn chroma(self) -> f64 {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        f64::from(max - min) / 255.0
    }

    /// A colour a brand would be recognised by: visibly coloured, and not
    /// so close to black or white that the hue is lost.
    pub fn is_chromatic(self) -> bool {
        let l = self.luminance();
        self.chroma() >= 0.16 && l > 0.01 && l < 0.93
    }

    /// Perceptually weighted RGB distance ("redmean"), `0..~765`.
    pub fn distance(self, other: Rgb) -> f64 {
        let rm = (f64::from(self.r) + f64::from(other.r)) / 2.0;
        let dr = f64::from(self.r) - f64::from(other.r);
        let dg = f64::from(self.g) - f64::from(other.g);
        let db = f64::from(self.b) - f64::from(other.b);
        ((2.0 + rm / 256.0) * dr * dr + 4.0 * dg * dg + (2.0 + (255.0 - rm) / 256.0) * db * db)
            .sqrt()
    }

    pub fn is_dark(self) -> bool {
        self.luminance() < 0.18
    }
}

/// Hue distance on the colour wheel, `0..=180` degrees.
pub fn hue_distance(a: Rgb, b: Rgb) -> f64 {
    let d = (a.to_hsl().0 - b.to_hsl().0).abs();
    d.min(360.0 - d)
}

/// Whichever of white / near-black reads better on `bg`.
pub fn best_on(bg: Rgb) -> Rgb {
    if WHITE.contrast(bg) >= INK.contrast(bg) {
        WHITE
    } else {
        INK
    }
}

/// Keep `fg` when it already meets `min` against `bg`; otherwise fall back
/// to black or white, whichever reads better.
pub fn ensure_contrast(fg: Rgb, bg: Rgb, min: f64) -> Rgb {
    if fg.contrast(bg) >= min {
        fg
    } else {
        best_on(bg)
    }
}

/// Move `fg` toward `anchor` in small steps until it meets `min` against
/// every colour in `grounds`; falls back to black/white when even the
/// anchor fails. Used for muted text, which should stay soft but legible.
pub fn raise_contrast(fg: Rgb, anchor: Rgb, grounds: &[Rgb], min: f64) -> Rgb {
    let passes = |c: Rgb| grounds.iter().all(|g| c.contrast(*g) >= min);
    for step in 0..=10 {
        let candidate = fg.mix(anchor, f64::from(step) / 10.0);
        if passes(candidate) {
            return candidate;
        }
    }
    let first = grounds.first().copied().unwrap_or(WHITE);
    best_on(first)
}

/// Parse a CSS colour value into an opaque colour. Values more than half
/// transparent return `None` — an overlay tint is not a brand colour.
///
/// Beyond everything `op-html` understands (hex, rgb/hsl/hwb/lab/lch/
/// oklab/oklch, color-mix, named colours) this accepts the bare channel
/// triples design systems store in custom properties so they can be
/// wrapped later: shadcn's `222.2 47.4% 11.2%` (HSL) and Tailwind's
/// `59 130 246` (RGB).
pub fn parse_css_color(value: &str) -> Option<Rgb> {
    let value = value.trim();
    if value.is_empty() || value.contains("var(") {
        return None;
    }
    let parsed = op_html::color::parse_css_color(value).or_else(|| {
        bare_channels(value).and_then(|wrapped| op_html::color::parse_css_color(&wrapped))
    })?;
    let hex = parsed.strip_prefix('#')?;
    if hex.len() == 8 {
        let alpha = u8::from_str_radix(&hex[6..8], 16).ok()?;
        if alpha < 128 {
            return None;
        }
    }
    Rgb::parse_hex(&parsed)
}

/// `H S% L%` → `hsl(H S% L%)`, `R G B` → `rgb(R G B)`; anything else `None`.
fn bare_channels(value: &str) -> Option<String> {
    let body = value.split('/').next()?.trim();
    let parts: Vec<&str> = body
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let number = |p: &str| p.trim_end_matches('%').parse::<f64>().ok();
    if parts.iter().any(|p| number(p).is_none()) {
        return None;
    }
    let pct = |p: &str| p.ends_with('%');
    if !pct(parts[0]) && pct(parts[1]) && pct(parts[2]) {
        return Some(format!("hsl({} {} {})", parts[0], parts[1], parts[2]));
    }
    if parts.iter().all(|p| !pct(p)) {
        return Some(format!("rgb({} {} {})", parts[0], parts[1], parts[2]));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip_and_shorthand() {
        assert_eq!(Rgb::parse_hex("#0f0"), Some(Rgb::new(0, 255, 0)));
        assert_eq!(Rgb::parse_hex("#12345680").unwrap().to_hex(), "#123456");
        assert_eq!(Rgb::parse_hex("zz"), None);
    }

    #[test]
    fn contrast_matches_wcag_reference_values() {
        assert!((WHITE.contrast(Rgb::new(0, 0, 0)) - 21.0).abs() < 0.01);
        let c = WHITE.contrast(Rgb::parse_hex("#767676").unwrap());
        assert!((c - 4.54).abs() < 0.02, "{c}");
    }

    #[test]
    fn hsl_round_trip_is_stable() {
        let c = Rgb::parse_hex("#3B82F6").unwrap();
        let (h, s, l) = c.to_hsl();
        assert!(Rgb::from_hsl(h, s, l).distance(c) < 3.0);
    }

    #[test]
    fn parses_design_system_channel_triples() {
        let hsl = parse_css_color("222.2 47.4% 11.2%").unwrap();
        assert!(hsl.luminance() < 0.02);
        assert_eq!(parse_css_color("59 130 246"), Some(Rgb::new(59, 130, 246)));
        assert_eq!(parse_css_color("rgba(0,0,0,0.1)"), None, "overlay tint");
        assert_eq!(parse_css_color("var(--x)"), None);
        assert!(parse_css_color("oklch(0.62 0.19 259)").is_some());
    }

    #[test]
    fn ensure_contrast_falls_back_to_black_or_white() {
        let yellow = Rgb::parse_hex("#FFD400").unwrap();
        assert_eq!(ensure_contrast(WHITE, yellow, AA_TEXT), INK);
        let navy = Rgb::parse_hex("#0B1F4D").unwrap();
        assert_eq!(ensure_contrast(INK, navy, AA_TEXT), WHITE);
    }

    #[test]
    fn raise_contrast_keeps_muted_text_soft_but_legible() {
        let bg = WHITE;
        let muted = Rgb::parse_hex("#C8C8C8").unwrap();
        let out = raise_contrast(muted, INK, &[bg], AA_TEXT);
        assert!(out.contrast(bg) >= AA_TEXT);
        assert_ne!(out, INK, "stops before reaching the anchor");
    }
}
