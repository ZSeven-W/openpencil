//! 56-token semantic palette — the Rust port of
//! `packages/pen-core/src/variables/semantic-palette.ts`.
//!
//! Composition (P1 token system, values verbatim from the TS table):
//! - 14 base color tokens (surface, border, text, accent, scrim) — light/dark
//! - 8 alert color tokens (info/success/warning/danger × bg/text) — light/dark
//! - 6 chart color tokens (color-chart-1..6) — single-value
//! - 18 typography tokens (size + weight + line-height × 6 roles) — numeric
//! - 2 letterSpacing tokens — numeric
//! - 5 spacing tokens (4/8/12/16/24) — numeric
//! - 3 radius tokens (4/8/12) — numeric
//!
//! Theme-aware tokens carry BOTH values on the single `Mode: [Light,
//! Dark]` axis; single-value entries have no theme axis. The palette is
//! what `$color-*` / `$type-*` refs emitted by theme="system" element
//! builders resolve against (paint-time resolution lives in
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
    /// Theme-aware color — (light hex, dark hex).
    LightDark(&'static str, &'static str),
    /// Single-value color, no theme axis (chart palette).
    Color(&'static str),
    /// Numeric token, no theme axis (typography / spacing / radius).
    Num(f64),
}

/// Raw value table — source of truth, kept in the same order as the TS
/// `PALETTE` constant so review diffs line up.
const PALETTE: &[(&str, Entry)] = &[
    ("color-surface", Entry::LightDark("#FFFFFF", "#1E293B")),
    ("color-surface-2", Entry::LightDark("#F1F5F9", "#334155")),
    ("color-surface-3", Entry::LightDark("#F3F4F6", "#475569")),
    ("color-bg-deep", Entry::LightDark("#F8FAFC", "#0F172A")),
    ("color-border", Entry::LightDark("#E2E8F0", "#334155")),
    (
        "color-border-strong",
        Entry::LightDark("#CBD5E1", "#475569"),
    ),
    ("color-text-primary", Entry::LightDark("#0F172A", "#F1F5F9")),
    ("color-text-body", Entry::LightDark("#334155", "#CBD5E1")),
    ("color-text-muted", Entry::LightDark("#64748B", "#94A3B8")),
    ("color-text-subtle", Entry::LightDark("#94A3B8", "#64748B")),
    ("color-accent", Entry::LightDark("#2563EB", "#60A5FA")),
    ("color-destructive", Entry::LightDark("#EF4444", "#F87171")),
    ("color-success", Entry::LightDark("#10B981", "#34D399")),
    ("color-scrim", Entry::LightDark("#00000080", "#00000099")),
    // ── Alert tokens ──────────────────────────────────────────────
    ("color-info-bg", Entry::LightDark("#DBEAFE", "#1E3A8A")),
    ("color-info-text", Entry::LightDark("#1E40AF", "#BFDBFE")),
    ("color-success-bg", Entry::LightDark("#DCFCE7", "#14532D")),
    ("color-success-text", Entry::LightDark("#166534", "#BBF7D0")),
    ("color-warning-bg", Entry::LightDark("#FEF3C7", "#78350F")),
    ("color-warning-text", Entry::LightDark("#92400E", "#FDE68A")),
    ("color-danger-bg", Entry::LightDark("#FEE2E2", "#7F1D1D")),
    ("color-danger-text", Entry::LightDark("#991B1B", "#FECACA")),
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
    ("radius-sm", Entry::Num(4.0)),
    ("radius-md", Entry::Num(8.0)),
    ("radius-lg", Entry::Num(12.0)),
    // ── Spacing scale ─────────────────────────────────────────────
    ("spacing-1", Entry::Num(4.0)),
    ("spacing-2", Entry::Num(8.0)),
    ("spacing-3", Entry::Num(12.0)),
    ("spacing-4", Entry::Num(16.0)),
    ("spacing-5", Entry::Num(24.0)),
    // ── Sparse letterSpacing ──────────────────────────────────────
    ("type-display-letter-spacing", Entry::Num(-0.5)),
    ("type-uppercase-label-letter-spacing", Entry::Num(1.5)),
    // ── Chart palette ─────────────────────────────────────────────
    ("color-chart-1", Entry::Color("#3B82F6")),
    ("color-chart-2", Entry::Color("#8B5CF6")),
    ("color-chart-3", Entry::Color("#EC4899")),
    ("color-chart-4", Entry::Color("#14B8A6")),
    ("color-chart-5", Entry::Color("#F59E0B")),
    ("color-chart-6", Entry::Color("#F97316")),
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

    /// 56 tokens: 22 themed colors + 6 chart colors + 28 numerics —
    /// the TS `SEMANTIC_PALETTE_NAMES` inventory.
    #[test]
    fn palette_inventory_matches_ts() {
        let vars = palette_variables();
        assert_eq!(vars.len(), 56);
        let themed = vars
            .values()
            .filter(|v| matches!(v.value, VariableValue::Themed(_)))
            .count();
        let numbers = vars
            .values()
            .filter(|v| v.kind == VariableKind::Number)
            .count();
        assert_eq!(themed, 22, "14 base + 8 alert light/dark pairs");
        assert_eq!(numbers, 28, "18 type + 2 tracking + 5 spacing + 3 radius");
        assert_eq!(vars.len() - themed - numbers, 6, "chart singles");
    }

    /// Spot-check values stay verbatim with the TS table.
    #[test]
    fn spot_values_verbatim() {
        let accent = palette_variable("color-accent").unwrap();
        let VariableValue::Themed(pair) = &accent.value else {
            panic!("accent must be themed");
        };
        assert_eq!(pair[0].value, VariableScalar::Str("#2563EB".into()));
        assert_eq!(pair[1].value, VariableScalar::Str("#60A5FA".into()));
        assert_eq!(
            pair[0].theme.as_ref().unwrap().get(THEME_AXIS),
            Some(&THEME_LIGHT.to_string())
        );

        let body = palette_variable("type-body-size").unwrap();
        assert_eq!(body.value, VariableValue::Scalar(VariableScalar::Num(14.0)));

        let chart = palette_variable("color-chart-1").unwrap();
        assert_eq!(
            chart.value,
            VariableValue::Scalar(VariableScalar::Str("#3B82F6".into()))
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
