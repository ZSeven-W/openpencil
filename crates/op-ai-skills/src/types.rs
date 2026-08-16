//! Core engine types — a faithful port of `pen-ai-skills/src/engine/
//! types.ts`. Plain data; the resolution logic lives in [`crate::
//! resolver`] / [`crate::budget`] / [`crate::resolve`].

use std::collections::HashMap;

/// One of the four generation phases a skill set is resolved for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Planning,
    Generation,
    Validation,
    Maintenance,
}

impl Phase {
    /// Lowercase wire token, matching the TS `Phase` string union.
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Planning => "planning",
            Phase::Generation => "generation",
            Phase::Validation => "validation",
            Phase::Maintenance => "maintenance",
        }
    }

    /// Parse a phase token (as it appears in skill frontmatter).
    // Token parser — `Option`-returning, not the `Result`-shaped
    // `FromStr`, so the trait does not apply.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Phase> {
        match s.trim() {
            "planning" => Some(Phase::Planning),
            "generation" => Some(Phase::Generation),
            "validation" => Some(Phase::Validation),
            "maintenance" => Some(Phase::Maintenance),
            _ => None,
        }
    }

    /// Default token budget for this phase. Planning/Validation/Maintenance
    /// are the original TS `DEFAULT_BUDGETS` values (faithful port).
    /// Generation was raised 8000 → 12000 (2026-07-24): the always-on Base
    /// skills alone already use ~6000 tokens, so 8000 left only ~2000 for
    /// Domain + Knowledge combined — routinely not enough for even one
    /// well-matched Domain skill (`mobile-app` ~1950 tok, `dashboard` ~2250
    /// tok), let alone a second Domain skill plus any Knowledge skill on
    /// top. `op-orchestrator/src/prompt.rs` had already reached the same
    /// number independently for its Full-tier reporting default (comment:
    /// "image-rich data-list sections … overflowed 8000 tokens and
    /// truncated their scripts to zero generated nodes") but never actually
    /// wired it into `budget_override` for that tier, so real trimming
    /// silently ran at 8000 while the diagnostics reported 12000. Raising
    /// the shared default here makes every caller — orchestrator Full tier,
    /// the builtin-agent chat preamble (`chat_runtime.rs`), and the direct
    /// chat system prompt (`chat_system_prompt.rs`) — actually get the
    /// headroom the codebase had already decided was correct. Tier-scaled
    /// callers (Basic/Standard mobile/desktop) are unaffected: they pass an
    /// explicit `budget_override` and never fall through to this default.
    ///
    /// Generation moved again 12000 → 13200 (2026-08-09) when `deck-contract`
    /// joined the deck corpus. A deck prompt now resolves four Domain skills
    /// totalling ~6200 tokens (`cjk-typography` 602, `deck-patterns` 1821,
    /// `deck-contract` 1599, `slides` 2176) on top of ~6700 of always-kept
    /// Base skills. At 12000 that overflowed by ~900 and the Step 3 knapsack
    /// cut `slides` down to 1274 tokens, so the tier tables and type floors
    /// silently vanished from every deck prompt. The three deck skills are
    /// deliberately orthogonal (tier selection, structural skeletons,
    /// cross-tier contract), so the fix is headroom rather than merging them
    /// back together.
    ///
    /// Generation moved again 13200 → 13500 (2026-08-11): nine new style guides
    /// and the projector-board corpus additions grew the deck set, so a deck
    /// prompt now resolves 13293 tokens with `design-principles` (438) included.
    /// At 13200 the Step 3 knapsack dropped `design-principles` while the report
    /// still showed headroom; 13500 keeps ~200 tokens of margin over the
    /// measured 13293.
    ///
    /// Generation moved again 13500 → 15700 (2026-08-13) when the LOGO review
    /// contract landed. A mixed brand-review + deck request legitimately needs
    /// the logo acceptance predicates and all three orthogonal deck contracts
    /// in one assembly; the measured CJK stress set is just under 15500 tokens.
    /// At 13500
    /// the knapsack cut the tail of `slides`, recreating the silent contract-loss
    /// bug this budget is meant to prevent. The new ceiling keeps the combined
    /// production prompt byte-complete with a small drift margin.
    ///
    /// Planning moved 4000 → 6000 for a related reason (2026-07-28). Its
    /// three `Base` skills are budget-EXEMPT but still counted against the
    /// total, and they need ~4500 tokens on their own once
    /// `style-guide-selector` carries the injected style-guide catalog. At
    /// 4000 the phase was already over budget before Step 3 ran, so
    /// `landing-page-predesign` — the phase's only Domain skill — could never
    /// be included on ANY prompt, matched or not. The ceiling now covers the
    /// base set plus that skill with headroom.
    pub fn default_budget(self) -> u32 {
        match self {
            Phase::Planning => 6000,
            Phase::Generation => 15700,
            Phase::Validation => 3000,
            Phase::Maintenance => 5000,
        }
    }
}

/// Per-phase default token budgets — the TS `DEFAULT_BUDGETS` record.
pub const DEFAULT_BUDGETS: [(Phase, u32); 4] = [
    (Phase::Planning, 6000),
    (Phase::Generation, 15700),
    (Phase::Validation, 3000),
    (Phase::Maintenance, 5000),
];

/// A skill's activation condition. The TS `SkillTrigger` is
/// `null | { keywords } | { flags }`; `Always` is the `null` arm —
/// the skill is included unconditionally for its phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillTrigger {
    /// Unconditional — always included for the skill's phase(s).
    Always,
    /// Included when the user message matches any keyword.
    Keywords(Vec<String>),
    /// Included when every named flag is set in `ResolveOptions`.
    Flags(Vec<String>),
}

/// Budget-priority class. `Base` is always kept; `Domain` fills the
/// budget in priority order; `Knowledge` is added only if room is
/// left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillCategory {
    Base,
    Domain,
    Knowledge,
}

impl SkillCategory {
    /// Parse a category token from skill frontmatter; defaults to
    /// `Domain` for an unknown / missing value (TS parity).
    // Infallible token parser, not the `Result`-shaped `FromStr`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> SkillCategory {
        match s.trim() {
            "base" => SkillCategory::Base,
            "knowledge" => SkillCategory::Knowledge,
            _ => SkillCategory::Domain,
        }
    }
}

/// Parsed skill frontmatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub phase: Vec<Phase>,
    pub trigger: SkillTrigger,
    pub priority: i32,
    pub budget: u32,
    pub category: SkillCategory,
    /// Optional model-family gate (DS P2-a overlay mechanism). A skill whose
    /// frontmatter lists `model_families` only enters the candidate set when
    /// the request's model id (normalized lowercase, `provider/` prefix
    /// stripped) contains one of the families as a substring. Empty = the
    /// historical ungated behaviour every existing skill keeps.
    ///
    /// Strategic line: output contracts belong in the public corpus, model
    /// behaviour adaptation belongs in the DS experiment field —
    /// `skills/overlays/` is the test bed; overlay teaching migrates into
    /// the public skills only after ab validation graduates it.
    pub model_families: Vec<String>,
}

/// A skill after resolution — content possibly truncated to fit its
/// budget, with the resulting token count recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSkill {
    pub meta: SkillMeta,
    pub content: String,
    pub token_count: u32,
    pub truncated: bool,
}

/// Options threaded into [`crate::resolve::resolve_skills`].
#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    /// Boolean flags consulted by `SkillTrigger::Flags` triggers.
    pub flags: HashMap<String, bool>,
    /// `{{key}}` placeholder substitutions injected into skill bodies.
    pub dynamic_content: HashMap<String, String>,
    /// Path of the document being designed (scopes history lookups).
    pub document_path: Option<String>,
    /// Override for the phase's default token budget.
    pub budget_override: Option<u32>,
    /// Design memory carried across turns.
    pub memory: ResolveMemory,
    /// Skill names that must be force-included (budget-exempt) when they match
    /// the phase/intent/flags. A pinned skill is kept like a `Base` skill —
    /// never dropped by the budget trim — because its teaching is the point of
    /// the feature that pinned it (e.g. `component-composition` when a reusable
    /// component library is loaded). Empty on every default path, so a
    /// no-library generation is byte-for-byte unchanged.
    pub pinned_skills: Vec<String>,
    /// Model id of the requesting model, normalized at match time (lowercase,
    /// `provider/` prefix stripped). Drives the `model_families` overlay gate
    /// (DS P2-a): a family-gated skill only enters the candidate set when
    /// this id contains one of its families. Empty (the default) never
    /// admits a gated skill, so every path that does not know its model
    /// keeps the historical behaviour.
    pub model_id: String,
}

/// Design memory bundle (`ResolveOptions.memory` in the TS).
#[derive(Debug, Clone, Default)]
pub struct ResolveMemory {
    pub document_context: Option<DesignContext>,
    pub generation_history: Vec<HistoryEntry>,
}

/// Accumulated knowledge about the document being designed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesignContext {
    pub document_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub design_system: DesignSystem,
    pub structure: DesignStructure,
    pub preferences: DesignPreferences,
}

/// Visual-system facts within a [`DesignContext`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesignSystem {
    pub palette: Vec<String>,
    pub typography: Option<String>,
    pub spacing: Option<String>,
    pub aesthetic: Option<String>,
}

/// Page-structure facts within a [`DesignContext`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesignStructure {
    pub page_type: Option<String>,
    pub sections: Vec<String>,
    pub component_patterns: Vec<String>,
}

/// User preference overrides within a [`DesignContext`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesignPreferences {
    pub overrides: Vec<PreferenceOverride>,
}

/// One "change X from Y to Z" preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceOverride {
    pub what: String,
    pub from: String,
    pub to: String,
}

/// User feedback on a recorded generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feedback {
    Accepted,
    Modified,
    Regenerated,
    Deleted,
}

/// One recorded generation run, used for the anti-repetition
/// feedback loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub document_path: String,
    pub input: HistoryInput,
    pub output: HistoryOutput,
    pub feedback: Option<Feedback>,
}

/// Input side of a [`HistoryEntry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryInput {
    pub prompt: String,
    pub phase: Phase,
    pub skills_used: Vec<String>,
}

/// Output side of a [`HistoryEntry`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryOutput {
    pub node_count: u32,
    pub section_types: Vec<String>,
    pub validation_score: Option<u32>,
    pub validation_rounds: Option<u32>,
    pub heading_font: Option<String>,
    pub palette: Option<String>,
    pub creative_variant: Option<String>,
}

/// Why a candidate skill was excluded from the resolved set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// Trigger keywords did not match the intent (`filter_by_intent`).
    IntentMiss,
    /// No budget remained to include the skill (`trim_by_budget`).
    BudgetExhausted,
    /// Removed by the model-tier allow-set (`apply_skill_filter`).
    TierFiltered,
    /// Removed because the minimal-mode floor was applied.
    MinimalMode,
    /// Removed because complexity was reduced for this request.
    ReducedComplexity,
    /// Removed as a duplicate of an already-included skill.
    Deduped,
    /// Removed because its content mismatched the request.
    ContentMismatch,
    /// Excluded because the skill's `model_families` gate did not admit
    /// the request's model id (overlay skills only — DS P2-a).
    ModelFamilyMiss,
}

/// One skill that was excluded, with the reason it was dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedSkill {
    pub name: String,
    pub reason: DropReason,
}

/// One skill that survived into the resolved set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillLoadEntry {
    pub name: String,
    pub category: SkillCategory,
    pub token_count: u32,
    pub truncated: bool,
}

/// Diagnostic record of what loaded vs dropped for one resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillLoadReport {
    pub included: Vec<SkillLoadEntry>,
    pub dropped: Vec<DroppedSkill>,
    pub budget_used: u32,
    pub budget_max: u32,
}

/// The fully-resolved agent context [`crate::resolve::resolve_skills`]
/// returns — the skill set plus the budget accounting + memory.
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub role: String,
    pub phase: Phase,
    pub skills: Vec<ResolvedSkill>,
    pub memory: ResolveMemory,
    pub budget_used: u32,
    pub budget_max: u32,
    pub report: SkillLoadReport,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_round_trips() {
        for p in [
            Phase::Planning,
            Phase::Generation,
            Phase::Validation,
            Phase::Maintenance,
        ] {
            assert_eq!(Phase::from_str(p.as_str()), Some(p));
        }
        assert_eq!(Phase::from_str("nonsense"), None);
    }

    #[test]
    fn default_budget_table() {
        assert_eq!(Phase::Planning.default_budget(), 6000);
        assert_eq!(Phase::Generation.default_budget(), 15700);
        assert_eq!(Phase::Validation.default_budget(), 3000);
        assert_eq!(Phase::Maintenance.default_budget(), 5000);
        // The const table agrees with the per-variant method.
        for (phase, budget) in DEFAULT_BUDGETS {
            assert_eq!(phase.default_budget(), budget);
        }
    }

    #[test]
    fn category_defaults_to_domain() {
        assert_eq!(SkillCategory::from_str("base"), SkillCategory::Base);
        assert_eq!(
            SkillCategory::from_str("knowledge"),
            SkillCategory::Knowledge
        );
        assert_eq!(SkillCategory::from_str("domain"), SkillCategory::Domain);
        assert_eq!(SkillCategory::from_str("???"), SkillCategory::Domain);
    }

    #[test]
    fn skill_load_report_defaults_empty() {
        let r = SkillLoadReport::default();
        assert!(r.included.is_empty());
        assert!(r.dropped.is_empty());
        assert_eq!(r.budget_used, 0);
        assert_eq!(r.budget_max, 0);
        let d = DroppedSkill {
            name: "examples".into(),
            reason: DropReason::BudgetExhausted,
        };
        assert_eq!(d.reason, DropReason::BudgetExhausted);
    }
}
