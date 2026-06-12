//! Deterministic element-kind nomination for the manifest sub-agent prompt.
//!
//! ab-v9.1 (openpencil-docs/model-benchmarks/2026-06-10-ab-v9-manifest)
//! showed element-adoption misses concentrate on briefs that DESCRIBE a
//! component instead of naming it (`setting_row` 10/10 missed) — yet the
//! literal kind tokens are almost always present in the planner's own
//! subtask text ("Notification Settings Row" / "toggle switch"), and even
//! a literal "toolbar" in the brief was missed 5/10. Recognition fails on
//! attention, not vocabulary — so this module does the recognition
//! deterministically: token-match catalog kind names against the subtask
//! text (plus a small synonym table for visual idioms that never carry
//! the kind's name) and surface the matches as ELEMENT HINTS lines in the
//! sub-agent user prompt.

use std::collections::BTreeSet;

/// Hard cap on hint count — hints are an anchor, not a checklist; past a
/// handful they read as noise and dilute the strong matches.
const MAX_HINTS: usize = 6;

/// Visual idiom → catalog kind, for kinds whose names don't appear
/// literally in the way people describe them. Grounded in the ab-v3
/// corpus briefs that missed: each phrase is how the brief actually
/// said it, kept generic enough to transfer.
const SYNONYM_PHRASES: &[(&str, &str)] = &[
    ("verification code", "otp_input"),
    ("one time password", "otp_input"),
    ("passcode", "otp_input"),
    ("pending invitation", "invite_row"),
    ("stacked avatar", "avatar_group"),
    ("overlapping avatar", "avatar_group"),
    ("avatar tile", "avatar_group"),
    ("facepile", "avatar_group"),
    ("profile hero", "profile_header"),
    ("profile page hero", "profile_header"),
    ("status indicator", "status_badge"),
    ("status pill", "status_badge"),
    ("status chip", "status_badge"),
    ("status dot", "status_badge"),
    ("metric cell", "metric_comparison"),
    ("kpi cell", "metric_comparison"),
    ("settings menu", "setting_row"),
    ("preference row", "setting_row"),
    ("hover hint", "tooltip"),
];

/// Nominate catalog kinds the text plausibly asks for, most specific
/// first, capped at [`MAX_HINTS`].
///
/// A kind token-matches when ALL of its `_`-separated name tokens appear
/// as words in the text (case- and trailing-plural-insensitive), so
/// "phone settings menu — one row" fires `setting_row` and "data-table
/// body row" fires `data_table_row`. Synonym phrases match as
/// space-bounded substrings (also tolerating a plural on the last word).
/// Specificity = matched token / phrase word count; multi-token kinds
/// outrank single-token ones so a composite (`setting_row`) lists before
/// its part (`switch`).
pub fn nominate_kinds(text: &str, kinds: &[String]) -> Vec<String> {
    let canon = canonical_text(text);
    if canon.is_empty() {
        return Vec::new();
    }
    let tokens: BTreeSet<String> = canon.split_whitespace().map(normalize_token).collect();

    // (specificity, kind) — token matches first.
    let mut scored: Vec<(usize, &str)> = Vec::new();
    for kind in kinds {
        let name_tokens: Vec<String> = kind.split('_').map(normalize_token).collect();
        if !name_tokens.is_empty() && name_tokens.iter().all(|t| tokens.contains(t)) {
            scored.push((name_tokens.len(), kind.as_str()));
        }
    }

    let padded = format!(" {canon} ");
    for (phrase, kind) in SYNONYM_PHRASES {
        // Catalog drift guard: never hint a kind the catalog can't build.
        if !kinds.iter().any(|k| k == kind) {
            continue;
        }
        let hit =
            padded.contains(&format!(" {phrase} ")) || padded.contains(&format!(" {phrase}s "));
        if hit {
            let words = phrase.split_whitespace().count();
            match scored.iter_mut().find(|(_, k)| k == kind) {
                Some(entry) => entry.0 = entry.0.max(words),
                None => scored.push((words, kind)),
            }
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    scored.truncate(MAX_HINTS);
    scored.into_iter().map(|(_, k)| k.to_string()).collect()
}

/// Lowercase, every non-alphanumeric run collapsed to a single space —
/// so "data-table", "data_table", and "Data Table" all canonicalize the
/// same way for both token and phrase matching.
fn canonical_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_space = true;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim_end().to_string()
}

/// Trailing-plural-insensitive token form, applied symmetrically to both
/// text and kind-name tokens ("settings" and "setting" both → "setting";
/// "progress" keeps its double-s).
fn normalize_token(t: &str) -> String {
    let t = t.to_ascii_lowercase();
    if t.len() > 3 && t.ends_with('s') && !t.ends_with("ss") {
        t[..t.len() - 1].to_string()
    } else {
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known() -> Vec<String> {
        op_mcp::element_manifest::known_element_kinds()
    }

    /// Kind-name tokens in the text fire the kind, plural- and
    /// punctuation-insensitive (the mobile-setting-row stable failure).
    #[test]
    fn token_match_fires_on_literal_kind_words() {
        let hints = nominate_kinds(
            "Design ONLY one row of a phone settings menu — the Notifications preference",
            &known(),
        );
        assert!(hints.contains(&"setting_row".to_string()), "{hints:?}");

        let hints = nominate_kinds(
            "a single body row of a desktop dashboard customers data-table",
            &known(),
        );
        assert!(hints.contains(&"data_table_row".to_string()), "{hints:?}");
    }

    /// Synonym phrases cover idioms that never carry the kind name
    /// (otp_input / avatar_group / status_badge stable failures).
    #[test]
    fn synonym_phrases_fire_without_kind_words() {
        let cases: &[(&str, &str)] = &[
            ("a 6-digit verification code input", "otp_input"),
            ("a stacked-avatar presence indicator", "avatar_group"),
            ("a status indicator: a small green dot", "status_badge"),
            ("one row of a Pending invitations table", "invite_row"),
        ];
        for (text, kind) in cases {
            let hints = nominate_kinds(text, &known());
            assert!(hints.contains(&kind.to_string()), "{text} → {hints:?}");
        }
    }

    /// Composite kinds (more name tokens) rank before their parts so the
    /// "most specific wins" prompt clause reads in hint order.
    #[test]
    fn multi_token_kinds_rank_before_single_token() {
        let hints = nominate_kinds(
            "Notification Settings Row with a trailing toggle switch",
            &known(),
        );
        let row = hints.iter().position(|k| k == "setting_row").unwrap();
        let sw = hints.iter().position(|k| k == "switch").unwrap();
        assert!(row < sw, "{hints:?}");
    }

    /// Hint volume is capped — a noun-soup brief can't flood the prompt.
    #[test]
    fn hints_cap_at_six() {
        let hints = nominate_kinds(
            "tag badge switch checkbox avatar divider spinner toast tabs link",
            &known(),
        );
        assert_eq!(hints.len(), MAX_HINTS);
    }

    /// No matches (or empty text) → empty, so the prompt block is omitted.
    #[test]
    fn no_match_returns_empty() {
        assert!(nominate_kinds("a pricing page", &known()).is_empty());
        assert!(nominate_kinds("   ", &known()).is_empty());
    }

    /// Every synonym target must exist in the live catalog; a rename in
    /// op-mcp should fail here, not silently stop hinting.
    #[test]
    fn synonym_targets_resolve_in_catalog() {
        let kinds = known();
        for (phrase, kind) in SYNONYM_PHRASES {
            assert!(
                kinds.iter().any(|k| k == kind),
                "synonym \"{phrase}\" targets unknown kind {kind}"
            );
        }
    }

    /// The drift guard drops synonym hits whose kind is missing from the
    /// provided catalog slice instead of hinting an unbuildable kind.
    #[test]
    fn synonym_skipped_when_kind_absent() {
        let only_switch = vec!["switch".to_string()];
        let hints = nominate_kinds("a 6-digit verification code input", &only_switch);
        assert!(hints.is_empty(), "{hints:?}");
    }
}
