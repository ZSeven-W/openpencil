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
    trim_by_budget_pinned(skills, total_budget, intent, &[])
}

/// [`trim_by_budget`] with a set of force-included ("pinned") skill names.
///
/// A pinned skill is kept exactly like a `Base` skill — never dropped by the
/// budget filler and never tier-trimmed downstream — regardless of category or
/// remaining headroom. This is the budget-exemption mechanism for skills whose
/// teaching is the *point* of a feature being active (e.g.
/// `component-composition` when a reusable-component library is loaded): the
/// model already received the component LIST, so the HOW-to-instantiate teaching
/// is not optional and must survive even on the tightest tier budget.
///
/// `pinned` is empty on every default path, so behaviour is byte-for-byte
/// identical to [`trim_by_budget`] when no skill is pinned.
pub fn trim_by_budget_pinned(
    skills: &[SkillEntry],
    total_budget: u32,
    intent: &str,
    pinned: &[&str],
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

    // A skill is force-kept when it is a `Base` skill OR its name is pinned.
    // Pinned + Base are treated identically: always kept, then excluded from the
    // category-priority fill below so they aren't added twice.
    let is_force_kept = |s: &ResolvedSkill| {
        s.meta.category == SkillCategory::Base || pinned.contains(&s.meta.name.as_str())
    };

    // Step 2 — base + pinned skills are always kept (budget-exempt).
    let mut result: Vec<ResolvedSkill> = with_tokens
        .iter()
        .filter(|s| is_force_kept(s))
        .cloned()
        .collect();
    let mut used: i64 = result.iter().map(|s| s.token_count as i64).sum();

    // Step 3 — domain skills fill the remainder by (priority, relevance);
    // skip a candidate that doesn't fit and continue, so a large low-rank
    // skill can't block smaller, more relevant ones. The single
    // highest-relevance Domain skill that partially fits may be truncated
    // once to fill the tail. Pinned domain skills are already kept above, so
    // they are excluded here to avoid a duplicate entry.
    let mut domain: Vec<&ResolvedSkill> = with_tokens
        .iter()
        .filter(|s| s.meta.category == SkillCategory::Domain && !is_force_kept(s))
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
    // skip-only (never truncated). A pinned Knowledge skill was already
    // force-kept in Step 2, so exclude it here to avoid a duplicate.
    let mut knowledge: Vec<&ResolvedSkill> = with_tokens
        .iter()
        .filter(|s| s.meta.category == SkillCategory::Knowledge && !is_force_kept(s))
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

    #[test]
    fn pinned_domain_skill_survives_a_budget_that_drops_it_unpinned() {
        // base (~100 tok) fills nearly the whole budget; the domain skill
        // (~10 tok) does not fit in the remaining headroom.
        let base = skill("base", SkillCategory::Base, 100_000, &"x".repeat(400)); // ~100 tok
        let comp = skill(
            "component-composition",
            SkillCategory::Domain,
            100_000,
            &"d".repeat(40),
        ); // ~10 tok
           // Budget 100 leaves 0 headroom after base — unpinned, comp is dropped.
        let unpinned = trim_by_budget(&[base.clone(), comp.clone()], 100, "");
        assert_eq!(
            unpinned
                .iter()
                .map(|s| s.meta.name.as_str())
                .collect::<Vec<_>>(),
            vec!["base"],
            "without a pin, the domain skill is budget-dropped"
        );
        // Pinned: comp is force-kept (budget-exempt) exactly like base.
        let pinned = trim_by_budget_pinned(&[base, comp], 100, "", &["component-composition"]);
        let names: Vec<_> = pinned.iter().map(|s| s.meta.name.as_str()).collect();
        assert!(
            names.contains(&"component-composition"),
            "pinned domain skill must survive the tight budget; got {names:?}"
        );
        assert!(names.contains(&"base"), "base is still kept");
    }

    #[test]
    fn pinned_skill_is_not_duplicated() {
        // A pinned domain skill that ALSO fits in budget must appear exactly
        // once, not twice (force-kept in Step 2 AND added in Step 3).
        let base = skill("base", SkillCategory::Base, 100_000, "short"); // ~2 tok
        let comp = skill(
            "component-composition",
            SkillCategory::Domain,
            100_000,
            &"d".repeat(40),
        ); // ~10 tok
        let out = trim_by_budget_pinned(&[base, comp], 100_000, "", &["component-composition"]);
        let count = out
            .iter()
            .filter(|s| s.meta.name == "component-composition")
            .count();
        assert_eq!(
            count, 1,
            "pinned skill must appear exactly once, got {count}"
        );
    }

    #[test]
    fn empty_pin_matches_unpinned_trim() {
        // Force-include with an empty pin set must be byte-for-byte identical
        // to the plain trim (the default no-library path is unchanged).
        let base = skill("base", SkillCategory::Base, 100_000, "short");
        let domain = skill("domain", SkillCategory::Domain, 100_000, &"d".repeat(40));
        let plain = trim_by_budget(&[base.clone(), domain.clone()], 1000, "intent");
        let empty_pin = trim_by_budget_pinned(&[base, domain], 1000, "intent", &[]);
        assert_eq!(plain, empty_pin);
    }
}
