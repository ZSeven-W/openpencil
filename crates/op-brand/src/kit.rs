//! The brand kit: resolved light + dark tokens, fonts, and radius scale,
//! and its three outputs — `.op` design variables (the shadcn vocabulary
//! generation already binds to), a design.md brief for the planner, and a
//! JSON report.

use std::collections::BTreeMap;

use jian_ops_schema::variable::{
    ThemedValue, VariableDefinition, VariableKind, VariableScalar, VariableValue,
};
use jian_ops_schema::{DesignMdColor, DesignMdSpec, DesignMdTypography};
use serde_json::{json, Value};

use crate::color::Rgb;
use crate::signals::BrandSignals;
use crate::tokens::{chart_colors, derive_alternate, resolve_mode, stand_out, ModeTokens};

/// Marker line in a kit-written design.md. A later kit may replace a
/// design.md carrying it; a user-written design.md is never overwritten.
/// `op-editor-core` keeps the same literal (asserted by a host test).
pub const BRAND_KIT_MARKER: &str = "<!-- openpencil:brand-kit -->";

/// The theme axis the bundled palette declares (`Mode: [Light, Dark]`).
pub const THEME_AXIS: &str = "Mode";
pub const THEME_LIGHT: &str = "Light";
pub const THEME_DARK: &str = "Dark";

/// Body font when the source only used a platform stack.
const FALLBACK_FONT: &str = "Inter";
/// Base corner radius when the source gave none.
const FALLBACK_RADIUS: f64 = 8.0;

/// Where a kit came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrandSource {
    Url(String),
    Image(String),
    Html(String),
}

impl BrandSource {
    pub fn label(&self) -> &str {
        match self {
            BrandSource::Url(s) | BrandSource::Image(s) | BrandSource::Html(s) => s,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            BrandSource::Url(_) => "url",
            BrandSource::Image(_) => "image",
            BrandSource::Html(_) => "html",
        }
    }
}

/// Whether a scheme was read from the source or derived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeOrigin {
    Extracted,
    Derived,
}

impl ThemeOrigin {
    fn as_str(self) -> &'static str {
        match self {
            ThemeOrigin::Extracted => "extracted",
            ThemeOrigin::Derived => "derived",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrandKit {
    pub name: String,
    pub source: BrandSource,
    pub light: ModeTokens,
    pub dark: ModeTokens,
    pub light_origin: ThemeOrigin,
    pub dark_origin: ThemeOrigin,
    pub charts: [Rgb; 6],
    /// How many leading `charts` are the source's own extra brand colours
    /// (the rest are default chart hues).
    pub extra_brand: usize,
    pub font_primary: String,
    pub font_secondary: String,
    /// Base radius (px) — `--radius-m`; the rest of the scale follows it.
    pub radius: f64,
    pub pill_buttons: bool,
    /// Provenance, one line per decision.
    pub notes: Vec<String>,
}

impl BrandKit {
    /// Resolve signals into a kit. `fallback_name` is used when the
    /// source carried no usable name (e.g. the URL host / file stem).
    pub fn from_signals(signals: BrandSignals, source: BrandSource, fallback_name: &str) -> Self {
        let base_dark = signals.base.background.is_some_and(|bg| bg.is_dark());
        let base_tokens = resolve_mode(&signals.base, base_dark);
        let mut notes = signals.base.provenance.clone();
        let alternate_tokens = match &signals.alternate {
            Some(alt) => {
                notes.extend(alt.provenance.iter().cloned());
                // A declared alternate theme often restyles only the ground
                // and ink; the brand colour then carries over, adjusted to
                // stand out on the new ground.
                let mut alt = alt.clone();
                if alt.primary.is_none() {
                    let ground = alt.background.unwrap_or(if base_dark {
                        crate::color::WHITE
                    } else {
                        crate::color::INK
                    });
                    alt.primary = base_tokens
                        .get("--primary")
                        .filter(|p| p.is_chromatic())
                        .map(|p| stand_out(p, ground, !base_dark));
                }
                (resolve_mode(&alt, !base_dark), ThemeOrigin::Extracted)
            }
            None => {
                let derived = derive_alternate(&base_tokens, &signals.base.extra_brand, !base_dark);
                notes.extend(derived.provenance.iter().cloned());
                (resolve_mode(&derived, !base_dark), ThemeOrigin::Derived)
            }
        };
        let (light, light_origin, dark, dark_origin) = if base_dark {
            (
                alternate_tokens.0,
                alternate_tokens.1,
                base_tokens,
                ThemeOrigin::Extracted,
            )
        } else {
            (
                base_tokens,
                ThemeOrigin::Extracted,
                alternate_tokens.0,
                alternate_tokens.1,
            )
        };
        let brand_primary = if base_dark {
            dark.get("--primary")
        } else {
            light.get("--primary")
        }
        .unwrap_or(Rgb::new(0x25, 0x63, 0xEB));
        notes.extend(signals.notes.iter().cloned());
        let font_primary = signals
            .font_body
            .clone()
            .unwrap_or_else(|| FALLBACK_FONT.to_string());
        let font_secondary = signals
            .font_heading
            .clone()
            .unwrap_or_else(|| font_primary.clone());
        let name = signals
            .name_hint
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| fallback_name.to_string());
        BrandKit {
            name,
            source,
            light,
            dark,
            light_origin,
            dark_origin,
            charts: chart_colors(brand_primary, &signals.base.extra_brand),
            extra_brand: signals.base.extra_brand.len().min(6),
            font_primary,
            font_secondary,
            radius: signals.radius.unwrap_or(FALLBACK_RADIUS).round(),
            pill_buttons: signals.pill_buttons,
            notes,
        }
    }

    /// The radius scale the palette names (`--radius-none/xs/m/l/pill`).
    pub fn radius_scale(&self) -> [(&'static str, f64); 5] {
        let base = self.radius.clamp(0.0, 32.0);
        let xs = if base == 0.0 {
            0.0
        } else {
            (base * 0.5).round().max(2.0)
        };
        [
            ("--radius-none", 0.0),
            ("--radius-xs", xs),
            ("--radius-m", base),
            ("--radius-l", (base * 1.5).round()),
            ("--radius-pill", 999.0),
        ]
    }

    /// Design variables in the bundled palette's vocabulary: 27 themed
    /// colours on the `Mode` axis, 6 chart colours, 2 fonts, 5 radii.
    pub fn variables(&self) -> BTreeMap<String, VariableDefinition> {
        let mut out = BTreeMap::new();
        for (name, light) in self.light.iter() {
            let dark = self.dark.get(name).unwrap_or(light);
            out.insert(
                name.to_string(),
                VariableDefinition {
                    kind: VariableKind::Color,
                    value: VariableValue::Themed(vec![
                        themed(THEME_LIGHT, light),
                        themed(THEME_DARK, dark),
                    ]),
                },
            );
        }
        for (i, c) in self.charts.iter().enumerate() {
            out.insert(
                format!("--chart-{}", i + 1),
                VariableDefinition {
                    kind: VariableKind::Color,
                    value: VariableValue::Scalar(VariableScalar::Str(c.to_hex())),
                },
            );
        }
        for (name, family) in [
            ("--font-primary", &self.font_primary),
            ("--font-secondary", &self.font_secondary),
        ] {
            out.insert(
                name.to_string(),
                VariableDefinition {
                    kind: VariableKind::String,
                    value: VariableValue::Scalar(VariableScalar::Str(family.clone())),
                },
            );
        }
        for (name, px) in self.radius_scale() {
            out.insert(
                name.to_string(),
                VariableDefinition {
                    kind: VariableKind::Number,
                    value: VariableValue::Scalar(VariableScalar::Num(px)),
                },
            );
        }
        out
    }

    /// The `Mode: [Light, Dark]` axis declaration.
    pub fn themes(&self) -> BTreeMap<String, Vec<String>> {
        BTreeMap::from([(
            THEME_AXIS.to_string(),
            vec![THEME_LIGHT.to_string(), THEME_DARK.to_string()],
        )])
    }

    /// Four colours that identify the kit at a glance (for UI chips):
    /// primary, background, foreground, then the first extra brand colour
    /// (or the accent surface).
    pub fn swatches(&self) -> Vec<String> {
        let get = |n: &str| self.light.get(n).map(Rgb::to_hex).unwrap_or_default();
        let fourth = if self.extra_brand > 0 {
            self.charts[0].to_hex()
        } else {
            get("--accent")
        };
        vec![
            get("--primary"),
            get("--background"),
            get("--foreground"),
            fourth,
        ]
    }

    /// A design.md brief for the planner: palette with roles, typography,
    /// component shapes, and the instruction to bind to variables.
    pub fn design_md(&self) -> DesignMdSpec {
        let hex = |n: &str| self.light.get(n).map(Rgb::to_hex).unwrap_or_default();
        let mut palette = vec![
            color(
                "Primary",
                hex("--primary"),
                "brand colour: primary buttons, links, active states — bind `$--primary`",
            ),
            color(
                "Primary Foreground",
                hex("--primary-foreground"),
                "text and icons on primary — `$--primary-foreground`",
            ),
            color(
                "Background",
                hex("--background"),
                "page background — `$--background`",
            ),
            color(
                "Foreground",
                hex("--foreground"),
                "headlines and body text — `$--foreground`",
            ),
            color(
                "Card",
                hex("--card"),
                "cards and raised surfaces — `$--card`",
            ),
            color(
                "Muted",
                hex("--muted"),
                "wells, inputs, quiet surfaces — `$--muted`",
            ),
            color(
                "Muted Foreground",
                hex("--muted-foreground"),
                "secondary text — `$--muted-foreground`",
            ),
            color(
                "Accent",
                hex("--accent"),
                "subtle brand-tinted hover surface — `$--accent`",
            ),
            color(
                "Border",
                hex("--border"),
                "dividers and outlines — `$--border`",
            ),
        ];
        for (i, c) in self.charts.iter().enumerate().take(self.extra_brand.min(2)) {
            palette.push(color(
                &format!("Secondary Brand {}", i + 1),
                c.to_hex(),
                &format!(
                    "secondary brand colour for highlights and charts — `$--chart-{}`",
                    i + 1
                ),
            ));
        }
        let scheme = if self.light.get("--background").is_some_and(|c| c.is_dark()) {
            "dark"
        } else {
            "light"
        };
        let visual_theme = format!(
            "Brand kit extracted from {}. {} ground with a {} brand colour. \
             Light theme {}, dark theme {} (both live on the `Mode` axis).",
            self.source.label(),
            capitalize(scheme),
            hex("--primary"),
            self.light_origin.as_str(),
            self.dark_origin.as_str(),
        );
        let typography = DesignMdTypography {
            font_family: Some(self.font_primary.clone()),
            headings: Some(format!("{} (`$--font-secondary`)", self.font_secondary)),
            body: Some(format!("{} (`$--font-primary`)", self.font_primary)),
            scale: Some(format!(
                "Headings in {}, body in {}.",
                self.font_secondary, self.font_primary
            )),
        };
        let button_shape = if self.pill_buttons {
            "pill-shaped (`$--radius-pill`)".to_string()
        } else {
            format!("{}px corners (`$--radius-m`)", self.radius)
        };
        let component_styles = format!(
            "Primary buttons: `$--primary` fill, `$--primary-foreground` label, {button_shape}. \
             Cards: `$--card` fill, `$--border` hairline, {}px corners (`$--radius-l`). \
             Inputs: `$--muted` or `$--background` fill with `$--input` stroke.",
            (self.radius * 1.5).round()
        );
        let generation_notes = "Bind every colour, font, and radius to the design variables above \
             ($--primary, $--background, $--foreground, $--card, $--border, $--font-primary, \
             $--radius-m …) instead of writing literal values: the brand is swapped later by \
             replacing the variable set, not by regenerating."
            .to_string();

        let mut raw = vec![
            BRAND_KIT_MARKER.to_string(),
            format!("# Design System: {}", self.name),
            String::new(),
            "## 1. Visual Theme & Atmosphere".into(),
            visual_theme.clone(),
            String::new(),
            "## 2. Color Palette & Roles".into(),
            String::new(),
        ];
        for c in &palette {
            raw.push(format!("- **{}** ({}) — {}", c.name, c.hex, c.role));
        }
        raw.extend([
            String::new(),
            "## 3. Typography Rules".into(),
            format!("**Primary Font Family:** {}", self.font_primary),
            typography.scale.clone().unwrap_or_default(),
            String::new(),
            "## 4. Component Stylings".into(),
            component_styles.clone(),
            String::new(),
            "## 6. Design System Notes".into(),
            generation_notes.clone(),
            String::new(),
        ]);
        DesignMdSpec {
            raw: raw.join("\n"),
            project_name: Some(self.name.clone()),
            visual_theme: Some(visual_theme),
            color_palette: Some(palette),
            typography: Some(typography),
            component_styles: Some(component_styles),
            layout_principles: None,
            generation_notes: Some(generation_notes),
        }
    }

    /// Machine-readable report (MCP / CLI output).
    pub fn to_json(&self) -> Value {
        let mode = |t: &ModeTokens| {
            Value::Object(
                t.iter()
                    .map(|(n, c)| (n.to_string(), Value::String(c.to_hex())))
                    .collect(),
            )
        };
        json!({
            "name": self.name,
            "source": { "kind": self.source.kind(), "value": self.source.label() },
            "light": mode(&self.light),
            "dark": mode(&self.dark),
            "lightOrigin": self.light_origin.as_str(),
            "darkOrigin": self.dark_origin.as_str(),
            "charts": self.charts.iter().map(|c| c.to_hex()).collect::<Vec<_>>(),
            "fonts": { "primary": self.font_primary, "secondary": self.font_secondary },
            "radius": self.radius_scale().iter().map(|(n, v)| (n.to_string(), json!(v))).collect::<serde_json::Map<_, _>>(),
            "pillButtons": self.pill_buttons,
            "swatches": self.swatches(),
            "notes": self.notes,
        })
    }
}

fn themed(mode: &str, color: Rgb) -> ThemedValue {
    ThemedValue {
        value: VariableScalar::Str(color.to_hex()),
        theme: Some(BTreeMap::from([(THEME_AXIS.to_string(), mode.to_string())])),
    }
}

fn color(name: &str, hex: String, role: &str) -> DesignMdColor {
    DesignMdColor {
        name: name.to_string(),
        hex,
        role: role.to_string(),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}
