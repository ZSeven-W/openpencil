//! Intent matching — a faithful port of `engine/resolver.ts`.
//!
//! `match_keyword` reproduces the TS word-boundary semantics: an ASCII
//! keyword like `form` matches `a form here` but never `platform`;
//! a CJK keyword (no word boundaries) falls back to substring match.

use std::collections::HashMap;

use crate::loader::SkillEntry;
use crate::types::SkillTrigger;

/// ASCII word characters for the `\b` boundary check (`[A-Za-z0-9_]`).
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Match a single (already-lowercased) keyword against a lowercased
/// message. ASCII keywords match on word boundaries; non-ASCII (CJK)
/// keywords fall back to substring containment.
pub fn match_keyword(msg: &str, kw: &str) -> bool {
    if !kw.is_ascii() {
        // CJK and friends have no whitespace word boundaries.
        return msg.contains(kw);
    }
    if kw.trim().is_empty() {
        return false;
    }
    let bytes = msg.as_bytes();
    let klen = kw.len();
    for (i, _) in msg.match_indices(kw) {
        let before_ok = i == 0 || !is_word_byte(bytes[i - 1]);
        let after = i + klen;
        // A multibyte (CJK) byte after the match is >= 0x80, so
        // `is_word_byte` returns false — a boundary, as intended.
        let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// True when `trigger` fires for the given message + flag set.
pub fn match_trigger(
    trigger: &SkillTrigger,
    user_message: &str,
    flags: &HashMap<String, bool>,
) -> bool {
    match trigger {
        SkillTrigger::Always => true,
        SkillTrigger::Keywords(keywords) => {
            let msg = user_message.to_lowercase();
            keywords
                .iter()
                .any(|kw| match_keyword(&msg, &kw.to_lowercase()))
        }
        // Every named flag must be present and `true`.
        SkillTrigger::Flags(needed) => {
            needed.iter().all(|f| flags.get(f).copied() == Some(true))
        }
    }
}

/// Keep the skills whose triggers fire, sorted by ascending priority
/// (lower number first — TS `filterByIntent`).
pub fn filter_by_intent(
    skills: &[SkillEntry],
    user_message: &str,
    flags: &HashMap<String, bool>,
) -> Vec<SkillEntry> {
    let mut matched: Vec<SkillEntry> = skills
        .iter()
        .filter(|s| match_trigger(&s.meta.trigger, user_message, flags))
        .cloned()
        .collect();
    matched.sort_by_key(|s| s.meta.priority);
    matched
}

/// Replace `{{key}}` placeholders in `content` with values from
/// `dynamic`. A missing key resolves to an empty string (TS parity).
/// An empty map leaves the content untouched — matching the TS
/// `if (!dynamicContent) return content` early-out.
pub fn inject_dynamic_content(content: &str, dynamic: &HashMap<String, String>) -> String {
    if dynamic.is_empty() {
        return content.to_string();
    }
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let key = &after[..end];
            let is_placeholder =
                !key.is_empty() && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
            if is_placeholder {
                out.push_str(dynamic.get(key).map(String::as_str).unwrap_or(""));
                rest = &after[end + 2..];
                continue;
            }
        }
        // Not a `{{word}}` placeholder — emit the literal `{{`.
        out.push_str("{{");
        rest = after;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Phase, SkillCategory, SkillMeta};

    fn skill(name: &str, trigger: SkillTrigger, priority: i32) -> SkillEntry {
        SkillEntry {
            meta: SkillMeta {
                name: name.into(),
                description: String::new(),
                phase: vec![Phase::Generation],
                trigger,
                priority,
                budget: 2000,
                category: SkillCategory::Domain,
            },
            content: String::new(),
        }
    }

    #[test]
    fn keyword_respects_word_boundaries() {
        assert!(match_keyword("design a form here", "form"));
        // Substring-only hits must NOT match.
        assert!(!match_keyword("a platform for information", "form"));
        assert!(!match_keyword("transform the format", "form"));
    }

    #[test]
    fn multi_word_keyword_matches() {
        assert!(match_keyword("please sign up now", "sign up"));
        assert!(match_keyword("a react-native app", "react-native"));
    }

    #[test]
    fn cjk_keyword_uses_substring() {
        assert!(match_keyword("帮我设计一个表单页面", "表单"));
        assert!(!match_keyword("帮我设计一个按钮", "表单"));
    }

    #[test]
    fn flags_trigger_requires_all_flags() {
        let mut flags = HashMap::new();
        flags.insert("isCodeGen".to_string(), true);
        let t = SkillTrigger::Flags(vec!["isCodeGen".into()]);
        assert!(match_trigger(&t, "anything", &flags));
        let t2 = SkillTrigger::Flags(vec!["isCodeGen".into(), "missing".into()]);
        assert!(!match_trigger(&t2, "anything", &flags));
    }

    #[test]
    fn always_trigger_fires_unconditionally() {
        assert!(match_trigger(&SkillTrigger::Always, "", &HashMap::new()));
    }

    #[test]
    fn filter_keeps_matches_and_sorts_by_priority() {
        let skills = vec![
            skill("late", SkillTrigger::Always, 90),
            skill(
                "kw",
                SkillTrigger::Keywords(vec!["dashboard".into()]),
                10,
            ),
            skill("early", SkillTrigger::Always, 5),
        ];
        let out = filter_by_intent(&skills, "build a dashboard", &HashMap::new());
        // All three match (two Always + one keyword); priority order.
        assert_eq!(
            out.iter().map(|s| s.meta.name.as_str()).collect::<Vec<_>>(),
            vec!["early", "kw", "late"]
        );
        // A message without the keyword drops the keyword skill.
        let out2 = filter_by_intent(&skills, "build a thing", &HashMap::new());
        assert_eq!(out2.len(), 2);
        assert!(out2.iter().all(|s| s.meta.name != "kw"));
    }

    #[test]
    fn dynamic_content_substitutes_and_blanks_missing() {
        let mut dynamic = HashMap::new();
        dynamic.insert("recentHistory".to_string(), "gen 1, gen 2".to_string());
        let out = inject_dynamic_content("before {{recentHistory}} after", &dynamic);
        assert_eq!(out, "before gen 1, gen 2 after");
        // Missing key → empty string.
        let out2 = inject_dynamic_content("x {{missing}} y", &dynamic);
        assert_eq!(out2, "x  y");
        // Empty map leaves the placeholder untouched.
        let out3 = inject_dynamic_content("x {{recentHistory}} y", &HashMap::new());
        assert_eq!(out3, "x {{recentHistory}} y");
    }
}
