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
    pub fn from_str(s: &str) -> Option<Phase> {
        match s.trim() {
            "planning" => Some(Phase::Planning),
            "generation" => Some(Phase::Generation),
            "validation" => Some(Phase::Validation),
            "maintenance" => Some(Phase::Maintenance),
            _ => None,
        }
    }

    /// Default token budget for this phase (TS `DEFAULT_BUDGETS`).
    pub fn default_budget(self) -> u32 {
        match self {
            Phase::Planning => 4000,
            Phase::Generation => 8000,
            Phase::Validation => 3000,
            Phase::Maintenance => 5000,
        }
    }
}

/// Per-phase default token budgets — the TS `DEFAULT_BUDGETS` record.
pub const DEFAULT_BUDGETS: [(Phase, u32); 4] = [
    (Phase::Planning, 4000),
    (Phase::Generation, 8000),
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
        assert_eq!(Phase::Planning.default_budget(), 4000);
        assert_eq!(Phase::Generation.default_budget(), 8000);
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
}
