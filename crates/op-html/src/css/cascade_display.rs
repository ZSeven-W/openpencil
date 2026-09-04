//! Supported CSS `display` grammar and browser-compatible serialization.

use super::cascade_shared::is_ident;

/// Parse-time validation for the display subset the importer can represent.
/// CSS-wide keywords and `var()` stay deferred until computed-value time.
pub(super) fn valid_specified_display(value: &str) -> bool {
    let trimmed = value.trim();
    matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "inherit" | "initial" | "unset" | "revert" | "revert-layer"
    ) || has_var_function(trimmed)
        || canonical_display_serialization(trimmed).is_some()
}

fn has_var_function(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.match_indices("var(").any(|(index, _)| {
        value[..index]
            .chars()
            .next_back()
            .is_none_or(|character| !is_ident(character) && character != '\\')
    })
}

/// Normalize recognized display keywords and serialize multi-keyword values
/// through browser-compatible legacy spellings.
pub(super) fn canonical_display_serialization(value: &str) -> Option<&'static str> {
    let mut parts = value.split_ascii_whitespace();
    let first = parts.next()?.to_ascii_lowercase();
    let Some(second) = parts.next() else {
        return canonical_display_keyword(&first);
    };
    let second = second.to_ascii_lowercase();
    if parts.next().is_some() {
        return None;
    }
    let (outside, inside) = if matches!(first.as_str(), "block" | "inline") {
        (first.as_str(), second.as_str())
    } else if matches!(second.as_str(), "block" | "inline") {
        (second.as_str(), first.as_str())
    } else {
        return None;
    };
    match (outside, inside) {
        ("block", "flow") => Some("block"),
        ("inline", "flow") => Some("inline"),
        ("block", "flow-root") => Some("flow-root"),
        ("inline", "flow-root") => Some("inline-block"),
        ("block", "flex") => Some("flex"),
        ("inline", "flex") => Some("inline-flex"),
        ("block", "grid") => Some("grid"),
        ("inline", "grid") => Some("inline-grid"),
        ("block", "table") => Some("table"),
        ("inline", "table") => Some("inline-table"),
        ("inline", "ruby") => Some("ruby"),
        _ => None,
    }
}

fn canonical_display_keyword(value: &str) -> Option<&'static str> {
    Some(match value {
        "none" => "none",
        "contents" => "contents",
        "block" => "block",
        "inline" => "inline",
        "inline-block" => "inline-block",
        "flow-root" => "flow-root",
        "flex" => "flex",
        "inline-flex" => "inline-flex",
        "grid" => "grid",
        "inline-grid" => "inline-grid",
        "ruby" => "ruby",
        "list-item" => "list-item",
        "table" => "table",
        "inline-table" => "inline-table",
        "table-row-group" => "table-row-group",
        "table-header-group" => "table-header-group",
        "table-footer-group" => "table-footer-group",
        "table-row" => "table-row",
        "table-cell" => "table-cell",
        "table-column-group" => "table-column-group",
        "table-column" => "table-column",
        "table-caption" => "table-caption",
        _ => return None,
    })
}
