//! Skill-load summaries carried by `Progress::SubtaskSkills`, split out of
//! `types.rs` at the 800-line cap (pure code motion; re-exported there).

/// A one-line summary of an included skill, surfaced to the chat UI via
/// `Progress::SubtaskSkills`. Mirrors `op_ai_skills::SkillLoadEntry` minus
/// the `category` field (the UI line doesn't display it).
#[derive(Debug, Clone)]
pub struct SkillBrief {
    pub name: String,
    pub token_count: u32,
    pub truncated: bool,
}

impl SkillBrief {
    /// Build a brief from an `op-ai-skills` report entry (name + token_count +
    /// truncated; category is dropped — the UI line doesn't show it).
    pub fn from_entry(e: &op_ai_skills::SkillLoadEntry) -> SkillBrief {
        SkillBrief {
            name: e.name.clone(),
            token_count: e.token_count,
            truncated: e.truncated,
        }
    }
}

/// Short, user-facing word for a `DropReason` (used in the `▸ dropped:` line).
fn drop_reason_display(reason: &op_ai_skills::DropReason) -> &'static str {
    use op_ai_skills::DropReason::*;
    match reason {
        IntentMiss => "intent",
        BudgetExhausted => "budget",
        TierFiltered => "tier",
        MinimalMode => "minimal",
        ReducedComplexity => "reduced",
        Deduped => "dedup",
        ContentMismatch => "mismatch",
        ModelFamilyMiss => "family",
    }
}

/// Decompose a merged `SkillLoadReport` into the four payload parts of
/// `Progress::SubtaskSkills` (included briefs, `(name, reason)` drops,
/// budget_used, budget_max).
pub fn report_to_progress_parts(
    report: &op_ai_skills::SkillLoadReport,
) -> (Vec<SkillBrief>, Vec<(String, String)>, u32, u32) {
    let included = report.included.iter().map(SkillBrief::from_entry).collect();
    let dropped = report
        .dropped
        .iter()
        .map(|d| (d.name.clone(), drop_reason_display(&d.reason).to_string()))
        .collect();
    (included, dropped, report.budget_used, report.budget_max)
}
