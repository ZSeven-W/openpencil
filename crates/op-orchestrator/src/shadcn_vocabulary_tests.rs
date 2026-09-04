//! B1 acceptance tests — the shadcn variable dictionary end to end.
//!
//! Locks three surfaces to the `--token` vocabulary shared with the
//! bundled `design-systems.json` presets and `apply_design_system`:
//! the seeded semantic palette, the design-system seed commands, and
//! the skill corpus teaching. Legacy `color-*` render compatibility is
//! asserted over in `op-editor-core`'s resolver tests (compat table).

use crate::design_system::{default_design_system, design_system_to_seed_commands};
use crate::semantic_palette::palette_variables;
use op_editor_core::EditorCommand;

/// The full B1 dictionary: shadcn core + sidebar family + status
/// colours + chart + scrim + fonts + radius scale.
const SHADCN_TOKENS: &[&str] = &[
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
    "--color-success",
    "--color-success-foreground",
    "--color-warning",
    "--color-warning-foreground",
    "--color-error",
    "--color-error-foreground",
    "--color-info",
    "--color-info-foreground",
    "--scrim",
    "--chart-1",
    "--chart-2",
    "--chart-3",
    "--chart-4",
    "--chart-5",
    "--chart-6",
    "--font-primary",
    "--font-secondary",
    "--radius-none",
    "--radius-xs",
    "--radius-m",
    "--radius-l",
    "--radius-pill",
];

/// Legacy self-invented names that must no longer be seeded anywhere.
const LEGACY_NAMES: &[&str] = &[
    "color-background",
    "color-bg-deep",
    "color-surface",
    "color-surface-2",
    "color-surface-3",
    "color-text-primary",
    "color-text-body",
    "color-text-muted",
    "color-text-subtle",
    "color-accent",
    "color-destructive",
    "color-border",
    "color-border-strong",
    "color-danger-bg",
    "color-danger-text",
    "color-success-bg",
    "color-success-text",
    "color-warning-bg",
    "color-warning-text",
    "color-info-bg",
    "color-info-text",
    "color-scrim",
    "color-chart-1",
];

#[test]
fn semantic_palette_is_shadcn_vocabulary() {
    let vars = palette_variables();
    for token in SHADCN_TOKENS {
        assert!(
            vars.contains_key(*token),
            "seeded palette missing `{token}`"
        );
    }
    for legacy in LEGACY_NAMES {
        assert!(
            !vars.contains_key(*legacy),
            "legacy token `{legacy}` still seeded"
        );
    }
}

#[test]
fn design_system_seed_emits_shadcn_names() {
    let cmds = design_system_to_seed_commands(default_design_system());
    let names: Vec<String> = cmds
        .iter()
        .filter_map(|c| match c {
            EditorCommand::SetVariableColor { name, .. }
            | EditorCommand::SetVariableScalar { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        names.len(),
        cmds.len(),
        "seed emitted a non-variable command: {cmds:?}"
    );
    for token in [
        "--background",
        "--foreground",
        "--card",
        "--card-foreground",
        "--primary",
        "--primary-foreground",
        "--muted",
        "--muted-foreground",
        "--border",
        "--ring",
        "--sidebar",
        "--sidebar-ring",
        "--color-success",
        "--color-error",
        "--font-primary",
        "--font-secondary",
        "--radius-none",
        "--radius-xs",
        "--radius-m",
        "--radius-l",
        "--radius-pill",
    ] {
        assert!(
            names.iter().any(|n| n == token),
            "seed commands missing `{token}`; got {names:?}"
        );
    }
    for name in &names {
        assert!(
            !LEGACY_NAMES.contains(&name.as_str()),
            "seed emitted legacy name `{name}`"
        );
        // Colors / fonts / radius use the `--` shadcn prefix; the
        // numeric `spacing-*` scale is outside the B1 vocabulary and
        // keeps its legacy naming.
        assert!(
            name.starts_with("--") || name.starts_with("spacing-"),
            "seeded variable `{name}` must use the `--` shadcn prefix"
        );
    }
}

/// Every skill that teaches variable references must speak the new
/// vocabulary: no `$color-*` refs, at least one `$--token` ref.
/// `layout.md` is intentionally absent — see the B1 report (protected
/// file, one stale `$color-surface` left in place by the red line).
#[test]
fn skill_corpus_teaches_shadcn_refs() {
    for name in [
        "variables",
        "design-system-composition",
        "shapes-and-decks",
        "dashboard",
        "landing-page",
        "slides",
        "card-item-template",
    ] {
        let entry = op_ai_skills::get_skill_by_name(name)
            .unwrap_or_else(|| panic!("skill `{name}` missing from corpus"));
        assert!(
            !entry.content.contains("$color-"),
            "skill `{name}` still teaches legacy `$color-*` refs"
        );
        assert!(
            entry.content.contains("$--"),
            "skill `{name}` teaches no `$--token` refs"
        );
    }
}
