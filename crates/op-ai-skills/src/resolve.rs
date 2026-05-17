//! The resolution pipeline — port of `engine/resolve-skills.ts`.
//!
//! `resolve_skills` is the crate's main entry point: phase filter →
//! intent match → per-phase memory injection → budget trim.

use crate::budget::trim_by_budget;
use crate::loader::{get_skills_by_phase, SkillEntry};
use crate::memory::generation_history::get_recent_entries;
use crate::resolver::{filter_by_intent, inject_dynamic_content};
use crate::types::{AgentContext, Phase, ResolveMemory, ResolveOptions};

/// How many history entries each phase loads (TS `historyLimits`).
fn history_limit(phase: Phase) -> usize {
    match phase {
        Phase::Planning => 5,
        Phase::Generation | Phase::Maintenance => 3,
        Phase::Validation => 0,
    }
}

/// Format the loaded history into the `{{recentHistory}}` block the
/// anti-slop generation skills expect.
fn format_recent_history(history: &[crate::types::HistoryEntry]) -> String {
    if history.is_empty() {
        return "No recent history.".to_string();
    }
    history
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let mut parts = vec![format!("Generation {} ({}):", i + 1, h.timestamp)];
            if let Some(font) = &h.output.heading_font {
                parts.push(format!("  Heading font: {font}"));
            }
            if let Some(palette) = &h.output.palette {
                parts.push(format!("  Palette: {palette}"));
            }
            if let Some(variant) = &h.output.creative_variant {
                parts.push(format!("  Variation: {variant}"));
            }
            parts.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Resolve the skill set for `phase` against `user_message`.
pub fn resolve_skills(phase: Phase, user_message: &str, options: &ResolveOptions) -> AgentContext {
    let total_budget = options
        .budget_override
        .unwrap_or_else(|| phase.default_budget());

    // Steps 1 + 2 — phase filter, then intent / flag match.
    let phase_skills: Vec<SkillEntry> = get_skills_by_phase(phase).into_iter().cloned().collect();
    let matched = filter_by_intent(&phase_skills, user_message, &options.flags);

    // Per-phase memory loading (done before injection so history is
    // available for the `{{recentHistory}}` placeholder).
    let mut memory = ResolveMemory::default();
    if phase != Phase::Validation {
        memory.document_context = options.memory.document_context.clone();
    }
    let limit = history_limit(phase);
    if !options.memory.generation_history.is_empty() && limit > 0 {
        memory.generation_history = get_recent_entries(
            &options.memory.generation_history,
            limit,
            options.document_path.as_deref(),
        );
    }

    // Merge caller-provided dynamic content with the generation-phase
    // `recentHistory` block.
    let mut merged = options.dynamic_content.clone();
    if phase == Phase::Generation {
        merged.insert(
            "recentHistory".to_string(),
            format_recent_history(&memory.generation_history),
        );
    }

    // Step 3 — inject dynamic content into every matched skill.
    let injected: Vec<SkillEntry> = matched
        .into_iter()
        .map(|mut s| {
            s.content = inject_dynamic_content(&s.content, &merged);
            s
        })
        .collect();

    // Step 4 — budget trim (after injection, so counts are accurate).
    let trimmed = trim_by_budget(&injected, total_budget);
    let budget_used: u32 = trimmed.iter().map(|s| s.token_count).sum();

    AgentContext {
        role: "general".to_string(),
        phase,
        skills: trimmed,
        memory,
        budget_used,
        budget_max: total_budget,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::generation_history::{create_history_entry, HistoryEntryParams};
    use crate::types::{DesignContext, ResolveOptions};

    fn history(doc: &str, n: usize) -> Vec<crate::types::HistoryEntry> {
        (0..n)
            .map(|i| {
                create_history_entry(
                    HistoryEntryParams {
                        document_path: doc.into(),
                        prompt: format!("prompt {i}"),
                        phase: Phase::Generation,
                        skills_used: vec![],
                        node_count: 5,
                        section_types: vec![],
                        validation_score: None,
                        validation_rounds: None,
                        heading_font: Some(format!("Font{i}")),
                        palette: None,
                        creative_variant: None,
                    },
                    1_000 + i as u64,
                    "2026-05-17T00:00:00Z",
                )
            })
            .collect()
    }

    #[test]
    fn generation_resolve_stays_within_budget() {
        let ctx = resolve_skills(
            Phase::Generation,
            "design a login form",
            &ResolveOptions::default(),
        );
        assert_eq!(ctx.budget_max, 8000);
        assert!(ctx.budget_used <= ctx.budget_max);
        assert!(
            !ctx.skills.is_empty(),
            "generation phase should resolve skills"
        );
    }

    #[test]
    fn generation_resolve_leaves_no_unresolved_placeholder() {
        // With history present, the `{{recentHistory}}` placeholder in
        // the anti-slop skill must be substituted away.
        let opts = ResolveOptions {
            memory: crate::types::ResolveMemory {
                generation_history: history("/a.op", 2),
                ..Default::default()
            },
            ..Default::default()
        };
        let ctx = resolve_skills(Phase::Generation, "design a dashboard", &opts);
        for skill in &ctx.skills {
            assert!(
                !skill.content.contains("{{recentHistory}}"),
                "skill {} still has an unresolved placeholder",
                skill.meta.name
            );
        }
        // History was loaded + capped at the generation limit (3).
        assert_eq!(ctx.memory.generation_history.len(), 2);
    }

    #[test]
    fn validation_phase_carries_no_memory() {
        let opts = ResolveOptions {
            memory: crate::types::ResolveMemory {
                document_context: Some(DesignContext::default()),
                generation_history: history("/a.op", 3),
            },
            ..Default::default()
        };
        let ctx = resolve_skills(Phase::Validation, "check this", &opts);
        // Validation is stateless — no context, no history.
        assert!(ctx.memory.document_context.is_none());
        assert!(ctx.memory.generation_history.is_empty());
    }

    #[test]
    fn planning_phase_keeps_document_context() {
        let opts = ResolveOptions {
            memory: crate::types::ResolveMemory {
                document_context: Some(DesignContext::default()),
                ..Default::default()
            },
            ..Default::default()
        };
        let ctx = resolve_skills(Phase::Planning, "plan a landing page", &opts);
        assert!(ctx.memory.document_context.is_some());
        assert_eq!(ctx.budget_max, 4000);
    }

    #[test]
    fn budget_override_is_honored() {
        let high = resolve_skills(
            Phase::Generation,
            "design something",
            &ResolveOptions::default(),
        );
        let opts = ResolveOptions {
            budget_override: Some(500),
            ..Default::default()
        };
        let low = resolve_skills(Phase::Generation, "design something", &opts);
        assert_eq!(low.budget_max, 500);
        // A tighter budget trims at least as much as the default
        // (base-category skills are always kept, so the floor isn't
        // strictly 500 — but the override must not grow the set).
        assert!(low.budget_used <= high.budget_used);
        assert!(low.skills.len() <= high.skills.len());
    }
}
