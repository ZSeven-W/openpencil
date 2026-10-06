//! The built-in semantic-palette fallback tables.
//!
//! `DEFAULT_PALETTE_FALLBACK` is the shadcn-vocabulary dictionary
//! (B1): `--background` / `--card` / `--primary` / the `--sidebar-*`
//! family / `--color-<status>` pairs / `--chart-*` / `--font-*` /
//! `--radius-*`, with the same `Mode: [Light, Dark]` axis the bundled
//! `design-systems.json` presets use. `LEGACY_PALETTE_FALLBACK` keeps
//! the retired self-invented `color-*` names resolving at their
//! original values so pre-B1 documents render without visual drift.

/// A fallback entry's value shape: light/dark colour keyed off the
/// `Mode` theme axis, single colour, or plain number. Chart colours
/// and numeric typography / spacing / radius tokens are
/// mode-independent.
pub(crate) enum FallbackValue {
    LightDark {
        light: &'static str,
        dark: &'static str,
    },
    Single(&'static str),
    Num(f64),
}

/// Light/dark pair. Kept as a macro so the 40+ themed entries stay
/// one line each and read as a table.
macro_rules! ld {
    ($light:expr, $dark:expr) => {
        FallbackValue::LightDark {
            light: $light,
            dark: $dark,
        }
    };
}

pub(crate) const DEFAULT_PALETTE_FALLBACK: &[(&str, FallbackValue)] = &[
    // ── shadcn core (page / surfaces / brand) ─────────────────────
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
    ("--scrim", ld!("#00000080", "#00000099")),
    // ── chart palette ─────────────────────────────────────────────
    ("--chart-1", FallbackValue::Single("#3B82F6")),
    ("--chart-2", FallbackValue::Single("#8B5CF6")),
    ("--chart-3", FallbackValue::Single("#EC4899")),
    ("--chart-4", FallbackValue::Single("#14B8A6")),
    ("--chart-5", FallbackValue::Single("#F59E0B")),
    ("--chart-6", FallbackValue::Single("#F97316")),
    // ── fonts ─────────────────────────────────────────────────────
    ("--font-primary", FallbackValue::Single("Inter")),
    ("--font-secondary", FallbackValue::Single("Space Grotesk")),
    // ── radius scale ──────────────────────────────────────────────
    ("--radius-none", FallbackValue::Num(0f64)),
    ("--radius-xs", FallbackValue::Num(4f64)),
    ("--radius-m", FallbackValue::Num(8f64)),
    ("--radius-l", FallbackValue::Num(12f64)),
    ("--radius-pill", FallbackValue::Num(999f64)),
    // ── numerics outside the B1 colour vocabulary (unchanged) ─────
    ("spacing-1", FallbackValue::Num(4f64)),
    ("spacing-2", FallbackValue::Num(8f64)),
    ("spacing-3", FallbackValue::Num(12f64)),
    ("spacing-4", FallbackValue::Num(16f64)),
    ("spacing-5", FallbackValue::Num(24f64)),
    ("type-body-line-height", FallbackValue::Num(1.5f64)),
    ("type-body-size", FallbackValue::Num(14f64)),
    ("type-body-weight", FallbackValue::Num(400f64)),
    ("type-caption-line-height", FallbackValue::Num(1.4f64)),
    ("type-caption-size", FallbackValue::Num(12f64)),
    ("type-caption-weight", FallbackValue::Num(400f64)),
    ("type-display-letter-spacing", FallbackValue::Num(-0.5f64)),
    ("type-display-line-height", FallbackValue::Num(1f64)),
    ("type-display-size", FallbackValue::Num(64f64)),
    ("type-display-weight", FallbackValue::Num(700f64)),
    ("type-h1-line-height", FallbackValue::Num(1.2f64)),
    ("type-h1-size", FallbackValue::Num(24f64)),
    ("type-h1-weight", FallbackValue::Num(600f64)),
    ("type-h2-line-height", FallbackValue::Num(1.25f64)),
    ("type-h2-size", FallbackValue::Num(20f64)),
    ("type-h2-weight", FallbackValue::Num(600f64)),
    ("type-h3-line-height", FallbackValue::Num(1.3f64)),
    ("type-h3-size", FallbackValue::Num(16f64)),
    ("type-h3-weight", FallbackValue::Num(600f64)),
    (
        "type-uppercase-label-letter-spacing",
        FallbackValue::Num(1.5f64),
    ),
];

/// Pre-B1 `color-*` vocabulary, kept ONLY so documents authored before
/// the shadcn dictionary still render their `$color-*` refs at the
/// exact values they were written with. Never seeded into new
/// documents; lookup consults this table after
/// [`DEFAULT_PALETTE_FALLBACK`] misses.
pub(crate) const LEGACY_PALETTE_FALLBACK: &[(&str, FallbackValue)] = &[
    ("color-accent", ld!("#2563EB", "#60A5FA")),
    ("color-background", ld!("#F8FAFC", "#0F172A")),
    ("color-bg-deep", ld!("#F8FAFC", "#0F172A")),
    ("color-border", ld!("#E2E8F0", "#334155")),
    ("color-border-strong", ld!("#CBD5E1", "#475569")),
    ("color-chart-1", FallbackValue::Single("#3B82F6")),
    ("color-chart-2", FallbackValue::Single("#8B5CF6")),
    ("color-chart-3", FallbackValue::Single("#EC4899")),
    ("color-chart-4", FallbackValue::Single("#14B8A6")),
    ("color-chart-5", FallbackValue::Single("#F59E0B")),
    ("color-chart-6", FallbackValue::Single("#F97316")),
    ("color-danger-bg", ld!("#FEE2E2", "#7F1D1D")),
    ("color-danger-text", ld!("#991B1B", "#FECACA")),
    ("color-destructive", ld!("#EF4444", "#F87171")),
    ("color-info-bg", ld!("#DBEAFE", "#1E3A8A")),
    ("color-info-text", ld!("#1E40AF", "#BFDBFE")),
    ("color-primary", ld!("#2563EB", "#60A5FA")),
    ("color-scrim", ld!("#00000080", "#00000099")),
    ("color-success", ld!("#10B981", "#34D399")),
    ("color-success-bg", ld!("#DCFCE7", "#14532D")),
    ("color-success-text", ld!("#166534", "#BBF7D0")),
    ("color-surface", ld!("#FFFFFF", "#1E293B")),
    ("color-surface-2", ld!("#F1F5F9", "#334155")),
    ("color-surface-3", ld!("#F3F4F6", "#475569")),
    ("color-text-body", ld!("#334155", "#CBD5E1")),
    ("color-text-muted", ld!("#64748B", "#94A3B8")),
    ("color-text-primary", ld!("#0F172A", "#F1F5F9")),
    ("color-text-subtle", ld!("#94A3B8", "#64748B")),
    ("color-warning-bg", ld!("#FEF3C7", "#78350F")),
    ("color-warning-text", ld!("#92400E", "#FDE68A")),
    ("radius-lg", FallbackValue::Num(12f64)),
    ("radius-md", FallbackValue::Num(8f64)),
    ("radius-sm", FallbackValue::Num(4f64)),
];
