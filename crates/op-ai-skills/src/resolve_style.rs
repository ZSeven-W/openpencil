use std::collections::BTreeMap;

use crate::color::contrast::{on_color, scan_pairs, ContrastReport};
use crate::color::palettes::{palette_anchors, palette_names, PaletteAnchors, PaletteMode};

pub const STYLE_COUNT: usize = 7;

#[derive(Clone, Debug, PartialEq)]
pub struct Fonts {
    pub headings: String,
    pub body: String,
    pub captions: String,
    pub data: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleParams {
    pub color_palette: String,
    pub roundness: String,
    pub elevation: String,
    pub fonts: Fonts,
    pub decorative_imagery: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shadow {
    pub shadow_type: String,
    pub color: String,
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TokenMap {
    pub surface: BTreeMap<String, String>,
    pub foreground: BTreeMap<String, String>,
    pub accent: BTreeMap<String, String>,
    pub border: BTreeMap<String, String>,
    pub rounded: BTreeMap<String, f64>,
    pub shadow: BTreeMap<String, Shadow>,
    pub typography: Fonts,
    pub on: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleGuide {
    pub prose: String,
    pub tokens: TokenMap,
    pub contrast: ContrastReport,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ResolveOutcome {
    Hit(StyleGuide),
    Miss {
        missing: Vec<String>,
        suggest: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct StyleCatalogEntry {
    name: String,
    prose: String,
    roundness: String,
    elevation: String,
    fonts: Fonts,
}

pub fn resolve_style(name: &str, params: &StyleParams) -> ResolveOutcome {
    let styles = style_catalog_entries();
    let style = styles
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(name.trim()));
    let anchors = palette_anchors(&params.color_palette, PaletteMode::Light);

    let mut missing = Vec::new();
    if style.is_none() {
        missing.push(format!("style:{name}"));
    }
    if anchors.is_none() {
        missing.push(format!("palette:{}", params.color_palette));
    }
    if !missing.is_empty() {
        return ResolveOutcome::Miss {
            missing,
            suggest: suggestions(name, &params.color_palette, &styles),
        };
    }

    let style = style.expect("style checked above");
    let anchors = anchors.expect("palette checked above");
    let typography = effective_fonts(&style.fonts, &params.fonts);
    let tokens = build_tokens(
        &anchors,
        &effective_choice(&style.roundness, &params.roundness),
        &effective_choice(&style.elevation, &params.elevation),
        typography,
    );
    let contrast = scan_pairs(&contrast_pairs(&tokens));
    let prose = compose_prose(style, params, &tokens);

    ResolveOutcome::Hit(StyleGuide {
        prose,
        tokens,
        contrast,
    })
}

pub fn style_names() -> Vec<String> {
    style_catalog_entries()
        .into_iter()
        .map(|entry| entry.name)
        .collect()
}

fn style_catalog_entries() -> Vec<StyleCatalogEntry> {
    let mut entries: Vec<StyleCatalogEntry> = crate::STYLE_CATALOG
        .files()
        .filter(|file| {
            file.path()
                .extension()
                .map(|ext| ext == "md")
                .unwrap_or(false)
        })
        .filter_map(|file| file.contents_utf8())
        .filter_map(parse_style_catalog_entry)
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn parse_style_catalog_entry(text: &str) -> Option<StyleCatalogEntry> {
    let rest = text.strip_prefix("---\n")?;
    let (frontmatter, prose) = rest.split_once("\n---\n")?;
    let meta = parse_frontmatter(frontmatter);
    Some(StyleCatalogEntry {
        name: meta.get("name")?.to_string(),
        prose: prose.trim().to_string(),
        roundness: meta.get("roundness")?.to_string(),
        elevation: meta.get("elevation")?.to_string(),
        fonts: Fonts {
            headings: meta.get("headings")?.to_string(),
            body: meta.get("body")?.to_string(),
            captions: meta.get("captions")?.to_string(),
            data: meta.get("data")?.to_string(),
        },
    })
}

fn parse_frontmatter(frontmatter: &str) -> BTreeMap<String, String> {
    frontmatter
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            ))
        })
        .collect()
}

fn effective_choice(default: &str, requested: &str) -> String {
    if requested.trim().is_empty() {
        default.to_string()
    } else {
        requested.trim().to_ascii_lowercase()
    }
}

fn effective_fonts(defaults: &Fonts, requested: &Fonts) -> Fonts {
    Fonts {
        headings: effective_font(&defaults.headings, &requested.headings),
        body: effective_font(&defaults.body, &requested.body),
        captions: effective_font(&defaults.captions, &requested.captions),
        data: effective_font(&defaults.data, &requested.data),
    }
}

fn effective_font(default: &str, requested: &str) -> String {
    if requested.trim().is_empty() {
        default.to_string()
    } else {
        requested.trim().to_string()
    }
}

fn build_tokens(
    anchors: &PaletteAnchors,
    roundness: &str,
    elevation: &str,
    typography: Fonts,
) -> TokenMap {
    let mut surface = group_roles(&anchors.roles, "surface");
    let mut foreground = group_roles(&anchors.roles, "foreground");
    let mut accent = group_roles(&anchors.roles, "accent");
    let mut border = group_roles(&anchors.roles, "border");

    surface
        .entry("tertiary".to_string())
        .or_insert_with(|| anchors.neutral_scale[2].clone());
    border
        .entry("primary".to_string())
        .or_insert_with(|| anchors.neutral_scale[7].clone());
    accent
        .entry("secondary".to_string())
        .or_insert_with(|| anchors.accent_scale[11].clone());
    accent
        .entry("tertiary".to_string())
        .or_insert_with(|| anchors.accent_scale[3].clone());

    foreground
        .entry("primary".to_string())
        .or_insert_with(|| anchors.neutral_scale[11].clone());
    foreground
        .entry("secondary".to_string())
        .or_insert_with(|| anchors.neutral_scale[10].clone());
    foreground
        .entry("muted".to_string())
        .or_insert_with(|| anchors.neutral_scale[10].clone());
    foreground
        .entry("inverse".to_string())
        .or_insert_with(|| anchors.neutral_scale[0].clone());

    let on = on_roles(&surface, &accent, &border);

    TokenMap {
        surface,
        foreground,
        accent,
        border,
        rounded: rounded_tokens(roundness),
        shadow: shadow_tokens(elevation),
        typography,
        on,
    }
}

fn group_roles(roles: &BTreeMap<String, String>, group: &str) -> BTreeMap<String, String> {
    let prefix = format!("{group}.");
    roles
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix(&prefix)
                .map(|role| (role.to_string(), value.clone()))
        })
        .collect()
}

fn on_roles(
    surface: &BTreeMap<String, String>,
    accent: &BTreeMap<String, String>,
    border: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (group, roles) in [("surface", surface), ("accent", accent), ("border", border)] {
        for (role, value) in roles {
            out.insert(format!("on-{group}.{role}"), on_color(value).to_string());
        }
    }
    out
}

fn rounded_tokens(profile: &str) -> BTreeMap<String, f64> {
    let scale = match profile {
        "sharp" | "flat" | "none" => [0.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 9999.0],
        "large" | "soft" | "full" => [0.0, 8.0, 12.0, 16.0, 20.0, 28.0, 36.0, 9999.0],
        _ => [0.0, 4.0, 8.0, 12.0, 16.0, 24.0, 32.0, 9999.0],
    };
    ["none", "sm", "md", "lg", "xl", "2xl", "3xl", "full"]
        .into_iter()
        .zip(scale)
        .map(|(name, value)| (name.to_string(), value))
        .collect()
}

fn shadow_tokens(profile: &str) -> BTreeMap<String, Shadow> {
    let values = match profile {
        "flat" | "none" => [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0), (0.0, 0.0, 0.0)],
        "raised" | "deep" => [(0.0, 2.0, 8.0), (0.0, 8.0, 24.0), (0.0, 18.0, 48.0)],
        _ => [(0.0, 1.0, 4.0), (0.0, 4.0, 14.0), (0.0, 10.0, 30.0)],
    };
    ["sm", "md", "lg"]
        .into_iter()
        .zip(values)
        .map(|(name, (offset_x, offset_y, blur))| {
            (
                name.to_string(),
                Shadow {
                    shadow_type: "drop".to_string(),
                    color: "#0F172A24".to_string(),
                    offset_x,
                    offset_y,
                    blur,
                },
            )
        })
        .collect()
}

fn contrast_pairs(tokens: &TokenMap) -> Vec<(String, String, f64)> {
    let mut pairs = Vec::new();
    for fg in ["primary", "secondary", "muted"] {
        for bg in ["primary", "secondary", "tertiary"] {
            push_pair(
                &mut pairs,
                tokens.foreground.get(fg),
                tokens.surface.get(bg),
                4.5,
            );
        }
    }
    push_pair(
        &mut pairs,
        tokens.foreground.get("inverse"),
        tokens.surface.get("inverse"),
        4.5,
    );

    for (role, bg) in tokens
        .surface
        .iter()
        .map(|(role, bg)| (format!("on-surface.{role}"), bg))
        .chain(
            tokens
                .accent
                .iter()
                .map(|(role, bg)| (format!("on-accent.{role}"), bg)),
        )
    {
        if let Some(fg) = tokens.on.get(&role) {
            pairs.push((fg.clone(), bg.clone(), 4.5));
        }
    }
    pairs
}

fn push_pair(
    pairs: &mut Vec<(String, String, f64)>,
    fg: Option<&String>,
    bg: Option<&String>,
    target: f64,
) {
    if let (Some(fg), Some(bg)) = (fg, bg) {
        pairs.push((fg.clone(), bg.clone(), target));
    }
}

fn compose_prose(style: &StyleCatalogEntry, params: &StyleParams, tokens: &TokenMap) -> String {
    let decoration = params
        .decorative_imagery
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("use imagery only when it clarifies the product or content");

    format!(
        "{prose}\n\n## Resolved Defaults\n- color palette: {palette}\n- roundness: {roundness}\n- elevation: {elevation}\n- headings: {headings}\n- body: {body}\n- captions: {captions}\n- data: {data}\n- decoration role: {decoration}\n\n## Token Map usage notes\nUse the returned role maps as reference data for concrete fills, text colors, borders, radii, shadows, and fonts. Keep authored nodes concrete and local to the design output.",
        prose = style.prose,
        palette = params.color_palette,
        roundness = tokens
            .rounded
            .get("md")
            .map(|value| format!("md {value}px"))
            .unwrap_or_else(|| style.roundness.clone()),
        elevation = style.elevation,
        headings = tokens.typography.headings,
        body = tokens.typography.body,
        captions = tokens.typography.captions,
        data = tokens.typography.data,
    )
}

fn suggestions(
    style_query: &str,
    palette_query: &str,
    styles: &[StyleCatalogEntry],
) -> Vec<String> {
    let mut out = nearest(
        style_query,
        styles.iter().map(|entry| entry.name.clone()),
        3,
    );
    out.extend(nearest(
        palette_query,
        palette_names().into_iter().map(str::to_string),
        3,
    ));
    out.sort();
    out.dedup();
    out
}

fn nearest(query: &str, candidates: impl Iterator<Item = String>, limit: usize) -> Vec<String> {
    let query = query.trim().to_ascii_lowercase();
    let mut ranked: Vec<(usize, String)> = candidates
        .map(|candidate| {
            let score = levenshtein(&query, &candidate.to_ascii_lowercase());
            (score, candidate)
        })
        .collect();
    ranked.sort_by(|(score_a, name_a), (score_b, name_b)| {
        score_a.cmp(score_b).then_with(|| name_a.cmp(name_b))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, candidate)| candidate)
        .collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut costs: Vec<usize> = (0..=b.chars().count()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = costs[0];
        costs[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let insert = costs[j + 1] + 1;
            let delete = costs[j] + 1;
            let replace = prev + usize::from(ca != cb);
            prev = costs[j + 1];
            costs[j + 1] = insert.min(delete).min(replace);
        }
    }
    costs.last().copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::palettes::{palette_anchors, HARSH_PALETTE_NAME, HARSH_ROLE};
    use crate::loader::{get_skill_by_name, get_skill_registry};

    fn default_fonts() -> Fonts {
        Fonts {
            headings: "Inter".to_string(),
            body: "Inter".to_string(),
            captions: "Inter".to_string(),
            data: "IBM Plex Mono".to_string(),
        }
    }

    fn params_for(palette: &str) -> StyleParams {
        StyleParams {
            color_palette: palette.to_string(),
            roundness: "medium".to_string(),
            elevation: "low".to_string(),
            fonts: default_fonts(),
            decorative_imagery: Some("restrained product imagery".to_string()),
        }
    }

    fn assert_required_roles(tokens: &TokenMap) {
        for key in ["primary", "secondary", "tertiary", "inverse"] {
            assert!(tokens.surface.contains_key(key), "missing surface.{key}");
        }
        for key in ["primary", "secondary", "muted", "inverse"] {
            assert!(
                tokens.foreground.contains_key(key),
                "missing foreground.{key}"
            );
        }
        for key in ["primary", "secondary", "tertiary"] {
            assert!(tokens.accent.contains_key(key), "missing accent.{key}");
        }
        for key in ["primary", "subtle"] {
            assert!(tokens.border.contains_key(key), "missing border.{key}");
        }
        for key in ["none", "sm", "md", "lg", "xl", "2xl", "3xl", "full"] {
            assert!(tokens.rounded.contains_key(key), "missing rounded.{key}");
        }
        for key in ["sm", "md", "lg"] {
            assert!(tokens.shadow.contains_key(key), "missing shadow.{key}");
        }
    }

    #[test]
    fn resolve_our_style_structurally_complete() {
        let style = style_names().first().cloned().expect("style catalog names");
        let palette = palette_names()
            .first()
            .copied()
            .expect("palette catalog names");
        let ResolveOutcome::Hit(guide) = resolve_style(&style, &params_for(palette)) else {
            panic!("known style and palette must resolve");
        };

        assert_required_roles(&guide.tokens);
        assert!(!guide.prose.contains("set_variables"));
        assert!(!guide.prose.contains("EditorCommand"));
    }

    #[test]
    fn resolve_our_catalog_zero_aa_violations() {
        for style in style_names() {
            for palette in palette_names() {
                let ResolveOutcome::Hit(guide) = resolve_style(&style, &params_for(palette)) else {
                    panic!("{style} with {palette} must resolve");
                };
                assert!(
                    guide.contrast.violations.is_empty(),
                    "{style} / {palette} reported contrast violations: {:?}",
                    guide.contrast.violations
                );
            }
        }
    }

    #[test]
    fn resolve_original_palette_value_not_mutated() {
        let before = palette_anchors(HARSH_PALETTE_NAME, PaletteMode::Light)
            .expect("known harsh palette")
            .roles
            .get(HARSH_ROLE)
            .cloned()
            .expect("harsh explicit anchor");
        let style = style_names().first().cloned().expect("style catalog names");
        let ResolveOutcome::Hit(guide) = resolve_style(&style, &params_for(HARSH_PALETTE_NAME))
        else {
            panic!("known style and harsh palette must resolve");
        };

        assert_eq!(
            guide.tokens.accent.get("primary"),
            Some(&before),
            "explicit palette anchor must survive resolve unchanged"
        );
    }

    #[test]
    fn resolve_unknown_returns_miss_with_suggest() {
        let ResolveOutcome::Miss { missing, suggest } =
            resolve_style("not-a-real-style", &params_for("not-a-real-palette"))
        else {
            panic!("unknown style or palette must miss");
        };

        assert!(!missing.is_empty(), "miss must identify missing inputs");
        assert!(!suggest.is_empty(), "miss must return catalog suggestions");
    }

    #[test]
    fn resolve_style_returns_data_not_command() {
        let style = style_names().first().cloned().expect("style catalog names");
        let palette = palette_names()
            .first()
            .copied()
            .expect("palette catalog names");
        let outcome = resolve_style(&style, &params_for(palette));

        let type_name = std::any::type_name::<ResolveOutcome>();
        let debug = format!("{outcome:?}");
        assert!(!type_name.contains("EditorCommand"));
        assert!(!debug.contains("EditorCommand"));
        assert!(!debug.contains("set_variables"));
    }

    #[test]
    fn style_catalog_not_scanned_as_skill() {
        let names = style_names();
        assert_eq!(crate::count_md(&crate::STYLE_CATALOG), STYLE_COUNT);
        assert_eq!(names.len(), STYLE_COUNT);
        assert_eq!(palette_names().len(), crate::color::palettes::PALETTE_COUNT);

        for name in &names {
            assert!(
                get_skill_by_name(name).is_none(),
                "style catalog entry leaked into skill registry: {name}"
            );
        }
        assert!(get_skill_registry()
            .iter()
            .all(|entry| !names.contains(&entry.meta.name)));
    }
}
