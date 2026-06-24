//! Sub-agent skill-set narrowing for reduced-complexity retries.
//!
//! Port of `apps/web/src/services/ai/orchestrator-sub-agent-compact.ts`
//! (`compactSubAgentSkills` + the `retryAllowed` set).
//!
//! Two filtering modes:
//! - **`minimal_skills`** — keep only `schema` + `jsonl-format` (the
//!   bare-minimum kernel for models whose safety scanner times out on
//!   a full-size system prompt).
//! - **`reduced_complexity` + `ModelTier::Basic`** — keep the retry
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
/// Returns `(kept, dropped)` where `dropped` is a list of `(name, reason)`
/// pairs classifying why each skill was removed.
///
/// # Arguments
/// * `skills`                 — The full resolved skill list from `resolve_skills`.
/// * `tier`                   — The model's capability tier.
/// * `is_mobile_screen`       — Whether the plan is a full mobile screen.
/// * `has_design_system_substitute` — Another styling source already covers
///   what `design-system` provides, so it can be dropped. On the Rust path this
///   is `has_design_md` (the design-md skill) OR `noStyleGuideMatch` (the
///   `style-defaults` skill loads). Pass `false` ONLY in the gap case — a style
///   guide is NAMED but there is no design.md and no style-defaults — because
///   Rust has not ported the TS `buildSubAgentStyleGuideInstruction` block, so
///   `design-system` is then the only styling guidance left. Passing `true`
///   there would suppress it with nothing to replace it; passing `false` in the
///   `noStyleGuideMatch` case would inject `design-system`'s conflicting
///   "output ONLY a JSON token object" header alongside `style-defaults`
///   (Codex review 2026-06-06).
/// * `minimal_skills`         — When `true`, keep only `schema` + `jsonl-format`.
/// * `reduced_complexity`     — When `true` AND `tier == Basic`, narrow to the
///   `retryAllowed` set (excludes `elements`).
pub fn apply_skill_filter<T: SkillNamed>(
    skills: Vec<T>,
    tier: ModelTier,
    is_mobile_screen: bool,
    has_design_system_substitute: bool,
    minimal_skills: bool,
    reduced_complexity: bool,
) -> (Vec<T>, Vec<(String, op_ai_skills::DropReason)>) {
    use op_ai_skills::DropReason;

    // Snapshot input names before any filtering.
    let before: Vec<String> = skills.iter().map(|s| s.skill_name().to_string()).collect();

    let next = if minimal_skills {
        // Last-ditch fallback: only the schema + jsonl-format kernel.
        // Verbatim port of orchestrator-sub-agent.ts:428-431 — exactly
        // two skill names, `jsonl-format-simplified` is NOT kept.
        skills
            .into_iter()
            .filter(|s| {
                let n = s.skill_name();
                n == "schema" || n == "jsonl-format"
            })
            .collect()
    } else {
        compact_subagent_skills(
            skills,
            tier,
            is_mobile_screen,
            has_design_system_substitute,
            reduced_complexity,
        )
    };

    // Classify drop reason from the active mode flags.
    let base_reason = if minimal_skills {
        DropReason::MinimalMode
    } else if reduced_complexity {
        DropReason::ReducedComplexity
    } else {
        DropReason::TierFiltered
    };

    let kept: std::collections::HashSet<&str> = next.iter().map(|s| s.skill_name()).collect();
    let jsonl_simplified_kept = kept.contains("jsonl-format-simplified");
    let dropped: Vec<(String, DropReason)> = before
        .into_iter()
        .filter(|n| !kept.contains(n.as_str()))
        .map(|n| {
            // verbose jsonl-format dropped while simplified survives = dedup, not tier.
            let reason = if n == "jsonl-format" && jsonl_simplified_kept {
                DropReason::Deduped
            } else {
                base_reason
            };
            (n, reason)
        })
        .collect();

    (next, dropped)
}

/// Port of `compactSubAgentSkills` (orchestrator-sub-agent-compact.ts:4-76):
/// a content-aware base filter for ALL tiers, then a `jsonl-format` dedup,
/// then a Basic-tier allow-set (further narrowed on reduced-complexity).
fn compact_subagent_skills<T: SkillNamed>(
    skills: Vec<T>,
    tier: ModelTier,
    is_mobile_screen: bool,
    has_design_system_substitute: bool,
    reduced_complexity: bool,
) -> Vec<T> {
    // Base filter (all tiers): mobile/desktop drops + a design-system drop
    // gated on whether another styling source already covers it. TS drops
    // `design-system` whenever an explicit style guide OR design.md is in
    // effect (it then injects `buildSubAgentStyleGuideInstruction` or the
    // design-md skill as the replacement). Rust has ported the design-md skill
    // AND the `style-defaults` skill (noStyleGuideMatch), but NOT the
    // style-guide block — so the caller passes `has_design_system_substitute =
    // has_design_md || no_style_guide_match`. That drops design-system when
    // design-md or style-defaults covers styling (avoiding a redundant,
    // conflicting "output ONLY JSON tokens" header) and keeps it only in the
    // gap case: a style guide is named but neither covers it (Codex 2026-06-06).
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
            if has_design_system_substitute && name == "design-system" {
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
            // Manifest output protocol — gated on `hasManifest` at the
            // resolve layer; a no-op when the flag is off, required when
            // on (same pattern as `elements` below).
            "element-manifest",
            "layout",
            "overflow",
            "text-rules",
            "variables",
            "design-md",
            // `design-system` is NOT in the TS allow-set (TS injects a
            // style-guide replacement). Rust keeps it in the allow-set so the
            // gap case survives — a Basic-tier model with a NAMED style guide
            // but no design.md and no style-defaults. In every other case the
            // base filter above has already dropped it (design-md OR
            // style-defaults covers styling), so this entry only matters for
            // the gap case. Remove once buildSubAgentStyleGuideInstruction is
            // ported. (Codex review 2026-06-06.)
            "design-system",
            "cjk-typography",
            "mobile-app",
            "mobile-ui",
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
                "mobile-ui",
                "style-defaults",
                "design-md",
                "variables",
                // Kept here for the SAME reason it's in ALLOWED above: Rust has
                // no style-guide instruction block, so a Basic-tier retry with a
                // selected guide (where `style-defaults` does NOT load —
                // noStyleGuideMatch is false) would otherwise have zero styling
                // guidance. The base filter still drops it when design-md (the
                // genuine replacement) is present; and when style-defaults IS
                // present, the lower-priority design-system is trimmed first
                // under the tight retry budget. (Codex review 2026-06-06.)
                "design-system",
                "cjk-typography",
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

impl SkillNamed for op_ai_skills::SkillEntry {
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
        let (kept, _drops) = apply_skill_filter(skills, tier, false, false, minimal, reduced);
        kept
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
        let (out, _drops) = apply_skill_filter(input, ModelTier::Full, false, false, false, false);
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
        let (out, _drops) = apply_skill_filter(input, ModelTier::Full, true, false, false, false);
        assert_eq!(names(&out), vec!["schema", "mobile-app"]);
    }

    #[test]
    fn base_filter_drops_design_system_when_substitute_present() {
        let input = skills(&["schema", "design-system", "layout"]);
        // has_design_system_substitute = true — another styling source covers
        // it (the design-md skill, OR style-defaults on noStyleGuideMatch). The
        // caller computes this as `has_design_md || no_style_guide_match`, so
        // this same drop handles BOTH the design.md case and the
        // no-style-guide case (Codex 2026-06-06: don't keep a conflicting
        // design-system alongside style-defaults).
        let (out, _drops) = apply_skill_filter(input, ModelTier::Full, false, true, false, false);
        assert_eq!(names(&out), vec!["schema", "layout"]);
    }

    #[test]
    fn design_system_kept_only_in_gap_case() {
        // Regression guard (Codex 2026-06-06): in the GAP case — a style-guide
        // NAME is selected but no design.md and no style-defaults — Rust injects
        // no replacement block, so design-system is the only styling guidance
        // left and must NOT be suppressed. The caller passes
        // has_design_system_substitute=false only for that case.
        // Full tier:
        let (out, _drops) = apply_skill_filter(
            skills(&["schema", "design-system", "layout"]),
            ModelTier::Full,
            false,
            false, // no replacement
            false,
            false,
        );
        assert!(
            names(&out).contains(&"design-system"),
            "Full tier must keep design-system when nothing replaces it"
        );
        // Basic tier (allow-set must include design-system as a fallback):
        let (out_basic, _drops) = apply_skill_filter(
            skills(&["schema", "design-system", "layout"]),
            ModelTier::Basic,
            false,
            false,
            false,
            false,
        );
        assert!(
            names(&out_basic).contains(&"design-system"),
            "Basic tier must keep design-system when nothing replaces it"
        );
        // Basic-tier REDUCED retry must also keep it (Codex 2026-06-06 #2):
        // the retry kernel had dropped design-system unconditionally.
        let (out_reduced, _drops) = apply_skill_filter(
            skills(&["schema", "design-system", "layout"]),
            ModelTier::Basic,
            false,
            false, // no replacement
            false,
            true, // reduced_complexity
        );
        assert!(
            names(&out_reduced).contains(&"design-system"),
            "Basic reduced retry must keep design-system when nothing replaces it"
        );
        // …but WITH a replacement (design.md), the reduced retry still drops it.
        let (out_reduced_replaced, _drops) = apply_skill_filter(
            skills(&["schema", "design-system", "layout"]),
            ModelTier::Basic,
            false,
            true, // replacement present
            false,
            true,
        );
        assert!(
            !names(&out_reduced_replaced).contains(&"design-system"),
            "Basic reduced retry drops design-system when design-md replaces it"
        );
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
            "cjk-typography",
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
        assert!(got.contains(&"cjk-typography"));
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
    // reduced_complexity + Basic tier -> retryAllowed set
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
            "cjk-typography",
            "style-defaults",
            "design-md",
            "variables",
            "elements",     // MUST be dropped
            "overflow",     // MUST be dropped (not in retryAllowed)
            "icon-catalog", // MUST be dropped
        ]);
        // is_mobile = true so `mobile-app` survives the base filter.
        let (out, _drops) = apply_skill_filter(input, ModelTier::Basic, true, false, false, true);
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
        // All retryAllowed skills that were in input should be present.
        for expected in &[
            "schema",
            "jsonl-format-simplified",
            "layout",
            "text-rules",
            "mobile-app",
            "cjk-typography",
            "style-defaults",
            "design-md",
            "variables",
        ] {
            assert!(
                got.contains(expected),
                "'{expected}' should be in retryAllowed output"
            );
        }
        assert_eq!(got.len(), 9, "exactly 9 skills in retryAllowed output");
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

    // -----------------------------------------------------------------------
    // apply_skill_filter reports drop reasons
    // -----------------------------------------------------------------------

    #[test]
    fn apply_skill_filter_reports_tier_drops() {
        // `examples` / `copywriting` are not in the Basic ALLOWED set.
        let input = skills(&["schema", "examples", "copywriting"]);
        let (kept, dropped) =
            apply_skill_filter(input, ModelTier::Basic, true, false, false, false);
        assert!(names(&kept).contains(&"schema"));
        let dn: Vec<&str> = dropped.iter().map(|(n, _)| n.as_str()).collect();
        assert!(dn.contains(&"examples") && dn.contains(&"copywriting"));
        assert!(dropped
            .iter()
            .all(|(_, r)| matches!(r, op_ai_skills::DropReason::TierFiltered)));
    }

    #[test]
    fn apply_skill_filter_reports_minimal_mode_drops() {
        let input = skills(&["schema", "jsonl-format", "layout", "text-rules"]);
        let (kept, dropped) = apply_skill_filter(input, ModelTier::Full, false, false, true, false);
        assert_eq!(names(&kept), vec!["schema", "jsonl-format"]);
        // layout + text-rules must be reported as MinimalMode drops.
        let dn: Vec<&str> = dropped.iter().map(|(n, _)| n.as_str()).collect();
        assert!(dn.contains(&"layout") && dn.contains(&"text-rules"));
        assert!(dropped
            .iter()
            .all(|(_, r)| matches!(r, op_ai_skills::DropReason::MinimalMode)));
    }

    #[test]
    fn apply_skill_filter_reports_deduped_jsonl_format() {
        // When jsonl-format-simplified survives, verbose jsonl-format is Deduped.
        let input = skills(&[
            "schema",
            "jsonl-format",
            "jsonl-format-simplified",
            "layout",
        ]);
        let (kept, dropped) =
            apply_skill_filter(input, ModelTier::Full, false, false, false, false);
        assert!(names(&kept).contains(&"jsonl-format-simplified"));
        assert!(!names(&kept).contains(&"jsonl-format"));
        let deduped: Vec<&str> = dropped
            .iter()
            .filter(|(_, r)| matches!(r, op_ai_skills::DropReason::Deduped))
            .map(|(n, _)| n.as_str())
            .collect();
        assert!(deduped.contains(&"jsonl-format"));
    }

    #[test]
    fn apply_skill_filter_reports_reduced_complexity_drops() {
        // elements is in ALLOWED but NOT in RETRY_ALLOWED.
        let input = skills(&["schema", "jsonl-format-simplified", "layout", "elements"]);
        let (kept, dropped) =
            apply_skill_filter(input, ModelTier::Basic, false, false, false, true);
        assert!(!names(&kept).contains(&"elements"));
        let dn: Vec<&str> = dropped.iter().map(|(n, _)| n.as_str()).collect();
        assert!(dn.contains(&"elements"));
        let elements_reason = dropped
            .iter()
            .find(|(n, _)| n == "elements")
            .map(|(_, r)| r);
        assert!(matches!(
            elements_reason,
            Some(op_ai_skills::DropReason::ReducedComplexity)
        ));
    }
}
