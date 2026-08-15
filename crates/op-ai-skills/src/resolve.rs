//! The resolution pipeline — port of `engine/resolve-skills.ts`.
//!
//! `resolve_skills` is the crate's main entry point: phase filter →
//! intent match → per-phase memory injection → budget trim.

use crate::budget::trim_by_budget_pinned;
use crate::loader::{get_skills_by_phase, SkillEntry};
use crate::memory::generation_history::get_recent_entries;
use crate::resolver::{filter_by_intent, filter_by_model_family, inject_dynamic_content};
use crate::types::{
    AgentContext, DropReason, DroppedSkill, Phase, ResolveMemory, ResolveOptions, SkillLoadEntry,
    SkillLoadReport,
};

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

    // Steps 1 + 2 — phase filter, then intent / flag match. Step 1.5 (DS
    // P2-a overlays) sits between them: the `model_families` gate removes
    // family-gated skills from the candidate set unless `options.model_id`
    // admits them. Empty gates pass unchanged (the historical behaviour) and
    // an empty model id admits nothing gated, so callers without a known
    // model resolve byte-for-byte as before.
    let phase_skills: Vec<SkillEntry> = get_skills_by_phase(phase).into_iter().cloned().collect();
    let (phase_skills, gated_out) = filter_by_model_family(&phase_skills, &options.model_id);
    let matched = filter_by_intent(&phase_skills, user_message, &options.flags);

    // Diagnostics — skills gated out by the family filter, then phase skills
    // that failed the intent/flag match.
    let mut dropped: Vec<DroppedSkill> = gated_out
        .iter()
        .map(|g| DroppedSkill {
            name: g.meta.name.clone(),
            reason: DropReason::ModelFamilyMiss,
        })
        .collect();
    dropped.extend(
        phase_skills
            .iter()
            .filter(|p| !matched.iter().any(|m| m.meta.name == p.meta.name))
            .map(|p| DroppedSkill {
                name: p.meta.name.clone(),
                reason: DropReason::IntentMiss,
            }),
    );

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
    // Caller-pinned skills are force-kept (budget-exempt) when they survived the
    // phase/intent/flag match above; an empty pin set (the default) is identical
    // to the plain trim.
    let pinned: Vec<&str> = options.pinned_skills.iter().map(String::as_str).collect();
    let trimmed = trim_by_budget_pinned(&injected, total_budget, user_message, &pinned);
    let budget_used: u32 = trimmed.iter().map(|s| s.token_count).sum();

    // Diagnostics — matched skills the trimmer could not fit.
    for inj in &injected {
        if !trimmed.iter().any(|t| t.meta.name == inj.meta.name) {
            dropped.push(DroppedSkill {
                name: inj.meta.name.clone(),
                reason: DropReason::BudgetExhausted,
            });
        }
    }
    let included: Vec<SkillLoadEntry> = trimmed
        .iter()
        .map(|s| SkillLoadEntry {
            name: s.meta.name.clone(),
            category: s.meta.category,
            token_count: s.token_count,
            truncated: s.truncated,
        })
        .collect();
    let report = SkillLoadReport {
        included,
        dropped,
        budget_used,
        budget_max: total_budget,
    };

    AgentContext {
        role: "general".to_string(),
        phase,
        skills: trimmed,
        memory,
        budget_used,
        budget_max: total_budget,
        report,
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
        assert_eq!(ctx.budget_max, 15700);
        assert!(ctx.budget_used <= ctx.budget_max);
        assert!(
            !ctx.skills.is_empty(),
            "generation phase should resolve skills"
        );
    }

    /// Regression guard for the 2026-07-24 budget audit: real-world
    /// generation prompts (mobile app / SaaS landing page / admin
    /// dashboard) used to silently truncate at least one Domain or
    /// Knowledge skill via the Step 3 total-budget knapsack — even when
    /// every skill fit its own per-skill budget — because the always-on
    /// Base skills alone (~6000 tokens) left only ~2000 tokens of
    /// headroom under the old 8000-token phase default (landing-page.md
    /// was cut to 275 of its 1203 tokens; design-principles was dropped
    /// from all three prompts entirely). The phase default is now 13200
    /// (see `Phase::default_budget`'s doc comment); this locks in that no
    /// resolved skill in these representative prompts gets truncated by
    /// either mechanism (own-budget cap or total-budget tail cut).
    #[test]
    fn representative_generation_prompts_hit_zero_truncation() {
        for prompt in [
            "design a polished mobile app home screen for a fitness tracking app with bottom navigation",
            "design a modern landing page for a SaaS product with hero, pricing, and testimonials",
            "design an admin dashboard with a data table, charts, and analytics for a SaaS product",
        ] {
            let ctx = resolve_skills(Phase::Generation, prompt, &ResolveOptions::default());
            let truncated: Vec<&str> = ctx
                .skills
                .iter()
                .filter(|s| s.truncated)
                .map(|s| s.meta.name.as_str())
                .collect();
            assert!(
                truncated.is_empty(),
                "prompt {prompt:?} truncated skills: {truncated:?}"
            );
            assert!(ctx.budget_used <= ctx.budget_max);
        }
    }

    /// The budget report and the trimmer's internal accounting are the same
    /// number, and a `BudgetExhausted` drop always means the skill genuinely
    /// did not fit — never that headroom was left on the table.
    ///
    /// This was asked as "the report says 652 tokens free yet a 438-token
    /// Knowledge skill was dropped, so `used` and `budget_used` must have
    /// drifted apart" (2026-08-09 deck audit). They have not: `budget_used`
    /// is summed from the very `token_count`s the knapsack accumulated, and
    /// both trim steps recompute `remaining` from that same running total, so
    /// a dropped skill is always strictly larger than the leftover (Step 3
    /// only skips a candidate when `token_count > remaining`, and `used` only
    /// grows afterwards). Free headroom next to an evicted skill therefore
    /// cannot be produced here — it can only come from a consumer that
    /// removes skills AFTER resolution (the orchestrator's tier filters do
    /// exactly that), which leaves a hole in a budget that was already spent.
    /// This test is the standing answer, so the question does not have to be
    /// re-litigated by hand next time.
    #[test]
    fn budget_report_matches_the_knapsack_and_drops_only_what_cannot_fit() {
        for prompt in [
            "帮我做一个 8 页的融资路演 PPT，深色科技感",
            "做一份季度数据汇报 PPT，要有仪表盘和数据表格",
            "generate a 10-slide deck for our SaaS admin console product with analytics data tables",
            "design an admin dashboard with a data table, charts, and analytics for a SaaS product",
        ] {
            let ctx = resolve_skills(Phase::Generation, prompt, &ResolveOptions::default());

            // The report is the knapsack's own accounting, not a re-estimate.
            let summed: u32 = ctx.skills.iter().map(|s| s.token_count).sum();
            assert_eq!(
                ctx.budget_used, summed,
                "{prompt:?}: report {} != sum of kept skills {summed}",
                ctx.budget_used
            );
            assert_eq!(ctx.report.budget_used, ctx.budget_used);
            assert!(ctx.budget_used <= ctx.budget_max);

            let leftover = ctx.budget_max - ctx.budget_used;
            for drop in ctx
                .report
                .dropped
                .iter()
                .filter(|d| d.reason == DropReason::BudgetExhausted)
            {
                let entry = crate::loader::get_skill_by_name(&drop.name)
                    .unwrap_or_else(|| panic!("dropped skill {:?} must exist", drop.name));
                // Skills carrying a `{{placeholder}}` are sized after
                // injection, which the registry copy cannot reproduce.
                if entry.content.contains("{{") {
                    continue;
                }
                let size = crate::budget::estimate_tokens(&entry.content).min(entry.meta.budget);
                assert!(
                    size > leftover,
                    "{prompt:?}: {} ({size} tokens) was dropped for budget while \
                     {leftover} tokens were still free",
                    drop.name
                );
            }
        }
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
        assert_eq!(ctx.budget_max, 6000);
    }

    #[test]
    fn report_records_intent_miss_and_included() {
        // A login-form prompt loads base skills (Always) and surfaces a
        // report; at least one phase skill whose keyword didn't match is
        // recorded as IntentMiss, and survivors appear as included.
        let ctx = resolve_skills(
            Phase::Generation,
            "design a login form",
            &ResolveOptions::default(),
        );
        assert_eq!(ctx.report.budget_max, ctx.budget_max);
        assert_eq!(ctx.report.budget_used, ctx.budget_used);
        assert!(
            !ctx.report.included.is_empty(),
            "survivors must be recorded"
        );
        // Every included entry maps to a resolved skill.
        for entry in &ctx.report.included {
            assert!(ctx.skills.iter().any(|s| s.meta.name == entry.name));
        }
        // IntentMiss is recorded for phase skills whose triggers didn't fire.
        assert!(
            ctx.report
                .dropped
                .iter()
                .any(|d| d.reason == crate::types::DropReason::IntentMiss),
            "expected at least one IntentMiss drop"
        );
    }

    #[test]
    fn rich_intent_surfaces_more_relevant_skills_than_bare_label() {
        // A bare subtask label has few keyword hits; the full prompt has
        // more, so under an identical tight budget the rich intent should
        // not resolve *fewer* relevant (keyword-triggered) skills.
        //
        // "header" alone matches no domain/knowledge keyword triggers.
        // The rich intent contains "mobile" (mobile-app skill) and
        // "landing" (landing-page, anti-slop, copywriting, role-definitions),
        // so it surfaces additional keyword-gated skills the bare label misses.
        //
        // Budget of 12000 is chosen so that the always-triggered base skills
        // (~9800 tokens) fit, leaving ~2200 tokens for keyword-gated skills.
        // "landing" triggers anti-slop (600 tok) which fits in the remainder;
        // the bare "header" label triggers none of those domain/knowledge skills.
        let tight = ResolveOptions {
            budget_override: Some(12_000),
            ..Default::default()
        };
        let bare = resolve_skills(Phase::Generation, "header", &tight);
        let rich = resolve_skills(
            Phase::Generation,
            "design a polished mobile-app food landing page\nheader\nscreen: home",
            &tight,
        );
        // Same budget ceiling, deterministic.
        assert_eq!(bare.budget_max, 12_000);
        assert_eq!(rich.budget_max, 12_000);
        // The richer intent matches at least as many keyword-triggered
        // skills, so its included set is never smaller.
        assert!(
            rich.report.included.len() >= bare.report.included.len(),
            "rich intent ({}) should surface >= bare ({})",
            rich.report.included.len(),
            bare.report.included.len()
        );
        // The rich prompt must surface at least one skill the bare label
        // misses — this is what makes the test non-vacuous: it fails if
        // resolve_skills ignores the intent and uses only the short label.
        assert!(
            rich.report.included.len() > bare.report.included.len(),
            "rich intent ({}) must surface more skills than bare label ({}) \
             — keyword triggers for 'mobile' and 'landing' should fire",
            rich.report.included.len(),
            bare.report.included.len()
        );
        // Both stay within budget.
        assert!(rich.budget_used <= rich.budget_max);
        assert!(bare.budget_used <= bare.budget_max);
        // Report budget_max mirrors what resolve_skills returns.
        assert_eq!(rich.report.budget_max, rich.budget_max);
        assert_eq!(bare.report.budget_max, bare.budget_max);
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
