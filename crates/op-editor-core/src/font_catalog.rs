//! Font-family names and CSS stack helpers shared by editor logic and UI.

/// Desktop's default app-shipped design-font catalog. Runtime availability
/// never trusts this list: hosts populate `EditorUiState::bundled_font_families`
/// from the renderer registry after registering their actual font blobs.
pub const BUNDLED_FONT_FAMILIES: [&str; 10] = [
    "Inter",
    "Space Grotesk",
    "Manrope",
    "Outfit",
    "DM Sans",
    "DM Serif Display",
    "DM Mono",
    "Instrument Serif",
    "JetBrains Mono",
    "Cormorant Garamond",
];

/// Split a CSS `font-family` value while preserving commas inside quoted
/// family names. Quotes are syntax and therefore omitted from the results;
/// a backslash escapes the following character.
pub fn split_font_family_stack(stack: &str) -> Vec<String> {
    let mut families = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in stack.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            ',' => push_family(&mut families, &mut current),
            _ => current.push(ch),
        }
    }
    if escaped {
        current.push('\\');
    }
    push_family(&mut families, &mut current);
    families
}

fn push_family(families: &mut Vec<String>, current: &mut String) {
    let family = current.trim();
    if !family.is_empty()
        && !families
            .iter()
            .any(|candidate: &String| candidate.eq_ignore_ascii_case(family))
    {
        families.push(family.to_string());
    }
    current.clear();
}

/// CSS generic families and browser/platform system aliases never require a
/// font-file import. The renderer resolves each to the current platform.
pub fn is_generic_or_system_font_alias(family: &str) -> bool {
    matches!(
        family.trim().to_ascii_lowercase().as_str(),
        "serif"
            | "sans-serif"
            | "cursive"
            | "fantasy"
            | "monospace"
            | "system-ui"
            | "ui-serif"
            | "ui-sans-serif"
            | "ui-monospace"
            | "ui-rounded"
            | "emoji"
            | "math"
            | "fangsong"
            | "-apple-system"
            | "blinkmacsystemfont"
    )
}

/// Whether two concrete family names denote the same installed face.
///
/// Matching ignores ASCII case and the Windows `… UI` family-name split:
/// one font file (e.g. `msyh.ttc`) ships under two family names —
/// `Microsoft YaHei` and `Microsoft YaHei UI` — and a system enumeration
/// may surface only one of the pair. A document authored with the other
/// spelling still resolves to the same face, so availability checks and
/// import mismatch notes must treat the pair as one family. The suffix
/// only matches as a separate trailing word (`Name UI`); a compound
/// (`NameUI`) stays distinct.
pub fn is_same_font_family(authored: &str, available: &str) -> bool {
    fn ui_folded_key(family: &str) -> String {
        let lowered = family.trim().to_ascii_lowercase();
        lowered
            .strip_suffix(" ui")
            .map(str::trim)
            .map(str::to_string)
            .unwrap_or(lowered)
    }
    let authored_key = ui_folded_key(authored);
    let available_key = ui_folded_key(available);
    !authored_key.is_empty() && !available_key.is_empty() && authored_key == available_key
}

/// First concrete authored family in a CSS stack. Generic fallbacks are
/// skipped because they cannot be supplied by a font file.
pub fn primary_concrete_font_family(stack: &str) -> Option<String> {
    let candidates = split_font_family_stack(stack);
    candidates
        .iter()
        .find(|family| !is_generic_or_system_font_alias(family))
        .or_else(|| candidates.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_parser_preserves_quoted_commas_and_ignores_case_when_deduping() {
        assert_eq!(
            split_font_family_stack(r#""ACME, Display", 'DM Sans', inter, INTER"#),
            vec!["ACME, Display", "DM Sans", "inter"]
        );
    }

    #[test]
    fn generic_and_system_alias_matching_is_case_insensitive() {
        for family in [
            "SANS-SERIF",
            "System-UI",
            "UI-SANS-SERIF",
            "-APPLE-SYSTEM",
            "BlinkMacSystemFont",
        ] {
            assert!(is_generic_or_system_font_alias(family), "{family}");
        }
    }

    #[test]
    fn primary_family_skips_generic_fallbacks() {
        assert_eq!(
            primary_concrete_font_family("system-ui, 'PingFang SC', sans-serif").as_deref(),
            Some("PingFang SC")
        );
    }

    #[test]
    fn same_family_matches_across_case_and_windows_ui_split() {
        // Issue #211: Windows surfaces `Microsoft YaHei UI` for msyh.ttc while
        // documents are authored with `Microsoft YaHei` (and vice versa).
        assert!(is_same_font_family("Microsoft YaHei", "Microsoft YaHei UI"));
        assert!(is_same_font_family("Microsoft YaHei UI", "Microsoft YaHei"));
        assert!(is_same_font_family("microsoft yahei", "Microsoft YaHei UI"));
        assert!(is_same_font_family("Segoe UI", "Segoe UI"));
        assert!(is_same_font_family(
            " Microsoft YaHei UI ",
            "microsoft yahei"
        ));
    }

    #[test]
    fn same_family_keeps_distinct_families_distinct() {
        assert!(!is_same_font_family(
            "Microsoft YaHei",
            "Microsoft JhengHei"
        ));
        assert!(!is_same_font_family("Segoe UI", "Noto Sans UI"));
        // `UI` must match only as a separate trailing word.
        assert!(!is_same_font_family("NameUI", "Name"));
        assert!(!is_same_font_family("Name", "NameUI"));
        // Trimming the suffix must not leave an empty match-all.
        assert!(!is_same_font_family("UI", ""));
        assert!(!is_same_font_family("", "ui"));
        assert!(!is_same_font_family("Microsoft YaHei", ""));
    }
}
