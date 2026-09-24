//! Named custom properties + per-role paint → one scheme's
//! [`ModeSignals`]. The property name tables are the vocabularies real
//! design systems publish (shadcn, Bootstrap, Tailwind v4, plain
//! `--brand-*` conventions), tried in priority order.

use std::collections::HashMap;

use crate::color::{hue_distance, parse_css_color, Rgb};
use crate::css_select::{resolve_vars, Role};
use crate::css_signals::top_key;
use crate::signals::ModeSignals;

/// Weighted evidence for one role.
#[derive(Clone, Debug, Default)]
pub(crate) struct RoleStats {
    pub bg: HashMap<Rgb, f64>,
    pub text: HashMap<Rgb, f64>,
    pub border: HashMap<Rgb, f64>,
    pub fonts: HashMap<String, f64>,
    /// `(px, weight)`; pill / percent radii are recorded as `9999`.
    pub radii: Vec<(f64, f64)>,
}

const BACKGROUND: &[&str] = &[
    "--background",
    "--color-background",
    "--bg",
    "--background-color",
    "--color-bg",
    "--bs-body-bg",
    "--body-bg",
    "--page-bg",
];
const FOREGROUND: &[&str] = &[
    "--foreground",
    "--color-foreground",
    "--text",
    "--text-color",
    "--color-text",
    "--bs-body-color",
    "--body-color",
    "--fg",
];
const PRIMARY: &[&str] = &[
    "--primary",
    "--color-primary",
    "--brand",
    "--brand-color",
    "--color-brand",
    "--brand-primary",
    "--primary-color",
    "--bs-primary",
    "--theme-color",
    "--accent-color",
    "--main-color",
];
const PRIMARY_FOREGROUND: &[&str] = &[
    "--primary-foreground",
    "--color-primary-foreground",
    "--on-primary",
    "--primary-contrast",
];
const CARD: &[&str] = &[
    "--card",
    "--color-card",
    "--surface",
    "--color-surface",
    "--card-bg",
];
const SECONDARY: &[&str] = &["--secondary", "--color-secondary", "--bs-secondary"];
const MUTED: &[&str] = &[
    "--muted",
    "--color-muted",
    "--bs-secondary-bg",
    "--bs-tertiary-bg",
];
const MUTED_FOREGROUND: &[&str] = &[
    "--muted-foreground",
    "--color-muted-foreground",
    "--text-muted",
    "--color-text-muted",
    "--bs-secondary-color",
];
const ACCENT: &[&str] = &["--accent", "--color-accent"];
const BORDER: &[&str] = &[
    "--border",
    "--color-border",
    "--border-color",
    "--bs-border-color",
];
const DESTRUCTIVE: &[&str] = &[
    "--destructive",
    "--color-destructive",
    "--danger",
    "--color-danger",
    "--bs-danger",
];

/// Resolve one scheme. `label` prefixes provenance lines.
pub(crate) fn mode_from_evidence(
    lookup: &dyn Fn(&str) -> Option<String>,
    stats: &HashMap<Role, RoleStats>,
    global: &HashMap<Rgb, f64>,
    theme_color: Option<Rgb>,
    shadcn_like: bool,
    label: &str,
) -> ModeSignals {
    let mut out = ModeSignals::default();
    let prop = |names: &[&str]| -> Option<(Rgb, String)> {
        names.iter().find_map(|name| {
            let raw = lookup(name)?;
            let value = resolve_vars(&raw, lookup);
            parse_css_color(&value).map(|c| (c, (*name).to_string()))
        })
    };
    let top = |role: Role, pick: fn(&RoleStats) -> &HashMap<Rgb, f64>| {
        stats.get(&role).and_then(|s| top_key(pick(s)))
    };
    let top_chromatic = |role: Role, pick: fn(&RoleStats) -> &HashMap<Rgb, f64>| {
        stats.get(&role).and_then(|s| {
            let only: HashMap<Rgb, f64> = pick(s)
                .iter()
                .filter(|(c, _)| c.is_chromatic())
                .map(|(c, w)| (*c, *w))
                .collect();
            top_key(&only)
        })
    };

    // Ground + ink.
    if let Some((c, name)) = prop(BACKGROUND) {
        out.background = Some(c);
        out.note(format!("{label} background: {name}"));
    } else if let Some(c) = top(Role::Page, |s| &s.bg) {
        out.background = Some(c);
        out.note(format!("{label} background: body background"));
    }
    if let Some((c, name)) = prop(FOREGROUND) {
        out.foreground = Some(c);
        out.note(format!("{label} foreground: {name}"));
    } else if let Some(c) = top(Role::Page, |s| &s.text).or_else(|| top(Role::Heading, |s| &s.text))
    {
        out.foreground = Some(c);
        out.note(format!("{label} foreground: body text"));
    }

    // Brand colour, strongest evidence first.
    let background = out.background;
    let distinct = |c: &Rgb| background.is_none_or(|bg| c.distance(bg) > 40.0);
    let primary = prop(PRIMARY)
        .filter(|(c, _)| distinct(c))
        .or_else(|| {
            top_chromatic(Role::Button, |s| &s.bg)
                .filter(distinct)
                .map(|c| (c, "button background".to_string()))
        })
        .or_else(|| {
            top_chromatic(Role::Link, |s| &s.text)
                .filter(distinct)
                .map(|c| (c, "link colour".to_string()))
        })
        .or_else(|| {
            theme_color
                .filter(|c| c.is_chromatic() && distinct(c))
                .map(|c| (c, "meta theme-color".to_string()))
        })
        .or_else(|| {
            let chromatic: HashMap<Rgb, f64> = global
                .iter()
                .filter(|(c, _)| c.is_chromatic() && distinct(c))
                .map(|(c, w)| (*c, *w))
                .collect();
            top_key(&chromatic).map(|c| (c, "most used colour".to_string()))
        })
        .or_else(|| {
            top(Role::Button, |s| &s.bg)
                .filter(distinct)
                .map(|c| (c, "button background".to_string()))
        });
    if let Some((c, source)) = primary {
        out.primary = Some(c);
        out.note(format!("{label} primary: {source}"));
        // Button text over a button painted in the primary is the site's
        // own on-primary ink.
        let button_text = stats.get(&Role::Button).and_then(|s| top_key(&s.text));
        out.primary_foreground = prop(PRIMARY_FOREGROUND)
            .map(|(c, _)| c)
            .or(button_text.filter(|t| t.distance(c) > 60.0));
    }

    out.card = prop(CARD)
        .map(|(c, _)| c)
        .or_else(|| top(Role::Card, |s| &s.bg));
    out.border = prop(BORDER).map(|(c, _)| c).or_else(|| {
        [Role::Card, Role::Input, Role::Header]
            .iter()
            .find_map(|r| stats.get(r).and_then(|s| top_key(&s.border)))
    });
    out.muted = prop(MUTED).map(|(c, _)| c);
    out.muted_foreground = prop(MUTED_FOREGROUND).map(|(c, _)| c);
    out.destructive = prop(DESTRUCTIVE).map(|(c, _)| c);

    // `--secondary` / `--accent` mean subtle surfaces only in a shadcn
    // vocabulary; elsewhere they are usually a second brand colour.
    let secondary = prop(SECONDARY).map(|(c, _)| c);
    let accent = prop(ACCENT).map(|(c, _)| c);
    if shadcn_like {
        out.secondary = secondary;
        out.accent = accent;
    }
    let primary = out.primary;
    let push_extra = |extra: &mut Vec<Rgb>, c: Rgb| {
        let clashes = primary.is_some_and(|p| {
            p.distance(c) < 60.0 || (hue_distance(p, c) < 20.0 && p.distance(c) < 140.0)
        });
        if extra.len() < 4
            && c.is_chromatic()
            && !clashes
            && !extra.iter().any(|e| e.distance(c) < 60.0)
        {
            extra.push(c);
        }
    };
    let mut extra: Vec<Rgb> = Vec::new();
    if !shadcn_like {
        for c in [secondary, accent].into_iter().flatten() {
            push_extra(&mut extra, c);
        }
    }
    let mut ranked: Vec<(Rgb, f64)> = global.iter().map(|(c, w)| (*c, *w)).collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (c, _) in ranked {
        push_extra(&mut extra, c);
    }
    out.extra_brand = extra;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn lookup_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name: &str| map.get(name).cloned()
    }

    #[test]
    fn named_tokens_win_over_painted_evidence() {
        let lookup = lookup_from(&[("--brand", "#0E7C66"), ("--bg", "#FFFDF8")]);
        let mut stats = HashMap::new();
        let mut button = RoleStats::default();
        button.bg.insert(Rgb::new(200, 30, 30), 10.0);
        stats.insert(Role::Button, button);
        let mode = mode_from_evidence(&lookup, &stats, &HashMap::new(), None, false, "site");
        assert_eq!(mode.primary, Some(Rgb::new(0x0E, 0x7C, 0x66)));
        assert_eq!(mode.background, Some(Rgb::new(0xFF, 0xFD, 0xF8)));
        assert!(mode.provenance.iter().any(|p| p.contains("--brand")));
    }

    #[test]
    fn falls_back_to_button_then_link_then_theme_color() {
        let lookup = lookup_from(&[]);
        let mut stats = HashMap::new();
        let mut link = RoleStats::default();
        link.text.insert(Rgb::new(0xD9, 0x46, 0x0F), 3.0);
        stats.insert(Role::Link, link);
        let mode = mode_from_evidence(&lookup, &stats, &HashMap::new(), None, false, "site");
        assert_eq!(mode.primary, Some(Rgb::new(0xD9, 0x46, 0x0F)));

        let theme = Rgb::new(0x5B, 0x21, 0xB6);
        let mode = mode_from_evidence(
            &lookup,
            &HashMap::new(),
            &HashMap::new(),
            Some(theme),
            false,
            "site",
        );
        assert_eq!(mode.primary, Some(theme));
    }

    #[test]
    fn non_shadcn_accent_becomes_an_extra_brand_colour() {
        let lookup = lookup_from(&[("--primary", "#1D4ED8"), ("--accent", "#F59E0B")]);
        let mode = mode_from_evidence(
            &lookup,
            &HashMap::new(),
            &HashMap::new(),
            None,
            false,
            "site",
        );
        assert_eq!(mode.accent, None, "not a subtle surface");
        assert_eq!(mode.extra_brand, vec![Rgb::new(0xF5, 0x9E, 0x0B)]);
    }
}
