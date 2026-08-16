//! Intent matching — a faithful port of `engine/resolver.ts`.
//!
//! `match_keyword` reproduces the TS word-boundary semantics: an ASCII
//! keyword like `form` matches `a form here` but never `platform`;
//! a CJK keyword (no word boundaries) falls back to substring match.

use std::collections::HashMap;

use crate::loader::SkillEntry;
use crate::types::{SkillMeta, SkillTrigger};

/// ASCII word characters for the `\b` boundary check (`[A-Za-z0-9_]`).
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Match a single (already-lowercased) keyword against a lowercased
/// message. ASCII keywords match on word boundaries; non-ASCII (CJK)
/// keywords fall back to substring containment.
pub fn match_keyword(msg: &str, kw: &str) -> bool {
    // A `/.../`-delimited keyword is a regex pattern (the TS resolver runs it
    // as a real regex). The corpus uses exactly one — the CJK/Japanese/Korean
    // character class for `cjk-typography`. Rather than embed a regex engine,
    // evaluate it as "the message contains a CJK / JP / KR character" (the
    // pattern's intent). Without this the literal substring `/[一…]/` is
    // searched for verbatim and the skill never fires, so CJK designs lost
    // their CJK-typography guidance.
    if kw.len() >= 2 && kw.starts_with('/') && kw.ends_with('/') {
        return msg.chars().any(is_cjk_jp_kr_char);
    }
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

/// True when `c` is a Chinese / Japanese / Korean character — the union of
/// the ranges in `cjk-typography`'s regex keyword (CJK Unified, Hiragana,
/// Katakana, Hangul syllables).
fn is_cjk_jp_kr_char(c: char) -> bool {
    matches!(c as u32,
        0x4E00..=0x9FFF   // CJK Unified Ideographs
        | 0x3040..=0x309F // Hiragana
        | 0x30A0..=0x30FF // Katakana
        | 0xAC00..=0xD7AF) // Hangul syllables
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
        SkillTrigger::Flags(needed) => needed.iter().all(|f| flags.get(f).copied() == Some(true)),
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

// ── Model-family gate (DS P2-a overlay mechanism) ─────────────────────────────
//
// Strategic line: output contracts belong in the public corpus, model
// behaviour adaptation belongs in the DS experiment field. `skills/overlays/`
// is the test bed — a family-gated skill only reaches the model when the
// request's model id admits it; after ab validation graduates the teaching,
// it migrates into the public skills and the gate field goes away.

/// Normalize a model id for family matching: lowercase, `provider/` prefix
/// stripped — the same normalization the orchestrator's
/// `resolve_model_profile` applies, so the two cannot drift.
pub fn normalized_model_id(model_id: &str) -> String {
    let stripped = match model_id.find('/') {
        Some(i) => &model_id[i + 1..],
        None => model_id,
    };
    stripped.to_lowercase()
}

/// True when the normalized `model_id` contains `family` (lowercased) as a
/// substring — the overlay-gate match rule. An empty id (or family) never
/// matches, so a missing model never admits a gated skill.
pub fn model_id_matches_family(model_id: &str, family: &str) -> bool {
    let family = family.trim().to_lowercase();
    !family.is_empty() && !model_id.is_empty() && normalized_model_id(model_id).contains(&family)
}

/// The `model_families` gate on one skill: an ungated skill (no field — the
/// historical default) always passes; a gated skill passes only when
/// `model_id` matches one of its families.
pub fn model_family_match(meta: &SkillMeta, model_id: &str) -> bool {
    if meta.model_families.is_empty() {
        return true;
    }
    meta.model_families
        .iter()
        .any(|family| model_id_matches_family(model_id, family))
}

/// Split skills into `(candidates, gated_out)` by the `model_families` gate.
/// Gated-out skills never reach intent matching / budgeting; callers record
/// them as `DropReason::ModelFamilyMiss`.
pub fn filter_by_model_family(
    skills: &[SkillEntry],
    model_id: &str,
) -> (Vec<SkillEntry>, Vec<SkillEntry>) {
    let (kept, gated): (Vec<SkillEntry>, Vec<SkillEntry>) = skills
        .iter()
        .cloned()
        .partition(|skill| model_family_match(&skill.meta, model_id));
    (kept, gated)
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
                model_families: Vec::new(),
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
    fn cjk_regex_keyword_fires_on_cjk_jp_kr_message() {
        // `cjk-typography`'s `/[…]/` regex keyword must fire when the message
        // contains any CJK / Japanese / Korean character (it previously never
        // fired — the regex was searched for as a literal substring).
        let re = "/[\\u4e00-\\u9fff\\u3040-\\u309f\\u30a0-\\u30ff\\uac00-\\ud7af]/";
        assert!(match_keyword("生成美食应用首页", re), "chinese fires");
        assert!(match_keyword("デザイン", re), "japanese fires");
        assert!(match_keyword("디자인", re), "korean fires");
        assert!(
            !match_keyword("design a food app home", re),
            "pure-ascii message must not fire"
        );
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
            skill("kw", SkillTrigger::Keywords(vec!["dashboard".into()]), 10),
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

    // -----------------------------------------------------------------------
    // Model-family gate (DS P2-a overlays)
    // -----------------------------------------------------------------------

    #[test]
    fn model_id_normalization_lowercases_and_strips_provider_prefix() {
        assert_eq!(normalized_model_id("deepseek-v4-pro"), "deepseek-v4-pro");
        assert_eq!(
            normalized_model_id("Anthropic/DeepSeek-V4-Pro"),
            "deepseek-v4-pro"
        );
        assert_eq!(normalized_model_id(""), "");
    }

    #[test]
    fn family_match_is_case_insensitive_substring_on_the_normalized_id() {
        assert!(model_id_matches_family("deepseek-v4-pro", "deepseek"));
        assert!(model_id_matches_family("DEEPSEEK-V4-PRO", "DeepSeek"));
        assert!(model_id_matches_family(
            "anthropic/deepseek-v4-pro",
            "deepseek"
        ));
        assert!(!model_id_matches_family("glm-5.2", "deepseek"));
        // Empty id never admits a family (default "" = no overlay).
        assert!(!model_id_matches_family("", "deepseek"));
        assert!(!model_id_matches_family("deepseek-v4-pro", ""));
    }

    #[test]
    fn ungated_skills_always_pass_the_family_gate() {
        let ungated = skill("plain", SkillTrigger::Always, 50);
        assert!(model_family_match(&ungated.meta, ""));
        assert!(model_family_match(&ungated.meta, "glm-5.2"));
    }

    #[test]
    fn gated_skill_requires_a_matching_family() {
        let mut gated = skill("overlay", SkillTrigger::Always, 50);
        gated.meta.model_families = vec!["deepseek".into()];
        assert!(model_family_match(&gated.meta, "deepseek-v4-pro"));
        assert!(model_family_match(&gated.meta, "provider/deepseek-v4-pro"));
        assert!(!model_family_match(&gated.meta, "glm-5.2"));
        assert!(!model_family_match(&gated.meta, ""));
    }

    #[test]
    fn filter_by_model_family_partitions_on_the_gate() {
        let mut gated = skill("overlay", SkillTrigger::Always, 50);
        gated.meta.model_families = vec!["deepseek".into()];
        let plain = skill("plain", SkillTrigger::Always, 50);
        let skills = vec![plain.clone(), gated.clone()];

        let (kept, gated_out) = filter_by_model_family(&skills, "deepseek-v4-pro");
        assert_eq!(kept.len(), 2);
        assert!(gated_out.is_empty());

        let (kept, gated_out) = filter_by_model_family(&skills, "glm-5.2");
        assert_eq!(
            kept.iter()
                .map(|s| s.meta.name.as_str())
                .collect::<Vec<_>>(),
            vec!["plain"]
        );
        assert_eq!(gated_out.len(), 1);
        assert_eq!(gated_out[0].meta.name, "overlay");

        // Default empty model id: the gated skill never enters the candidate set.
        let (kept, gated_out) = filter_by_model_family(&skills, "");
        assert_eq!(kept.len(), 1);
        assert_eq!(gated_out.len(), 1);
    }
}
