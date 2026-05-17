//! Token budgeting — a faithful port of `engine/budget.ts`.
//!
//! Three stages: cap each skill at its own `budget`, then fill the
//! total budget by category priority — `Base` skills are always
//! kept, `Domain` skills fill the remainder in order (truncating the
//! last to fit), `Knowledge` skills are added only if room is left.

use crate::loader::SkillEntry;
use crate::types::{ResolvedSkill, SkillCategory};

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

/// Apply per-skill caps then fill `total_budget` by category priority.
pub fn trim_by_budget(skills: &[SkillEntry], total_budget: u32) -> Vec<ResolvedSkill> {
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

    // Step 3 — domain skills fill the remainder, truncating the last.
    for skill in with_tokens
        .iter()
        .filter(|s| s.meta.category == SkillCategory::Domain)
    {
        let remaining = total - used;
        if remaining <= 0 {
            break;
        }
        if (skill.token_count as i64) <= remaining {
            used += skill.token_count as i64;
            result.push(skill.clone());
        } else {
            let content = truncate_content(&skill.content, remaining as u32);
            let token_count = estimate_tokens(&content);
            used += token_count as i64;
            result.push(ResolvedSkill {
                meta: skill.meta.clone(),
                content,
                token_count,
                truncated: true,
            });
            break;
        }
    }

    // Step 4 — knowledge skills only if budget remains.
    for skill in with_tokens
        .iter()
        .filter(|s| s.meta.category == SkillCategory::Knowledge)
    {
        let remaining = total - used;
        if remaining <= 0 || (skill.token_count as i64) > remaining {
            break;
        }
        used += skill.token_count as i64;
        result.push(skill.clone());
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
        let out = trim_by_budget(&[skill("a", SkillCategory::Base, 10, &big)], 100_000);
        assert_eq!(out.len(), 1);
        assert!(out[0].truncated);
        assert!(out[0].token_count <= 10);
    }

    #[test]
    fn base_is_always_kept_even_over_budget() {
        let base = skill("base", SkillCategory::Base, 100_000, &"x".repeat(800));
        // Total budget of 1 token can't fit it, but base is kept.
        let out = trim_by_budget(&[base], 1);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].meta.name, "base");
    }

    #[test]
    fn knowledge_dropped_when_budget_is_tight() {
        let base = skill("base", SkillCategory::Base, 100_000, &"x".repeat(400)); // ~100 tok
        let knowledge = skill("k", SkillCategory::Knowledge, 100_000, &"y".repeat(400));
        // Budget only covers base.
        let out = trim_by_budget(&[base, knowledge], 100);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].meta.name, "base");
    }

    #[test]
    fn domain_fills_then_knowledge_when_room_remains() {
        let base = skill("base", SkillCategory::Base, 100_000, "short"); // ~2 tok
        let domain = skill("domain", SkillCategory::Domain, 100_000, &"d".repeat(40)); // ~10 tok
        let knowledge = skill("k", SkillCategory::Knowledge, 100_000, &"k".repeat(40)); // ~10 tok
        let out = trim_by_budget(&[base, domain, knowledge], 1000);
        let names: Vec<_> = out.iter().map(|s| s.meta.name.as_str()).collect();
        assert_eq!(names, vec!["base", "domain", "k"]);
    }
}
