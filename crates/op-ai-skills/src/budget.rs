//! Token budgeting — a faithful port of `engine/budget.ts`.
//!
//! Three stages: cap each skill at its own `budget`, then fill the
//! total budget by category priority — `Base` skills are always
//! kept, `Domain` skills fill the remainder sorted by relevance
//! (skip-and-continue, one truncation max), `Knowledge` skills are
//! added only if room is left (skip-only, never truncated).

use crate::loader::SkillEntry;
use crate::resolver::match_keyword;
use crate::types::{ResolvedSkill, SkillCategory, SkillTrigger};

/// Estimate a string's token count with the TS `length / 4` heuristic.
/// Counts Unicode scalar values — the Rust analogue of JS string
/// `.length` for the BMP characters the skill corpus uses.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.chars().count().div_ceil(4)) as u32
}

/// Truncate `content` to roughly `max_tokens`, preferring to cut at a
/// newline when one falls in the second half of the window (so the
/// truncation lands on a clean line break).
fn truncate_content(content: &str, max_tokens: u32) -> String {
    let max_chars = max_tokens as usize * 4;
    let chars: Vec<char> = content.chars().collect();
    if chars.len() <= max_chars {
        return content.to_string();
    }
    let window = &chars[..max_chars];
    match window.iter().rposition(|&c| c == '\n') {
        Some(nl) if (nl as f64) > max_chars as f64 * 0.5 => window[..nl].iter().collect(),
        _ => window.iter().collect(),
    }
}

/// Score a skill's relevance to `intent`: each keyword hit counts once.
/// `Always`-trigger skills get a baseline of 1 so they aren't sorted
/// below keyword matches; `Flags` triggers carry no keyword signal.
fn relevance_score(meta: &crate::types::SkillMeta, intent: &str) -> i64 {
    match &meta.trigger {
        SkillTrigger::Always => 1,
        SkillTrigger::Flags(_) => 0,
        SkillTrigger::Keywords(keywords) => {
            let msg = intent.to_lowercase();
            keywords
                .iter()
                .filter(|kw| match_keyword(&msg, &kw.to_lowercase()))
                .count() as i64
        }
    }
}

/// Apply per-skill caps then fill `total_budget` by category priority.
/// `intent` is the resolution intent string (original prompt + subtask
/// label + hints); used to relevance-rank Domain / Knowledge skills.
pub fn trim_by_budget(
    skills: &[SkillEntry],
    total_budget: u32,
    intent: &str,
) -> Vec<ResolvedSkill> {
    // Step 1 — cap each skill at its own per-skill budget.
    let with_tokens: Vec<ResolvedSkill> = skills
        .iter()
        .map(|s| {
            let per = s.meta.budget;
            let raw = estimate_tokens(&s.content);
            let needs_truncate = raw > per;
            let content = if needs_truncate {
                truncate_content(&s.content, per)
            } else {
                s.content.clone()
            };
            let token_count = if needs_truncate {
                estimate_tokens(&content)
            } else {
                raw
            };
            ResolvedSkill {
                meta: s.meta.clone(),
                content,
                token_count,
                truncated: needs_truncate,
            }
        })
        .collect();

    let total = total_budget as i64;

    // Step 2 — base skills are always kept.
    let mut result: Vec<ResolvedSkill> = with_tokens
        .iter()
        .filter(|s| s.meta.category == SkillCategory::Base)
        .cloned()
        .collect();
    let mut used: i64 = result.iter().map(|s| s.token_count as i64).sum();

    // Step 3 — domain skills fill the remainder by (priority, relevance);
    // skip a candidate that doesn't fit and continue, so a large low-rank
    // skill can't block smaller, more relevant ones. The single
    // highest-relevance Domain skill that partially fits may be truncated
    // once to fill the tail.
    let mut domain: Vec<&ResolvedSkill> = with_tokens
        .iter()
        .filter(|s| s.meta.category == SkillCategory::Domain)
        .collect();
    domain.sort_by(|a, b| {
        a.meta
            .priority
            .cmp(&b.meta.priority)
            .then(relevance_score(&b.meta, intent).cmp(&relevance_score(&a.meta, intent)))
    });
    let mut domain_truncated = false;
    // Track the highest relevance of any domain skill that already fit fully,
    // so we only truncate an overflow candidate that is at least as relevant.
    let mut best_fit_relevance: i64 = i64::MIN;
    for skill in domain {
        let remaining = total - used;
        if remaining <= 0 {
            break;
        }
        let rel = relevance_score(&skill.meta, intent);
        if (skill.token_count as i64) <= remaining {
            used += skill.token_count as i64;
            result.push(skill.clone());
            if rel > best_fit_relevance {
                best_fit_relevance = rel;
            }
        } else if !domain_truncated && rel >= best_fit_relevance {
            // One truncation max: the most-relevant partial fit fills the tail,
            // but only if it is at least as relevant as what already fit fully.
            let content = truncate_content(&skill.content, remaining as u32);
            let token_count = estimate_tokens(&content);
            used += token_count as i64;
            result.push(ResolvedSkill {
                meta: skill.meta.clone(),
                content,
                token_count,
                truncated: true,
            });
            domain_truncated = true;
        }
        // else: skip this candidate, keep scanning.
    }

    // Step 4 — knowledge skills only if budget remains; relevance-ranked,
    // skip-only (never truncated).
    let mut knowledge: Vec<&ResolvedSkill> = with_tokens
        .iter()
        .filter(|s| s.meta.category == SkillCategory::Knowledge)
        .collect();
    knowledge.sort_by(|a, b| {
        a.meta
            .priority
            .cmp(&b.meta.priority)
            .then(relevance_score(&b.meta, intent).cmp(&relevance_score(&a.meta, intent)))
    });
    for skill in knowledge {
        let remaining = total - used;
        if remaining <= 0 {
            break;
        }
        if (skill.token_count as i64) <= remaining {
            used += skill.token_count as i64;
            result.push(skill.clone());
        }
        // else: skip and continue (a smaller later skill may still fit).
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Phase, SkillMeta, SkillTrigger};

    fn skill(name: &str, category: SkillCategory, budget: u32, content: &str) -> SkillEntry {
        SkillEntry {
            meta: SkillMeta {
                name: name.into(),
                description: String::new(),
                phase: vec![Phase::Generation],
                trigger: SkillTrigger::Always,
                priority: 50,
                budget,
                category,
            },
            content: content.into(),
        }
    }

    fn kw_skill(
        name: &str,
        cat: SkillCategory,
        budget: u32,
        kws: &[&str],
        content: &str,
    ) -> SkillEntry {
        let mut s = skill(name, cat, budget, content);
        s.meta.trigger = SkillTrigger::Keywords(kws.iter().map(|k| k.to_string()).collect());
        s
    }

    #[test]
    fn relevance_keeps_smaller_high_relevance_over_larger_low_priority() {
        let base = skill("base", SkillCategory::Base, 100_000, "short"); // ~2 tok
                                                                         // Corpus order would fit `big` first and skip `small`; relevance
                                                                         // must pick the keyword-matching `small` instead.
        let big = kw_skill(
            "big",
            SkillCategory::Domain,
            100_000,
            &["nope"],
            &"d".repeat(120),
        ); // ~30 tok
        let small = kw_skill(
            "small",
            SkillCategory::Domain,
            100_000,
            &["login"],
            &"d".repeat(40),
        ); // ~10 tok
        let out = trim_by_budget(&[base, big, small], 14, "design a login form");
        let names: Vec<_> = out.iter().map(|s| s.meta.name.as_str()).collect();
        // base (~2) + small (~10) fit in 14; big is skipped (doesn't fit
        // AND lower relevance), not truncated, since small is more relevant.
        assert_eq!(names, vec!["base", "small"]);
    }

    #[test]
    fn estimate_tokens_is_quarter_char_count() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2); // ceil(5/4)
    }

    #[test]
    fn per_skill_cap_truncates_oversized_content() {
        // 400 chars ≈ 100 tokens, capped at 10 tokens (≈40 chars).
        let big = "x".repeat(400);
        let out = trim_by_budget(&[skill("a", SkillCategory::Base, 10, &big)], 100_000, "");
        assert_eq!(out.len(), 1);
        assert!(out[0].truncated);
        assert!(out[0].token_count <= 10);
    }

    #[test]
    fn base_is_always_kept_even_over_budget() {
        let base = skill("base", SkillCategory::Base, 100_000, &"x".repeat(800));
        // Total budget of 1 token can't fit it, but base is kept.
        let out = trim_by_budget(&[base], 1, "");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].meta.name, "base");
    }

    #[test]
    fn knowledge_dropped_when_budget_is_tight() {
        let base = skill("base", SkillCategory::Base, 100_000, &"x".repeat(400)); // ~100 tok
        let knowledge = skill("k", SkillCategory::Knowledge, 100_000, &"y".repeat(400));
        // Budget only covers base.
        let out = trim_by_budget(&[base, knowledge], 100, "");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].meta.name, "base");
    }

    #[test]
    fn domain_fills_then_knowledge_when_room_remains() {
        let base = skill("base", SkillCategory::Base, 100_000, "short"); // ~2 tok
        let domain = skill("domain", SkillCategory::Domain, 100_000, &"d".repeat(40)); // ~10 tok
        let knowledge = skill("k", SkillCategory::Knowledge, 100_000, &"k".repeat(40)); // ~10 tok
        let out = trim_by_budget(&[base, domain, knowledge], 1000, "");
        let names: Vec<_> = out.iter().map(|s| s.meta.name.as_str()).collect();
        assert_eq!(names, vec!["base", "domain", "k"]);
    }

    #[test]
    fn trim_accepts_an_intent_argument() {
        let base = skill("base", SkillCategory::Base, 100_000, "short");
        // The new third arg is the resolution intent string.
        let out = trim_by_budget(&[base], 1000, "design a login form");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].meta.name, "base");
    }
}
