//! [`ModeSignals`] → the shadcn colour vocabulary for one scheme, with
//! every text-on-surface pair held to WCAG AA, plus derivation of the
//! scheme a source did not declare (dark from light, or light from dark).

use std::collections::BTreeMap;

use crate::color::{ensure_contrast, raise_contrast, Rgb, AA_TEXT, INK, WHITE};
use crate::signals::ModeSignals;

/// The themed colour tokens a kit defines, in the palette's order.
pub const COLOR_TOKENS: [&str; 27] = [
    "--background",
    "--foreground",
    "--card",
    "--card-foreground",
    "--popover",
    "--popover-foreground",
    "--primary",
    "--primary-foreground",
    "--secondary",
    "--secondary-foreground",
    "--muted",
    "--muted-foreground",
    "--accent",
    "--accent-foreground",
    "--destructive",
    "--destructive-foreground",
    "--border",
    "--input",
    "--ring",
    "--sidebar",
    "--sidebar-foreground",
    "--sidebar-primary",
    "--sidebar-primary-foreground",
    "--sidebar-accent",
    "--sidebar-accent-foreground",
    "--sidebar-border",
    "--sidebar-ring",
];

/// `(surface, text)` pairs that must read at [`AA_TEXT`].
pub const CONTRAST_PAIRS: [(&str, &str); 9] = [
    ("--background", "--foreground"),
    ("--card", "--card-foreground"),
    ("--popover", "--popover-foreground"),
    ("--primary", "--primary-foreground"),
    ("--secondary", "--secondary-foreground"),
    ("--muted", "--muted-foreground"),
    ("--accent", "--accent-foreground"),
    ("--destructive", "--destructive-foreground"),
    ("--sidebar-primary", "--sidebar-primary-foreground"),
];

/// Default chart hues (the bundled palette's), used to top up a brand
/// that has fewer distinct colours than chart slots.
const CHART_DEFAULTS: [Rgb; 6] = [
    Rgb::new(0x3B, 0x82, 0xF6),
    Rgb::new(0x8B, 0x5C, 0xF6),
    Rgb::new(0xEC, 0x48, 0x99),
    Rgb::new(0x14, 0xB8, 0xA6),
    Rgb::new(0xF5, 0x9E, 0x0B),
    Rgb::new(0xF9, 0x73, 0x16),
];

/// One scheme's resolved colours, keyed by token name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModeTokens {
    values: BTreeMap<&'static str, Rgb>,
}

impl ModeTokens {
    pub fn get(&self, name: &str) -> Option<Rgb> {
        self.values.get(name).copied()
    }

    /// `(token, colour)` in [`COLOR_TOKENS`] order.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, Rgb)> + '_ {
        COLOR_TOKENS
            .iter()
            .filter_map(|name| self.values.get(name).map(|c| (*name, *c)))
    }

    fn set(&mut self, name: &'static str, color: Rgb) {
        self.values.insert(name, color);
    }

    fn color(&self, name: &str) -> Rgb {
        self.get(name).unwrap_or(WHITE)
    }
}

/// Resolve every token for one scheme. Missing evidence is derived from
/// the resolved ground / ink / brand, never from an unrelated palette.
pub fn resolve_mode(signals: &ModeSignals, dark: bool) -> ModeTokens {
    let mut t = ModeTokens::default();
    let bg = signals.background.unwrap_or(if dark {
        Rgb::new(0x0B, 0x0B, 0x0F)
    } else {
        WHITE
    });
    let fg = ensure_contrast(
        signals
            .foreground
            .unwrap_or_else(|| crate::color::best_on(bg)),
        bg,
        AA_TEXT,
    );
    let primary = signals
        .primary
        .filter(|p| p.contrast(bg) >= 1.3)
        .unwrap_or(fg);
    let primary_fg = ensure_contrast(
        signals
            .primary_foreground
            .unwrap_or_else(|| crate::color::best_on(primary)),
        primary,
        AA_TEXT,
    );
    let card = signals
        .card
        .unwrap_or(if dark { bg.mix(fg, 0.04) } else { bg });
    let secondary = signals
        .secondary
        .unwrap_or_else(|| bg.mix(fg, if dark { 0.12 } else { 0.06 }));
    let muted = signals
        .muted
        .unwrap_or_else(|| bg.mix(fg, if dark { 0.10 } else { 0.05 }));
    let muted_fg = raise_contrast(
        signals.muted_foreground.unwrap_or_else(|| fg.mix(bg, 0.45)),
        fg,
        &[muted, bg],
        AA_TEXT,
    );
    let accent = signals
        .accent
        .unwrap_or_else(|| bg.mix(primary, if dark { 0.18 } else { 0.10 }));
    let destructive = signals.destructive.unwrap_or(if dark {
        Rgb::new(0xF8, 0x71, 0x71)
    } else {
        Rgb::new(0xDC, 0x26, 0x26)
    });
    let border = signals
        .border
        .unwrap_or_else(|| bg.mix(fg, if dark { 0.16 } else { 0.12 }));

    t.set("--background", bg);
    t.set("--foreground", fg);
    t.set("--card", card);
    t.set("--card-foreground", ensure_contrast(fg, card, AA_TEXT));
    t.set("--popover", card);
    t.set("--popover-foreground", ensure_contrast(fg, card, AA_TEXT));
    t.set("--primary", primary);
    t.set("--primary-foreground", primary_fg);
    t.set("--secondary", secondary);
    t.set(
        "--secondary-foreground",
        ensure_contrast(fg, secondary, AA_TEXT),
    );
    t.set("--muted", muted);
    t.set("--muted-foreground", muted_fg);
    t.set("--accent", accent);
    t.set("--accent-foreground", ensure_contrast(fg, accent, AA_TEXT));
    t.set("--destructive", destructive);
    t.set(
        "--destructive-foreground",
        ensure_contrast(if dark { INK } else { WHITE }, destructive, AA_TEXT),
    );
    t.set("--border", border);
    t.set("--input", border);
    t.set("--ring", primary);
    t.set("--sidebar", card);
    t.set("--sidebar-foreground", ensure_contrast(fg, card, AA_TEXT));
    t.set("--sidebar-primary", primary);
    t.set("--sidebar-primary-foreground", primary_fg);
    t.set("--sidebar-accent", accent);
    t.set(
        "--sidebar-accent-foreground",
        ensure_contrast(fg, accent, AA_TEXT),
    );
    t.set("--sidebar-border", border);
    t.set("--sidebar-ring", primary);
    t
}

/// Evidence for the scheme a source did not declare, derived from the one
/// it did: the brand hue carries over, ground and ink flip, and the brand
/// colour is lightened (for dark) or darkened (for light) until it still
/// stands out against the new ground.
pub fn derive_alternate(from: &ModeTokens, extra: &[Rgb], to_dark: bool) -> ModeSignals {
    let primary = from.color("--primary");
    let bg = from.color("--background");
    let monochrome = !primary.is_chromatic();
    let (h, s, _) = if monochrome {
        bg.to_hsl()
    } else {
        primary.to_hsl()
    };
    let mut out = ModeSignals::default();
    let (bg2, fg2) = if to_dark {
        (
            Rgb::from_hsl(h, (s * 0.35).min(0.22), 0.075),
            Rgb::from_hsl(h, (s * 0.1).min(0.08), 0.96),
        )
    } else {
        (
            Rgb::from_hsl(h, (s * 0.3).min(0.2), 0.985),
            Rgb::from_hsl(h, (s * 0.3).min(0.25), 0.08),
        )
    };
    let primary2 = if monochrome {
        fg2
    } else {
        stand_out(primary, bg2, to_dark)
    };
    out.background = Some(bg2);
    out.foreground = Some(fg2);
    out.primary = Some(primary2);
    out.card = Some(if to_dark {
        bg2.mix(fg2, 0.05)
    } else {
        bg2.mix(WHITE, 0.7)
    });
    out.border = Some(bg2.mix(fg2, if to_dark { 0.16 } else { 0.12 }));
    out.muted = Some(bg2.mix(fg2, if to_dark { 0.10 } else { 0.05 }));
    out.extra_brand = extra.iter().map(|c| stand_out(*c, bg2, to_dark)).collect();
    out.note(if to_dark {
        "dark theme: derived from the light theme (brand hue kept)"
    } else {
        "light theme: derived from the dark theme (brand hue kept)"
    });
    out
}

/// Shift `c`'s lightness away from `ground` until it reaches 3:1 (the
/// non-text UI contrast floor), keeping hue and saturation.
pub(crate) fn stand_out(c: Rgb, ground: Rgb, lighter: bool) -> Rgb {
    let (h, s, mut l) = c.to_hsl();
    let mut out = c;
    for _ in 0..20 {
        if out.contrast(ground) >= 3.0 {
            break;
        }
        l = if lighter {
            (l + 0.04).min(0.85)
        } else {
            (l - 0.04).max(0.15)
        };
        out = Rgb::from_hsl(h, s, l);
    }
    out
}

/// Six chart colours: the brand's other colours first, topped up with the
/// default chart hues that do not clash with the brand or each other.
///
/// The primary itself is deliberately NOT a chart colour: generation binds
/// a literal colour to the first variable (in name order) holding it, and
/// `--chart-1` sorts before `--primary` — a chart slot equal to the brand
/// colour would capture every generated brand fill.
pub fn chart_colors(primary: Rgb, extra: &[Rgb]) -> [Rgb; 6] {
    let mut picked: Vec<Rgb> = Vec::with_capacity(6);
    for c in extra {
        if picked.len() < 6 && *c != primary && !picked.iter().any(|p| p.distance(*c) < 60.0) {
            picked.push(*c);
        }
    }
    let brand_clash = |c: Rgb| primary.is_chromatic() && primary.distance(c) < 80.0;
    for c in CHART_DEFAULTS {
        if picked.len() < 6 && !brand_clash(c) && !picked.iter().any(|p| p.distance(c) < 80.0) {
            picked.push(c);
        }
    }
    // Clashing defaults were skipped above; if that left slots open, fill
    // them with any default not already present so all six stay distinct.
    for c in CHART_DEFAULTS {
        if picked.len() < 6 && !picked.contains(&c) && c != primary {
            picked.push(c);
        }
    }
    let mut out = CHART_DEFAULTS;
    for (slot, c) in out.iter_mut().zip(picked) {
        *slot = c;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals(bg: &str, fg: &str, primary: &str) -> ModeSignals {
        ModeSignals {
            background: Rgb::parse_hex(bg),
            foreground: Rgb::parse_hex(fg),
            primary: Rgb::parse_hex(primary),
            ..ModeSignals::default()
        }
    }

    fn assert_pairs_pass(t: &ModeTokens) {
        for (surface, text) in CONTRAST_PAIRS {
            let (s, x) = (t.get(surface).unwrap(), t.get(text).unwrap());
            assert!(
                x.contrast(s) >= AA_TEXT,
                "{text} {} on {surface} {} = {:.2}",
                x.to_hex(),
                s.to_hex(),
                x.contrast(s)
            );
        }
    }

    #[test]
    fn every_token_resolves_and_every_pair_meets_aa() {
        // A yellow brand whose site put white text on it: the pair is fixed.
        let mut sig = signals("#FFFFFF", "#333333", "#FFD400");
        sig.primary_foreground = Some(WHITE);
        let t = resolve_mode(&sig, false);
        assert_eq!(t.iter().count(), COLOR_TOKENS.len());
        assert_eq!(t.get("--primary-foreground"), Some(INK));
        assert_pairs_pass(&t);
    }

    #[test]
    fn low_contrast_ink_is_replaced() {
        let t = resolve_mode(&signals("#FFFFFF", "#DDDDDD", "#0E7C66"), false);
        assert_eq!(t.get("--foreground"), Some(INK));
        assert_pairs_pass(&t);
    }

    #[test]
    fn derived_dark_keeps_the_brand_hue_and_passes_aa() {
        let light = resolve_mode(&signals("#FFFFFF", "#111111", "#1D4ED8"), false);
        let dark_sig = derive_alternate(&light, &[], true);
        let dark = resolve_mode(&dark_sig, true);
        let bg = dark.get("--background").unwrap();
        assert!(bg.is_dark());
        let p = dark.get("--primary").unwrap();
        assert!(crate::color::hue_distance(p, Rgb::parse_hex("#1D4ED8").unwrap()) < 12.0);
        assert!(p.contrast(bg) >= 3.0);
        assert_pairs_pass(&dark);
    }

    #[test]
    fn monochrome_brand_inverts_for_dark() {
        let light = resolve_mode(&signals("#FFFFFF", "#0A0A0A", "#111111"), false);
        let dark = resolve_mode(&derive_alternate(&light, &[], true), true);
        assert!(!dark.get("--primary").unwrap().is_dark());
        assert_pairs_pass(&dark);
    }

    #[test]
    fn charts_lead_with_the_other_brand_colours_never_the_primary() {
        let brand = Rgb::parse_hex("#0E7C66").unwrap();
        let charts = chart_colors(brand, &[Rgb::parse_hex("#F97316").unwrap()]);
        assert_eq!(charts[0], Rgb::parse_hex("#F97316").unwrap());
        assert!(!charts.contains(&brand));
        let unique: std::collections::BTreeSet<_> = charts.iter().collect();
        assert_eq!(unique.len(), 6);
    }
}
