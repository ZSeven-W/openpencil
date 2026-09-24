//! Theme files for the `react-tailwind` target: a Tailwind v4 +
//! shadcn/ui `globals.css` binding the document's design variables as
//! CSS custom properties per theme, and the equivalent Tailwind v3
//! `tailwind.config.ts` colour / radius extension.
//!
//! Values come from the document's variable table resolved per theme
//! (default theme → `:root`, the dark axis value → `.dark`, any other
//! axis value → `[data-<axis>="<value>"]`). Tokens the markup references
//! but the document never defines fall back to the built-in shadcn
//! palette the canvas renders them with, so the exported page looks
//! like the canvas.

use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::variable::{VariableDefinition, VariableKind, VariableScalar, VariableValue};
use jian_ops_schema::PenDocument;
use op_editor_core::variables_resolve::{
    default_theme, has_palette_fallback, resolve_variable_ref,
};

use crate::fmt_num;

type Theme = BTreeMap<String, String>;

/// One bound design token.
#[derive(Debug, Clone)]
struct Token {
    /// CSS name without the leading dashes (`primary-foreground`).
    name: String,
    /// Document variable key when `defined`, else the palette `$ref`.
    key: String,
    /// Whether the document defines it (vs. palette fallback).
    defined: bool,
    color: bool,
}

/// Resolved theme blocks, ready to print.
#[derive(Debug, Default)]
pub(super) struct ThemeTables {
    root: Vec<(String, String)>,
    dark: Vec<(String, String)>,
    /// `(selector, declarations)` for non-dark axis values.
    variants: Vec<(String, Vec<(String, String)>)>,
    colors: Vec<String>,
    /// Whether `--radius` is bound (drives the shadcn radius ladder).
    has_radius: bool,
}

fn bare(name: &str) -> &str {
    name.trim_start_matches('-')
}

fn is_css_ident(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Numeric tokens whose name reads as a length get a `px` unit.
fn is_length_token(name: &str) -> bool {
    const UNITLESS: &[&str] = &[
        "weight",
        "line-height",
        "leading",
        "opacity",
        "z-",
        "ratio",
        "scale",
    ];
    const LENGTHS: &[&str] = &[
        "radius", "spacing", "space", "gap", "padding", "margin", "size", "width", "height",
        "inset", "offset", "blur", "stroke", "border",
    ];
    !UNITLESS.iter().any(|u| name.contains(u)) && LENGTHS.iter().any(|l| name.contains(l))
}

fn scalar_css(name: &str, scalar: &VariableScalar) -> Option<String> {
    match scalar {
        VariableScalar::Bool(_) => None,
        VariableScalar::Num(n) if is_length_token(name) => Some(format!("{}px", fmt_num(*n))),
        VariableScalar::Num(n) => Some(fmt_num(*n)),
        VariableScalar::Str(s) => {
            let s = s.trim();
            // Values land inside a CSS block that itself sits in a
            // comment; anything that could close either is dropped.
            (!s.is_empty()
                && !s.contains(['{', '}', ';'])
                && !s.contains("*/")
                && !s.contains("/*"))
            .then(|| s.to_string())
        }
    }
}

/// Parse a CSS length (`8`, `8px`, `0.5rem`) into px.
fn length_px(value: &str) -> Option<f64> {
    let value = value.trim();
    if let Some(rem) = value.strip_suffix("rem") {
        return rem.trim().parse::<f64>().ok().map(|v| v * 16.0);
    }
    value
        .strip_suffix("px")
        .unwrap_or(value)
        .trim()
        .parse::<f64>()
        .ok()
}

/// Resolve a document variable for `theme`. Unlike the canvas resolver
/// (which needs an entry naming *every* active axis), an entry matches
/// when each axis it names has the active value, and the most specific
/// match wins — so per-axis entries (`{Mode: Dark}` next to
/// `{Density: Compact}`) each resolve in a multi-axis document. A value
/// that is itself a `$ref` follows the reference a few levels deep.
fn resolve_defined(
    vars: &BTreeMap<String, VariableDefinition>,
    key: &str,
    theme: &Theme,
    depth: u8,
) -> Option<VariableScalar> {
    let def = vars.get(key)?;
    let scalar = match &def.value {
        VariableValue::Scalar(s) => s,
        VariableValue::Themed(entries) => {
            let mut best: Option<(usize, &VariableScalar)> = None;
            for entry in entries {
                let axes = entry.theme.as_ref();
                let matches = axes.is_none_or(|m| m.iter().all(|(a, v)| theme.get(a) == Some(v)));
                let specificity = axes.map_or(0, BTreeMap::len);
                if matches && best.is_none_or(|(s, _)| specificity > s) {
                    best = Some((specificity, &entry.value));
                }
            }
            best.map(|(_, v)| v)
                .or_else(|| entries.first().map(|e| &e.value))?
        }
    };
    match scalar {
        VariableScalar::Str(s) if s.starts_with('$') && depth < 4 => {
            resolve_defined(vars, &s[1..], theme, depth + 1)
        }
        other => Some(other.clone()),
    }
}

/// The document's `--radius` (or `radius`) token in px at the default
/// theme — the base of the shadcn radius ladder.
pub(super) fn document_radius_px(doc: &PenDocument) -> Option<f64> {
    let vars = doc.variables.as_ref()?;
    let key = vars.keys().find(|key| bare(key) == "radius")?;
    let theme = default_theme(Some(&axes(doc)));
    match resolve_defined(vars, key, &theme, 0)? {
        VariableScalar::Num(n) => Some(n),
        VariableScalar::Str(s) => length_px(&s),
        VariableScalar::Bool(_) => None,
    }
}

/// The theme axes and their ordered values: the document's declared
/// axes, else the `Mode: [Light, Dark]` axis the built-in palette uses.
fn axes(doc: &PenDocument) -> BTreeMap<String, Vec<String>> {
    match doc.themes.as_ref() {
        Some(themes) if !themes.is_empty() => themes.clone(),
        _ => BTreeMap::from([(
            "Mode".to_string(),
            vec!["Light".to_string(), "Dark".to_string()],
        )]),
    }
}

/// `(axis, value)` of the dark theme, when one axis has a `dark` value.
fn dark_axis(axes: &BTreeMap<String, Vec<String>>) -> Option<(String, String)> {
    axes.iter().find_map(|(axis, values)| {
        values
            .iter()
            .find(|v| v.eq_ignore_ascii_case("dark"))
            .map(|v| (axis.clone(), v.clone()))
    })
}

/// Palette fallbacks key off `Mode`; map a document dark theme onto it.
fn palette_theme(dark: bool) -> Theme {
    Theme::from([(
        "Mode".to_string(),
        if dark { "Dark" } else { "Light" }.to_string(),
    )])
}

fn collect_tokens(doc: &PenDocument, used: &BTreeSet<String>) -> Vec<Token> {
    let mut tokens: BTreeMap<String, Token> = BTreeMap::new();
    for (key, def) in doc.variables.iter().flatten() {
        let name = bare(key).to_string();
        if !is_css_ident(&name) || tokens.contains_key(&name) {
            continue;
        }
        tokens.insert(
            name.clone(),
            Token {
                name,
                key: key.clone(),
                defined: true,
                color: def.kind == VariableKind::Color,
            },
        );
    }
    for name in used {
        if tokens.contains_key(name) || !is_css_ident(name) {
            continue;
        }
        let reference = if has_palette_fallback(&format!("--{name}")) {
            format!("$--{name}")
        } else if has_palette_fallback(name) {
            format!("${name}")
        } else {
            continue;
        };
        let color = matches!(
            resolve_variable_ref(&reference, None, &palette_theme(false)),
            Some(VariableScalar::Str(ref s)) if s.starts_with('#')
        );
        tokens.insert(
            name.clone(),
            Token {
                name: name.clone(),
                key: reference,
                defined: false,
                color,
            },
        );
    }
    tokens.into_values().collect()
}

impl ThemeTables {
    pub(super) fn build(doc: &PenDocument, used: &BTreeSet<String>) -> Self {
        let axes = axes(doc);
        let default = default_theme(Some(&axes));
        let dark = dark_axis(&axes);
        let empty = BTreeMap::new();
        let vars = doc.variables.as_ref().unwrap_or(&empty);
        let tokens = collect_tokens(doc, used);
        let is_dark = |theme: &Theme| {
            dark.as_ref()
                .is_some_and(|(axis, value)| theme.get(axis) == Some(value))
        };
        let resolve = |token: &Token, theme: &Theme| -> Option<String> {
            let scalar = if token.defined {
                resolve_defined(vars, &token.key, theme, 0)?
            } else {
                resolve_variable_ref(&token.key, None, &palette_theme(is_dark(theme)))?
            };
            scalar_css(&token.name, &scalar)
        };
        let default_is_dark = is_dark(&default);

        let mut tables = ThemeTables::default();
        let mut root_values = BTreeMap::new();
        for token in &tokens {
            if let Some(value) = resolve(token, &default) {
                root_values.insert(token.name.clone(), value.clone());
                tables.root.push((token.name.clone(), value));
                if token.color {
                    tables.colors.push(token.name.clone());
                }
            }
        }
        // Same predicate the class mapper's `RadiusScale` keys off, so
        // `rounded-lg` means the same radius in markup and theme.
        tables.has_radius = document_radius_px(doc).is_some();
        if let Some((axis, value)) = &dark {
            if !default_is_dark {
                let mut theme = default.clone();
                theme.insert(axis.clone(), value.clone());
                tables.dark = themed_diff(&tokens, &root_values, |t| resolve(t, &theme));
            }
        }
        // Every other non-default axis value (including `Light` when the
        // document defaults to dark) gets a data-attribute block.
        for (axis, values) in &axes {
            for value in values.iter().skip(1) {
                if !default_is_dark && dark.as_ref() == Some(&(axis.clone(), value.clone())) {
                    continue;
                }
                let mut theme = default.clone();
                theme.insert(axis.clone(), value.clone());
                let decls = themed_diff(&tokens, &root_values, |t| resolve(t, &theme));
                if !decls.is_empty() {
                    let selector = format!("[data-{}=\"{}\"]", slug(axis), slug(value));
                    tables.variants.push((selector, decls));
                }
            }
        }
        tables
    }

    /// `app/globals.css` for Tailwind v4 + shadcn/ui.
    pub(super) fn globals_css(&self) -> String {
        let mut out = String::new();
        out.push_str("@import \"tailwindcss\";\n\n");
        out.push_str("@custom-variant dark (&:is(.dark *));\n");
        push_block(&mut out, ":root", &self.root);
        push_block(&mut out, ".dark", &self.dark);
        for (selector, decls) in &self.variants {
            push_block(&mut out, selector, decls);
        }
        let mut theme: Vec<(String, String)> = self
            .colors
            .iter()
            .map(|name| (format!("color-{name}"), format!("var(--{name})")))
            .collect();
        if self.has_radius {
            theme.extend(radius_steps().map(|(step, value)| (format!("radius-{step}"), value)));
        }
        if !theme.is_empty() {
            out.push_str("\n@theme inline {\n");
            for (name, value) in theme {
                out.push_str(&format!("  --{name}: {value};\n"));
            }
            out.push_str("}\n");
        }
        out
    }

    /// `tailwind.config.ts` for Tailwind v3 projects.
    pub(super) fn tailwind_config(&self) -> String {
        let mut out = String::new();
        out.push_str("import type { Config } from \"tailwindcss\"\n\n");
        out.push_str("const config = {\n");
        out.push_str("  darkMode: [\"class\"],\n");
        out.push_str(
            "  content: [\"./app/**/*.{ts,tsx}\", \"./components/**/*.{ts,tsx}\", \"./src/**/*.{ts,tsx}\"],\n",
        );
        out.push_str("  theme: {\n    extend: {\n");
        out.push_str("      colors: {\n");
        for name in &self.colors {
            out.push_str(&format!("        {}: \"var(--{name})\",\n", js_key(name)));
        }
        out.push_str("      },\n");
        if self.has_radius {
            out.push_str("      borderRadius: {\n");
            for (step, value) in radius_steps() {
                out.push_str(&format!("        {step}: \"{value}\",\n"));
            }
            out.push_str("      },\n");
        }
        out.push_str("    },\n  },\n} satisfies Config\n\nexport default config\n");
        out
    }
}

/// Declarations whose themed value differs from the root value.
fn themed_diff(
    tokens: &[Token],
    root: &BTreeMap<String, String>,
    resolve: impl Fn(&Token) -> Option<String>,
) -> Vec<(String, String)> {
    tokens
        .iter()
        .filter_map(|token| {
            let value = resolve(token)?;
            (root.get(&token.name) != Some(&value)).then(|| (token.name.clone(), value))
        })
        .collect()
}

/// shadcn's radius ladder derived from `--radius`.
fn radius_steps() -> impl Iterator<Item = (&'static str, String)> {
    [
        ("sm", "calc(var(--radius) - 4px)"),
        ("md", "calc(var(--radius) - 2px)"),
        ("lg", "var(--radius)"),
        ("xl", "calc(var(--radius) + 4px)"),
    ]
    .into_iter()
    .map(|(step, value)| (step, value.to_string()))
}

fn push_block(out: &mut String, selector: &str, decls: &[(String, String)]) {
    if decls.is_empty() {
        return;
    }
    out.push_str(&format!("\n{selector} {{\n"));
    for (name, value) in decls {
        out.push_str(&format!("  --{name}: {value};\n"));
    }
    out.push_str("}\n");
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// Object key: bare when it is a JS identifier, quoted otherwise.
fn js_key(name: &str) -> String {
    let ident = name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ident {
        name.to_string()
    } else {
        format!("\"{name}\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_tokens_get_px_and_ratios_stay_unitless() {
        assert!(is_length_token("radius"));
        assert!(is_length_token("spacing-4"));
        assert!(!is_length_token("font-weight-bold"));
        assert!(!is_length_token("line-height-body"));
        assert_eq!(length_px("0.5rem"), Some(8.0));
        assert_eq!(length_px("10px"), Some(10.0));
        assert_eq!(js_key("primary-foreground"), "\"primary-foreground\"");
        assert_eq!(js_key("card"), "card");
    }
}
