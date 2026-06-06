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

/// Filter a resolved skill list according to retry + content parameters.
///
/// Port of the `minimalSkills` branch in `executeSubAgent`
/// (orchestrator-sub-agent.ts:428-431) followed by `compactSubAgentSkills`
/// (orchestrator-sub-agent-compact.ts:4-76), which TS calls unconditionally
/// on every sub-agent prompt.
///
/// # Arguments
/// * `skills`                 — The full resolved skill list from `resolve_skills`.
/// * `tier`                   — The model's capability tier.
/// * `is_mobile_screen`       — Whether the plan is a full mobile screen.
/// * `has_explicit_style_guide` — A style guide or design.md is in effect.
/// * `minimal_skills`         — When `true`, keep only `schema` + `jsonl-format`.
/// * `reduced_complexity`     — When `true` AND `tier == Basic`, narrow to the
///   `retryAllowed` 8-skill set (excludes `elements`).
pub fn apply_skill_filter<T: SkillNamed>(
    skills: Vec<T>,
    tier: ModelTier,
    is_mobile_screen: bool,
    has_explicit_style_guide: bool,
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

    compact_subagent_skills(
        skills,
        tier,
        is_mobile_screen,
        has_explicit_style_guide,
        reduced_complexity,
    )
}

/// Port of `compactSubAgentSkills` (orchestrator-sub-agent-compact.ts:4-76):
/// a content-aware base filter for ALL tiers, then a `jsonl-format` dedup,
/// then a Basic-tier allow-set (further narrowed on reduced-complexity).
fn compact_subagent_skills<T: SkillNamed>(
    skills: Vec<T>,
    tier: ModelTier,
    is_mobile_screen: bool,
    has_explicit_style_guide: bool,
    reduced_complexity: bool,
) -> Vec<T> {
    // Base filter (all tiers): mobile/desktop + explicit-style-guide drops.
    let mut next: Vec<T> = skills
        .into_iter()
        .filter(|s| {
            let name = s.skill_name();
            if is_mobile_screen
                && (name == "landing-page" || name == "copywriting" || name == "anti-slop")
            {
                return false;
            }
            if !is_mobile_screen && name == "mobile-app" {
                return false;
            }
            if has_explicit_style_guide && name == "design-system" {
                return false;
            }
            true
        })
        .collect();

    // When the simplified JSONL format is present, drop the verbose one so a
    // Basic-tier model doesn't carry both (orchestrator-sub-agent-compact.ts:24-28).
    let has_simplified = next
        .iter()
        .any(|s| s.skill_name() == "jsonl-format-simplified");
    if has_simplified {
        next.retain(|s| s.skill_name() != "jsonl-format");
    }

    if tier == ModelTier::Basic {
        // Basic-tier allow-set (orchestrator-sub-agent-compact.ts:31-52).
        // `elements` is included; it's already gated off at the resolve layer
        // when the element-tools flag is false, so keeping it here is a no-op
        // in that case and required when the flag is on.
        const ALLOWED: &[&str] = &[
            "schema",
            "jsonl-format-simplified",
            "jsonl-format",
            "layout",
            "overflow",
            "text-rules",
            "variables",
            "design-md",
            "mobile-app",
            "icon-catalog",
            "style-defaults",
            "elements",
        ];
        next.retain(|s| ALLOWED.contains(&s.skill_name()));

        if reduced_complexity {
            // Reduced-complexity retry kernel (orchestrator-sub-agent-compact.ts:54-71).
            // `elements` deliberately OMITTED — the retry wants the smallest prompt.
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
            next.retain(|s| RETRY_ALLOWED.contains(&s.skill_name()));
        }
    }

    next
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

    // Convenience: the common non-mobile, no-style-guide call shape.
    fn filter(
        skills: Vec<FakeSkill>,
        tier: ModelTier,
        minimal: bool,
        reduced: bool,
    ) -> Vec<FakeSkill> {
        apply_skill_filter(skills, tier, false, false, minimal, reduced)
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
        let out = filter(input, ModelTier::Full, true, false);
        assert_eq!(names(&out), vec!["schema", "jsonl-format"]);
    }

    #[test]
    fn minimal_skills_drops_jsonl_format_simplified() {
        // The TS minimal_skills filter (orchestrator-sub-agent.ts:428-431)
        // keeps exactly `schema` + `jsonl-format` — `jsonl-format-simplified`
        // is NOT in the allow-set, so it is dropped.
        let input = skills(&["schema", "jsonl-format-simplified", "layout", "elements"]);
        let out = filter(input, ModelTier::Basic, true, false);
        assert_eq!(names(&out), vec!["schema"]);
    }

    #[test]
    fn minimal_skills_true_overrides_reduced_complexity() {
        // minimal_skills takes precedence (early return, reduced ignored).
        let input = skills(&["schema", "jsonl-format", "layout", "mobile-app"]);
        let out = filter(input, ModelTier::Basic, true, true);
        assert_eq!(names(&out), vec!["schema", "jsonl-format"]);
    }

    // -----------------------------------------------------------------------
    // base filter (all tiers) + jsonl dedup
    // -----------------------------------------------------------------------

    #[test]
    fn base_filter_drops_mobile_app_on_non_mobile() {
        let input = skills(&["schema", "layout", "mobile-app"]);
        // Full tier, non-mobile → mobile-app dropped, rest kept.
        let out = apply_skill_filter(input, ModelTier::Full, false, false, false, false);
        assert_eq!(names(&out), vec!["schema", "layout"]);
    }

    #[test]
    fn base_filter_drops_landing_copy_antislop_on_mobile() {
        let input = skills(&[
            "schema",
            "landing-page",
            "copywriting",
            "anti-slop",
            "mobile-app",
        ]);
        // Full tier, mobile → landing-page/copywriting/anti-slop dropped,
        // mobile-app kept (it's a mobile screen).
        let out = apply_skill_filter(input, ModelTier::Full, true, false, false, false);
        assert_eq!(names(&out), vec!["schema", "mobile-app"]);
    }

    #[test]
    fn base_filter_drops_design_system_when_explicit_style_guide() {
        let input = skills(&["schema", "design-system", "layout"]);
        // has_explicit_style_guide = true → design-system dropped.
        let out = apply_skill_filter(input, ModelTier::Full, false, true, false, false);
        assert_eq!(names(&out), vec!["schema", "layout"]);
    }

    #[test]
    fn jsonl_dedup_drops_verbose_when_simplified_present() {
        let input = skills(&[
            "schema",
            "jsonl-format",
            "jsonl-format-simplified",
            "layout",
        ]);
        let out = filter(input, ModelTier::Full, false, false);
        assert!(
            !names(&out).contains(&"jsonl-format"),
            "verbose jsonl-format dropped when simplified present"
        );
        assert!(names(&out).contains(&"jsonl-format-simplified"));
    }

    // -----------------------------------------------------------------------
    // Basic-tier allow-set (non-reduced)
    // -----------------------------------------------------------------------

    #[test]
    fn basic_tier_allow_set_drops_non_allowed_skills() {
        // `component-composition` / `examples` are not in the Basic allow-set.
        let input = skills(&[
            "schema",
            "layout",
            "component-composition",
            "examples",
            "jsonl-format-simplified",
        ]);
        let out = filter(input, ModelTier::Basic, false, false);
        let got = names(&out);
        assert!(!got.contains(&"component-composition"));
        assert!(!got.contains(&"examples"));
        assert!(got.contains(&"schema"));
        assert!(got.contains(&"layout"));
        assert!(got.contains(&"jsonl-format-simplified"));
    }

    #[test]
    fn standard_tier_keeps_non_allowed_skills() {
        // The allow-set is Basic-only; Standard/Full keep everything the base
        // filter left.
        let input = skills(&["schema", "component-composition", "examples"]);
        let out = filter(input.clone(), ModelTier::Standard, false, false);
        assert_eq!(out, input, "Standard tier: no Basic allow-set narrowing");
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
        // is_mobile = true so `mobile-app` survives the base filter.
        let out = apply_skill_filter(input, ModelTier::Basic, true, false, false, true);
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
        let out = filter(input, ModelTier::Basic, false, true);
        let got = names(&out);
        assert!(!got.contains(&"elements"));
        assert!(got.contains(&"schema"));
        assert!(got.contains(&"jsonl-format-simplified"));
        assert!(got.contains(&"layout"));
    }

    // -----------------------------------------------------------------------
    // reduced_complexity + Standard/Full -> no Basic narrowing
    // -----------------------------------------------------------------------

    #[test]
    fn reduced_complexity_standard_is_noop() {
        let input = skills(&["schema", "layout", "elements", "overflow", "anti-slop"]);
        let out = filter(input.clone(), ModelTier::Standard, false, true);
        assert_eq!(
            out, input,
            "Standard tier: reduced_complexity must be no-op"
        );
    }

    #[test]
    fn reduced_complexity_full_is_noop() {
        let input = skills(&["schema", "layout", "elements", "overflow"]);
        let out = filter(input.clone(), ModelTier::Full, false, true);
        assert_eq!(out, input, "Full tier: reduced_complexity must be no-op");
    }

    // -----------------------------------------------------------------------
    // no filtering -> passthrough (Basic allow-set keeps all-allowed input)
    // -----------------------------------------------------------------------

    #[test]
    fn no_filtering_returns_all_allowed_skills() {
        // All four are in the Basic allow-set, so the Basic intersection is a
        // no-op and the list passes through unchanged.
        let input = skills(&["schema", "layout", "text-rules", "elements"]);
        let out = filter(input.clone(), ModelTier::Basic, false, false);
        assert_eq!(out, input);
    }
}
