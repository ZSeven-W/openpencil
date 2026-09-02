//! shadcn-vocabulary semantic palette — the B1 dictionary seeded into
//! every generated document (replaces the retired 56-token `color-*`
//! palette; the old names keep resolving via `op-editor-core`'s
//! legacy compat table so pre-B1 documents still render).
//!
//! Composition (74 tokens, `--`-prefixed, same vocabulary as the
//! bundled `design-systems.json` presets and `apply_design_system`):
//! - 35 themed colour tokens (shadcn core 19 + sidebar 8 + status 8)
//! - 1 themed overlay token (`--scrim`)
//! - 6 single chart tokens (`--chart-1..6`, shadcn `--chart-*` shape)
//! - 2 font tokens (`--font-primary/--font-secondary`) — string
//! - 30 numeric tokens: 18 typography + 2 tracking + 5 spacing
//!   (unchanged legacy numerics) + 5 radius (`--radius-none/xs/m/l/pill`)
//!
//! Theme-aware tokens carry BOTH values on the single `Mode: [Light,
//! Dark]` axis; single-value entries have no theme axis. The palette
//! is what `$--token` refs emitted by theme="system" element builders
//! resolve against (paint-time resolution lives in
//! `op-editor-ui::scene_vars::VariableTable`).

use jian_ops_schema::variable::{
    ThemedValue, VariableDefinition, VariableKind, VariableScalar, VariableValue,
};
use std::collections::BTreeMap;

/// The single theme axis the palette declares.
pub const THEME_AXIS: &str = "Mode";
pub const THEME_LIGHT: &str = "Light";
pub const THEME_DARK: &str = "Dark";

enum Entry {
    /// Theme-aware colour — (light hex, dark hex).
    LightDark(&'static str, &'static str),
    /// Single-value colour, no theme axis (chart palette).
    Color(&'static str),
    /// Single-value string, no theme axis (fonts).
    Str(&'static str),
    /// Numeric token, no theme axis (typography / spacing / radius).
    Num(f64),
}

/// Light/dark pair, kept one line per entry so the table reads as a
/// dictionary.
macro_rules! ld {
    ($light:expr, $dark:expr) => {
        Entry::LightDark($light, $dark)
    };
}

/// Raw value table — source of truth. Values carry the slate identity
/// the retired `color-*` palette had, re-keyed to shadcn roles
/// (`--primary` is the old brand accent, `--muted` the old
/// `--muted`, `--accent` the old hover surface …).
const PALETTE: &[(&str, Entry)] = &[
    // ── shadcn core ───────────────────────────────────────────────
    ("--background", ld!("#F8FAFC", "#0F172A")),
    ("--foreground", ld!("#0F172A", "#F1F5F9")),
    ("--card", ld!("#FFFFFF", "#1E293B")),
    ("--card-foreground", ld!("#0F172A", "#F1F5F9")),
    ("--popover", ld!("#FFFFFF", "#1E293B")),
    ("--popover-foreground", ld!("#0F172A", "#F1F5F9")),
    ("--primary", ld!("#2563EB", "#60A5FA")),
    ("--primary-foreground", ld!("#FFFFFF", "#0F172A")),
    ("--secondary", ld!("#F1F5F9", "#334155")),
    ("--secondary-foreground", ld!("#0F172A", "#F1F5F9")),
    ("--muted", ld!("#F1F5F9", "#334155")),
    ("--muted-foreground", ld!("#64748B", "#94A3B8")),
    ("--accent", ld!("#F3F4F6", "#475569")),
    ("--accent-foreground", ld!("#0F172A", "#F1F5F9")),
    ("--destructive", ld!("#EF4444", "#F87171")),
    ("--destructive-foreground", ld!("#FFFFFF", "#0F172A")),
    ("--border", ld!("#E2E8F0", "#334155")),
    ("--input", ld!("#E2E8F0", "#334155")),
    ("--ring", ld!("#2563EB", "#60A5FA")),
    // ── sidebar family ────────────────────────────────────────────
    ("--sidebar", ld!("#FFFFFF", "#1E293B")),
    ("--sidebar-foreground", ld!("#0F172A", "#F1F5F9")),
    ("--sidebar-primary", ld!("#2563EB", "#60A5FA")),
    ("--sidebar-primary-foreground", ld!("#FFFFFF", "#0F172A")),
    ("--sidebar-accent", ld!("#F1F5F9", "#334155")),
    ("--sidebar-accent-foreground", ld!("#0F172A", "#F1F5F9")),
    ("--sidebar-border", ld!("#E2E8F0", "#334155")),
    ("--sidebar-ring", ld!("#2563EB", "#60A5FA")),
    // ── status colours ────────────────────────────────────────────
    ("--color-success", ld!("#10B981", "#34D399")),
    ("--color-success-foreground", ld!("#FFFFFF", "#0F172A")),
    ("--color-warning", ld!("#F59E0B", "#FBBF24")),
    ("--color-warning-foreground", ld!("#FFFFFF", "#0F172A")),
    ("--color-error", ld!("#EF4444", "#F87171")),
    ("--color-error-foreground", ld!("#FFFFFF", "#0F172A")),
    ("--color-info", ld!("#3B82F6", "#60A5FA")),
    ("--color-info-foreground", ld!("#FFFFFF", "#0F172A")),
    // ── overlay ───────────────────────────────────────────────────
    ("--scrim", ld!("#00000080", "#00000099")),
    // ── Typography: size + weight + line-height × 6 roles ─────────
    ("type-display-size", Entry::Num(64.0)),
    ("type-display-weight", Entry::Num(700.0)),
    ("type-display-line-height", Entry::Num(1.0)),
    ("type-h1-size", Entry::Num(24.0)),
    ("type-h1-weight", Entry::Num(600.0)),
    ("type-h1-line-height", Entry::Num(1.2)),
    ("type-h2-size", Entry::Num(20.0)),
    ("type-h2-weight", Entry::Num(600.0)),
    ("type-h2-line-height", Entry::Num(1.25)),
    ("type-h3-size", Entry::Num(16.0)),
    ("type-h3-weight", Entry::Num(600.0)),
    ("type-h3-line-height", Entry::Num(1.3)),
    ("type-body-size", Entry::Num(14.0)),
    ("type-body-weight", Entry::Num(400.0)),
    ("type-body-line-height", Entry::Num(1.5)),
    ("type-caption-size", Entry::Num(12.0)),
    ("type-caption-weight", Entry::Num(400.0)),
    ("type-caption-line-height", Entry::Num(1.4)),
    // ── Border radius ─────────────────────────────────────────────
    ("--radius-none", Entry::Num(0.0)),
    ("--radius-xs", Entry::Num(4.0)),
    ("--radius-m", Entry::Num(8.0)),
    ("--radius-l", Entry::Num(12.0)),
    ("--radius-pill", Entry::Num(999.0)),
    // ── Spacing scale ─────────────────────────────────────────────
    ("spacing-1", Entry::Num(4.0)),
    ("spacing-2", Entry::Num(8.0)),
    ("spacing-3", Entry::Num(12.0)),
    ("spacing-4", Entry::Num(16.0)),
    ("spacing-5", Entry::Num(24.0)),
    // ── Sparse letterSpacing ──────────────────────────────────────
    ("type-display-letter-spacing", Entry::Num(-0.5)),
    ("type-uppercase-label-letter-spacing", Entry::Num(1.5)),
    // ── Fonts ─────────────────────────────────────────────────────
    ("--font-primary", Entry::Str("Inter")),
    ("--font-secondary", Entry::Str("Space Grotesk")),
    // ── Chart palette ─────────────────────────────────────────────
    ("--chart-1", Entry::Color("#3B82F6")),
    ("--chart-2", Entry::Color("#8B5CF6")),
    ("--chart-3", Entry::Color("#EC4899")),
    ("--chart-4", Entry::Color("#14B8A6")),
    ("--chart-5", Entry::Color("#F59E0B")),
    ("--chart-6", Entry::Color("#F97316")),
];

/// Every palette token name, table order.
pub fn palette_names() -> Vec<&'static str> {
    PALETTE.iter().map(|(name, _)| *name).collect()
}

fn themed(mode: &str, hex: &str) -> ThemedValue {
    ThemedValue {
        value: VariableScalar::Str(hex.to_string()),
        theme: Some(BTreeMap::from([(THEME_AXIS.to_string(), mode.to_string())])),
    }
}

/// One token's `VariableDefinition`; `None` for unknown names.
pub fn palette_variable(name: &str) -> Option<VariableDefinition> {
    let entry = PALETTE.iter().find(|(n, _)| *n == name).map(|(_, e)| e)?;
    Some(match entry {
        Entry::LightDark(light, dark) => VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                themed(THEME_LIGHT, light),
                themed(THEME_DARK, dark),
            ]),
        },
        Entry::Color(hex) => VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str(hex.to_string())),
        },
        Entry::Str(s) => VariableDefinition {
            kind: VariableKind::String,
            value: VariableValue::Scalar(VariableScalar::Str(s.to_string())),
        },
        Entry::Num(n) => VariableDefinition {
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(*n)),
        },
    })
}

/// The full palette as a variables map (for `MergeThemePreset`).
pub fn palette_variables() -> BTreeMap<String, VariableDefinition> {
    PALETTE
        .iter()
        .map(|(name, _)| {
            (
                name.to_string(),
                palette_variable(name).expect("table name"),
            )
        })
        .collect()
}

/// The `Mode: [Light, Dark]` theme axis declaration.
pub fn palette_themes() -> BTreeMap<String, Vec<String>> {
    BTreeMap::from([(
        THEME_AXIS.to_string(),
        vec![THEME_LIGHT.to_string(), THEME_DARK.to_string()],
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 74 tokens: 36 themed colours + 6 chart singles + 2 font
    /// strings + 30 numerics — the B1 shadcn dictionary.
    #[test]
    fn palette_inventory_matches_b1_dictionary() {
        let vars = palette_variables();
        assert_eq!(vars.len(), 74);
        let themed = vars
            .values()
            .filter(|v| matches!(v.value, VariableValue::Themed(_)))
            .count();
        let numbers = vars
            .values()
            .filter(|v| v.kind == VariableKind::Number)
            .count();
        assert_eq!(themed, 36, "19 shadcn core + 8 sidebar + 8 status + scrim");
        assert_eq!(numbers, 30, "18 type + 2 tracking + 5 spacing + 5 radius");
        assert_eq!(vars.len() - themed - numbers, 8, "6 chart + 2 font singles");
    }

    /// Spot-check values stay stable with the retired palette's
    /// identity: the brand accent moved to `--primary`, the page
    /// ground to `--background`.
    #[test]
    fn spot_values_verbatim() {
        let primary = palette_variable("--primary").unwrap();
        let VariableValue::Themed(pair) = &primary.value else {
            panic!("primary must be themed");
        };
        assert_eq!(pair[0].value, VariableScalar::Str("#2563EB".into()));
        assert_eq!(pair[1].value, VariableScalar::Str("#60A5FA".into()));
        assert_eq!(
            pair[0].theme.as_ref().unwrap().get(THEME_AXIS),
            Some(&THEME_LIGHT.to_string())
        );

        let body = palette_variable("type-body-size").unwrap();
        assert_eq!(body.value, VariableValue::Scalar(VariableScalar::Num(14.0)));

        let font = palette_variable("--font-primary").unwrap();
        assert_eq!(
            font.value,
            VariableValue::Scalar(VariableScalar::Str("Inter".into()))
        );

        let pill = palette_variable("--radius-pill").unwrap();
        assert_eq!(
            pill.value,
            VariableValue::Scalar(VariableScalar::Num(999.0))
        );
    }

    #[test]
    fn themes_declare_mode_axis() {
        let themes = palette_themes();
        assert_eq!(
            themes.get(THEME_AXIS),
            Some(&vec!["Light".to_string(), "Dark".to_string()])
        );
    }
}
