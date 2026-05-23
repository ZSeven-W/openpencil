//! Sub-agent skill-set narrowing for reduced-complexity retries.
//!
//! Port of `apps/web/src/services/ai/orchestrator-sub-agent-compact.ts`
//! (`compactSubAgentSkills` + the `retryAllowed` set).
//!
//! Two filtering modes:
//! - **`minimal_skills`** — keep only `schema` + `jsonl-format` (the
//!   bare-minimum kernel for models whose safety scanner times out on
//!   a full-size system prompt).
//! - **`reduced_complexity` + `ModelTier::Basic`** — keep the 8-skill
//!   `retryAllowed` set (drops `elements` and all non-essential skills
//!   so the retry has the smallest viable prompt).
//!
//! For `Standard` / `Full` tier `reduced_complexity` is a no-op (full
//! set returned unchanged), matching the TS comment "Basic tier only".

use crate::model_profile::ModelTier;

/// Filter a resolved skill list according to retry parameters.
///
/// Mirrors the `minimalSkills` / `reducedComplexity` branches in
/// `executeSubAgent` (orchestrator-sub-agent.ts:428-453) and the
/// `retryAllowed` set in `compactSubAgentSkills`
/// (orchestrator-sub-agent-compact.ts:53-72).
///
/// # Arguments
/// * `skills`            — The full resolved skill list from `resolve_skills`.
/// * `tier`              — The model's capability tier.
/// * `minimal_skills`    — When `true`, keep only `schema` + `jsonl-format`.
/// * `reduced_complexity`— When `true` AND `tier == Basic`, keep the
///   `retryAllowed` 8-skill set (excludes `elements`).
pub fn apply_skill_filter<T: SkillNamed>(
    skills: Vec<T>,
    tier: ModelTier,
    minimal_skills: bool,
    reduced_complexity: bool,
) -> Vec<T> {
    if minimal_skills {
        // Last-ditch fallback: only the schema + jsonl-format kernel.
        // Verbatim port of orchestrator-sub-agent.ts:428-431 — exactly
        // two skill names, `jsonl-format-simplified` is NOT kept.
        return skills
            .into_iter()
            .filter(|s| {
                let n = s.skill_name();
                n == "schema" || n == "jsonl-format"
            })
            .collect();
    }

    if reduced_complexity && tier == ModelTier::Basic {
        // Reduced-complexity retry for Basic tier: the retryAllowed set.
        // Verbatim port from orchestrator-sub-agent-compact.ts:54-71.
        // `elements` is deliberately OMITTED — see comment in TS source.
        const RETRY_ALLOWED: &[&str] = &[
            "schema",
            "jsonl-format-simplified",
            "layout",
            "text-rules",
            "mobile-app",
            "style-defaults",
            "design-md",
            "variables",
        ];
        return skills
            .into_iter()
            .filter(|s| RETRY_ALLOWED.contains(&s.skill_name()))
            .collect();
    }

    // Standard / Full tier with reduced_complexity, or no filtering requested:
    // return unchanged.
    skills
}

/// Minimal trait so the filter works over any struct that exposes a skill
/// name — keeps the filter generic without depending on `ResolvedSkill`
/// directly (avoids a circular / fat import from `op_ai_skills`).
pub trait SkillNamed {
    fn skill_name(&self) -> &str;
}

impl SkillNamed for op_ai_skills::ResolvedSkill {
    fn skill_name(&self) -> &str {
        &self.meta.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal stand-in for a resolved skill in tests.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeSkill(String);

    impl SkillNamed for FakeSkill {
        fn skill_name(&self) -> &str {
            &self.0
        }
    }

    fn skills(names: &[&str]) -> Vec<FakeSkill> {
        names.iter().map(|n| FakeSkill(n.to_string())).collect()
    }

    fn names(skills: &[FakeSkill]) -> Vec<&str> {
        skills.iter().map(|s| s.skill_name()).collect()
    }

    // -----------------------------------------------------------------------
    // minimal_skills = true
    // -----------------------------------------------------------------------

    #[test]
    fn minimal_skills_keeps_only_schema_and_jsonl_format() {
        let input = skills(&[
            "schema",
            "jsonl-format",
            "layout",
            "text-rules",
            "variables",
            "design-md",
        ]);
        let out = apply_skill_filter(input, ModelTier::Full, true, false);
        assert_eq!(names(&out), vec!["schema", "jsonl-format"]);
    }

    #[test]
    fn minimal_skills_drops_jsonl_format_simplified() {
        // The TS minimal_skills filter (orchestrator-sub-agent.ts:428-431)
        // keeps exactly `schema` + `jsonl-format` — `jsonl-format-simplified`
        // is NOT in the allow-set, so it is dropped.
        let input = skills(&["schema", "jsonl-format-simplified", "layout", "elements"]);
        let out = apply_skill_filter(input, ModelTier::Basic, true, false);
        assert_eq!(names(&out), vec!["schema"]);
    }

    #[test]
    fn minimal_skills_true_overrides_reduced_complexity() {
        // minimal_skills takes precedence
        let input = skills(&["schema", "jsonl-format", "layout", "mobile-app"]);
        let out = apply_skill_filter(input, ModelTier::Basic, true, true);
        assert_eq!(names(&out), vec!["schema", "jsonl-format"]);
    }

    // -----------------------------------------------------------------------
    // reduced_complexity + Basic tier -> retryAllowed 8-set
    // -----------------------------------------------------------------------

    #[test]
    fn reduced_complexity_basic_keeps_retry_allowed_set() {
        // Full skill corpus that includes everything; retryAllowed drops elements + extras.
        let input = skills(&[
            "schema",
            "jsonl-format-simplified",
            "layout",
            "text-rules",
            "mobile-app",
            "style-defaults",
            "design-md",
            "variables",
            "elements",     // MUST be dropped
            "overflow",     // MUST be dropped (not in retryAllowed)
            "icon-catalog", // MUST be dropped
        ]);
        let out = apply_skill_filter(input, ModelTier::Basic, false, true);
        let got = names(&out);
        // elements must NOT be present
        assert!(
            !got.contains(&"elements"),
            "elements should be dropped from retryAllowed set"
        );
        // overflow must NOT be present
        assert!(
            !got.contains(&"overflow"),
            "overflow should be dropped from retryAllowed set"
        );
        // All 8 retryAllowed skills that were in input should be present
        for expected in &[
            "schema",
            "jsonl-format-simplified",
            "layout",
            "text-rules",
            "mobile-app",
            "style-defaults",
            "design-md",
            "variables",
        ] {
            assert!(
                got.contains(expected),
                "'{expected}' should be in retryAllowed output"
            );
        }
        assert_eq!(got.len(), 8, "exactly 8 skills in retryAllowed output");
    }

    #[test]
    fn reduced_complexity_basic_drops_elements() {
        let input = skills(&["schema", "jsonl-format-simplified", "elements", "layout"]);
        let out = apply_skill_filter(input, ModelTier::Basic, false, true);
        let got = names(&out);
        assert!(!got.contains(&"elements"));
        assert!(got.contains(&"schema"));
        assert!(got.contains(&"jsonl-format-simplified"));
        assert!(got.contains(&"layout"));
    }

    // -----------------------------------------------------------------------
    // reduced_complexity + Standard/Full -> no-op
    // -----------------------------------------------------------------------

    #[test]
    fn reduced_complexity_standard_is_noop() {
        let input = skills(&["schema", "layout", "elements", "overflow", "anti-slop"]);
        let out = apply_skill_filter(input.clone(), ModelTier::Standard, false, true);
        assert_eq!(
            out, input,
            "Standard tier: reduced_complexity must be no-op"
        );
    }

    #[test]
    fn reduced_complexity_full_is_noop() {
        let input = skills(&["schema", "layout", "elements", "overflow"]);
        let out = apply_skill_filter(input.clone(), ModelTier::Full, false, true);
        assert_eq!(out, input, "Full tier: reduced_complexity must be no-op");
    }

    // -----------------------------------------------------------------------
    // no filtering -> passthrough
    // -----------------------------------------------------------------------

    #[test]
    fn no_filtering_returns_all_skills() {
        let input = skills(&["schema", "layout", "text-rules", "elements"]);
        let out = apply_skill_filter(input.clone(), ModelTier::Basic, false, false);
        assert_eq!(out, input);
    }
}
