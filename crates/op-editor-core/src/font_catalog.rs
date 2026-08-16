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

/// Windows pairs where one font FILE ships two family NAMES.
///
/// Deliberately not a general `Name UI ≡ Name` fold: for most Windows
/// families the UI variant is a distinct face with different vertical
/// metrics (`Yu Gothic UI`, `Meiryo UI`, `Leelawadee UI`, `Segoe UI`).
/// Only `msyh.ttc` is known to register both `Microsoft YaHei` and
/// `Microsoft YaHei UI`.
pub const WINDOWS_UI_FAMILY_ALIASES: &[(&str, &str)] = &[("microsoft yahei", "microsoft yahei ui")];

/// Whether `left` and `right` are a documented same-file Windows alias pair.
pub fn is_windows_family_alias(left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();
    if left.is_empty() || right.is_empty() {
        return false;
    }
    WINDOWS_UI_FAMILY_ALIASES.iter().any(|(a, b)| {
        (left.eq_ignore_ascii_case(a) && right.eq_ignore_ascii_case(b))
            || (left.eq_ignore_ascii_case(b) && right.eq_ignore_ascii_case(a))
    })
}

/// Whether two concrete family names should be treated as the same installed
/// face for availability, picker highlight, and import mismatch notes.
///
/// Matching is ASCII-case-insensitive equality, plus the documented
/// same-file aliases in [`WINDOWS_UI_FAMILY_ALIASES`]. It is not a general
/// `Name UI ≡ Name` rule, and it allocates nothing.
pub fn is_same_font_family(authored: &str, available: &str) -> bool {
    let authored = authored.trim();
    let available = available.trim();
    !authored.is_empty()
        && !available.is_empty()
        && (authored.eq_ignore_ascii_case(available)
            || is_windows_family_alias(authored, available))
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
        assert!(is_windows_family_alias(
            "Microsoft YaHei",
            "Microsoft YaHei UI"
        ));
        assert!(!is_windows_family_alias("Yu Gothic", "Yu Gothic UI"));
    }

    #[test]
    fn same_family_keeps_distinct_families_distinct() {
        assert!(!is_same_font_family(
            "Microsoft YaHei",
            "Microsoft JhengHei"
        ));
        assert!(!is_same_font_family("Segoe UI", "Noto Sans UI"));
        // Trimming must not leave an empty match-all.
        assert!(!is_same_font_family("UI", ""));
        assert!(!is_same_font_family("", "ui"));
        assert!(!is_same_font_family("Microsoft YaHei", ""));
    }

    #[test]
    fn windows_ui_variant_is_not_a_universal_alias() {
        // The UI face is a distinct family for these Windows pairs.
        for (plain, ui) in [
            ("Yu Gothic", "Yu Gothic UI"),
            ("Segoe", "Segoe UI"),
            ("Meiryo", "Meiryo UI"),
            ("Leelawadee", "Leelawadee UI"),
        ] {
            assert!(
                !is_same_font_family(plain, ui),
                "{plain} must not equal {ui}"
            );
            assert!(
                !is_same_font_family(ui, plain),
                "{ui} must not equal {plain}"
            );
        }
    }
}
