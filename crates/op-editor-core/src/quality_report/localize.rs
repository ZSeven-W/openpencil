//! The report's item lines in the user's language.
//!
//! An item's `detail` is written for diagnostics: a remaining issue
//! carries the design-lint detector's English reason, a fix carries the
//! repair record's field-level `gap 24 → 16`. Neither is fit to show a
//! reader in another locale, so the panel renders through here instead:
//!
//! - a remaining issue is named by its detector category, one localized
//!   sentence per category (the English reason stays in `detail` for the
//!   log);
//! - a fix keeps its values (numbers and hex colours read the same in
//!   every language) and translates the field names in front of them;
//!   a fix whose shape is not a plain field change — a subtree diff, a
//!   removed or moved node — reads as a localized "adjusted
//!   automatically" rather than leaking English prose.

use super::QualityItem;
use crate::Locale;

/// Remaining-issue source (a design-lint category the report shows, or
/// the audit's `unfilled-screen`) → the i18n key of its sentence. Kept
/// in step with the topic table's `LINT_TOPICS` by a test.
pub(super) const ISSUE_KEYS: &[(&str, &str)] = &[
    (
        "text-bg-contrast",
        "workspace.quality.issue.text-bg-contrast",
    ),
    (
        "edge-section-padding",
        "workspace.quality.issue.edge-section-padding",
    ),
    (
        "stacked-horizontal-padding",
        "workspace.quality.issue.stacked-horizontal-padding",
    ),
    (
        "mixed-sibling-padding",
        "workspace.quality.issue.mixed-sibling-padding",
    ),
    (
        "text-explicit-height",
        "workspace.quality.issue.text-explicit-height",
    ),
    (
        "unexpected-rotation",
        "workspace.quality.issue.unexpected-rotation",
    ),
    ("text-effect", "workspace.quality.issue.text-effect"),
    ("text-stroke", "workspace.quality.issue.text-stroke"),
    (
        "text-corner-radius",
        "workspace.quality.issue.text-corner-radius",
    ),
    (
        "empty-filled-panel",
        "workspace.quality.issue.empty-filled-panel",
    ),
    (
        "invisible-container",
        "workspace.quality.issue.invisible-container",
    ),
    (
        "sibling-inconsistency",
        "workspace.quality.issue.sibling-inconsistency",
    ),
    (
        "mixed-sibling-corner-radius",
        "workspace.quality.issue.mixed-sibling-corner-radius",
    ),
    (
        "excessive-frame-effects",
        "workspace.quality.issue.excessive-frame-effects",
    ),
    ("motion-budget", "workspace.quality.issue.motion-budget"),
    ("shader-budget", "workspace.quality.issue.shader-budget"),
    ("shader-invalid", "workspace.quality.issue.shader-invalid"),
    (
        "top-anchored-bars",
        "workspace.quality.issue.top-anchored-bars",
    ),
    (
        "no-baseline-bars",
        "workspace.quality.issue.no-baseline-bars",
    ),
    ("widget-a11y", "workspace.quality.issue.widget-a11y"),
    ("unfilled-screen", "workspace.quality.issue.unfilled-screen"),
];

/// Repair-record field token → the i18n key naming it. Several wire
/// fields share one user word (`fills` / `backgroundColor` are both the
/// fill; `justifyContent` / `alignItems` are both alignment).
const FIELD_KEYS: &[(&str, &str)] = &[
    ("fill", "workspace.quality.field.fill"),
    ("fills", "workspace.quality.field.fill"),
    ("backgroundColor", "workspace.quality.field.fill"),
    ("stroke", "workspace.quality.field.stroke"),
    ("strokeWidth", "workspace.quality.field.strokeWidth"),
    ("padding", "workspace.quality.field.padding"),
    ("gap", "workspace.quality.field.gap"),
    ("layoutMode", "workspace.quality.field.layout"),
    ("layout", "workspace.quality.field.layout"),
    ("justifyContent", "workspace.quality.field.alignment"),
    ("alignItems", "workspace.quality.field.alignment"),
    ("clipContent", "workspace.quality.field.clip"),
    ("width", "workspace.quality.field.width"),
    ("height", "workspace.quality.field.height"),
    ("x", "workspace.quality.field.position"),
    ("y", "workspace.quality.field.position"),
    ("opacity", "workspace.quality.field.opacity"),
    ("cornerRadius", "workspace.quality.field.cornerRadius"),
    ("fontSize", "workspace.quality.field.fontSize"),
    ("fontWeight", "workspace.quality.field.fontWeight"),
    ("text", "workspace.quality.field.text"),
    ("name", "workspace.quality.field.name"),
    ("rotation", "workspace.quality.field.rotation"),
];

/// How the repair record spells an absent value.
const UNSET: &str = "(unset)";

/// The i18n key of a remaining issue's sentence, when its source is a
/// category the report knows.
pub fn issue_key(source: &str) -> Option<&'static str> {
    ISSUE_KEYS
        .iter()
        .find(|(name, _)| *name == source)
        .map(|(_, key)| *key)
}

fn field_key(field: &str) -> Option<&'static str> {
    FIELD_KEYS
        .iter()
        .find(|(name, _)| *name == field)
        .map(|(_, key)| *key)
}

/// One `field before → after` (or `field → after`) change with its field
/// name localized, or `None` when the segment is not that shape.
fn localize_change(segment: &str, locale: Locale) -> Option<String> {
    let segment = segment.trim();
    if !segment.contains('→') {
        return None;
    }
    let (field, rest) = segment.split_once(' ')?;
    let key = field_key(field)?;
    let rest = rest.replace(UNSET, op_i18n::translate(locale, "workspace.quality.unset"));
    Some(format!("{} {rest}", op_i18n::translate(locale, key)))
}

/// A fix's detail in `locale`: every comma-separated field change
/// localized, or the generic line when any part is not a field change.
fn localize_fix_detail(detail: &str, locale: Locale) -> String {
    let detail = detail.trim();
    if detail.is_empty() {
        return String::new();
    }
    let parts: Option<Vec<String>> = detail
        .split(", ")
        .map(|segment| localize_change(segment, locale))
        .collect();
    match parts {
        Some(parts) => parts.join(", "),
        None => op_i18n::translate(locale, "workspace.quality.fixedGeneric").to_string(),
    }
}

impl QualityItem {
    /// The detail line in `locale` — see the module docs for the rules.
    /// `fixed` says which half of the report the item belongs to.
    pub fn localized_detail(&self, locale: Locale, fixed: bool) -> String {
        if !fixed {
            if let Some(key) = issue_key(&self.source) {
                return op_i18n::translate(locale, key).to_string();
            }
        }
        localize_fix_detail(&self.detail, locale)
    }

    /// `Name · detail` in `locale`, or whichever half exists — the
    /// localized twin of [`QualityItem::label`] the panel paints.
    pub fn localized_label(&self, locale: Locale, fixed: bool) -> String {
        let name = self
            .node_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty());
        let detail = self.localized_detail(locale, fixed);
        match (name, detail.is_empty()) {
            (Some(name), false) => format!("{name} · {detail}"),
            (Some(name), true) => name.to_string(),
            (None, false) => detail,
            (None, true) => self.node_id.clone().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
#[path = "localize_tests.rs"]
mod tests;
