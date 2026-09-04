//! Timeout wiring, APPEND-MODE injection, self-check / geometry retry
//! feedback, `subtask_intent` and the skill-load report.

use super::*;

// ── C4: timeout wiring tests ──────────────────────────────────────────────

/// Rich/Minimal mode sets profile-derived timeouts (not None).
#[test]
fn orchestrator_prompt_rich_has_profile_timeouts() {
    let pp = build_orchestrator_prompt(&req(), PlanningMode::Rich, AbortFlag::new());
    assert!(
        pp.call_request.no_text_timeout.is_some(),
        "no_text_timeout must be Some for Rich mode"
    );
    assert!(
        pp.call_request.first_text_timeout.is_some(),
        "first_text_timeout must be Some for Rich mode"
    );
    // Short prompt (< 2200 chars): hard = 300s for Standard/Full tier.
    assert_eq!(
        pp.call_request.timeout,
        std::time::Duration::from_millis(300_000),
        "short-prompt Rich timeout should be 300s"
    );
}

/// Compact mode uses builtin_planning_timeouts (60s hard for Full tier, not 300s).
#[test]
fn orchestrator_prompt_compact_uses_builtin_timeouts() {
    let pp = build_orchestrator_prompt(&req(), PlanningMode::Compact, AbortFlag::new());
    // builtin: hard=60_000ms for Full tier (multiplier=1.0)
    assert_eq!(
        pp.call_request.timeout,
        std::time::Duration::from_millis(60_000),
        "Compact mode should use builtin planning timeout (60s hard)"
    );
    assert!(
        pp.call_request.no_text_timeout.is_some(),
        "no_text_timeout must be Some for Compact mode"
    );
    assert!(
        pp.call_request.first_text_timeout.is_some(),
        "first_text_timeout must be Some for Compact mode"
    );
}

/// A long prompt (>= 4200 chars) yields a larger hard timeout than a short one
/// for Rich mode.
#[test]
fn orchestrator_prompt_long_prompt_has_larger_timeout_than_short() {
    let short_req = DesignRequest {
        prompt: "short".into(), // < 2200 chars
        model: Some("claude-sonnet".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let long_prompt = "x".repeat(5000); // >= 4200 chars
    let long_req = DesignRequest {
        prompt: long_prompt,
        model: Some("claude-sonnet".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let short_pp = build_orchestrator_prompt(&short_req, PlanningMode::Rich, AbortFlag::new());
    let long_pp = build_orchestrator_prompt(&long_req, PlanningMode::Rich, AbortFlag::new());
    assert!(
        long_pp.call_request.timeout > short_pp.call_request.timeout,
        "long prompt should yield a larger timeout than short prompt"
    );
}

/// timeout_multiplier is applied: deepseek-v4-pro has multiplier=2.0,
/// so its timeout should be 2× the standard short-bucket orchestrator timeout.
#[test]
fn orchestrator_prompt_multiplier_applied() {
    let ds_req = DesignRequest {
        prompt: "a page".into(), // short bucket
        model: Some("deepseek-v4-pro".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let pp = build_orchestrator_prompt(&ds_req, PlanningMode::Rich, AbortFlag::new());
    // Short bucket base: 300_000ms × 2.0 = 600_000ms
    assert_eq!(
        pp.call_request.timeout,
        std::time::Duration::from_millis(600_000),
        "deepseek-v4-pro multiplier=2.0 should double the short-bucket timeout"
    );
}

/// Sub-agent prompt has profile-derived timeouts (not None).
#[test]
fn subagent_prompt_has_profile_timeouts() {
    let (cr, _) = bsp(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        cr.no_text_timeout.is_some(),
        "no_text_timeout must be Some for sub-agent"
    );
    assert!(
        cr.first_text_timeout.is_some(),
        "first_text_timeout must be Some for sub-agent"
    );
    // req() has short prompt → Short bucket SA hard = 420_000ms (Full tier, multiplier=1)
    assert_eq!(
        cr.timeout,
        std::time::Duration::from_millis(420_000),
        "short-prompt sub-agent hard timeout should be 420s"
    );
}

/// A long prompt (>= 4200 chars) yields a larger sub-agent hard timeout.
#[test]
fn subagent_prompt_long_prompt_has_larger_timeout() {
    let short_req = DesignRequest {
        prompt: "design a page".into(),
        model: Some("claude-sonnet".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let long_req = DesignRequest {
        prompt: "x".repeat(5000),
        model: Some("claude-sonnet".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let (short_cr, _) = bsp(
        &subtask(),
        &plan(),
        &short_req,
        AbortFlag::new(),
        false,
        false,
    );
    let (long_cr, _) = bsp(
        &subtask(),
        &plan(),
        &long_req,
        AbortFlag::new(),
        false,
        false,
    );
    assert!(
        long_cr.timeout > short_cr.timeout,
        "long prompt should yield a larger sub-agent timeout"
    );
}

/// Basic-tier sub-agent clamps no_text and first_text timeouts.
#[test]
fn subagent_prompt_basic_tier_clamps_soft_timeouts() {
    let basic_req = DesignRequest {
        prompt: "a page".into(), // short bucket
        model: Some("claude-haiku".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let (cr, _) = bsp(
        &subtask(),
        &plan(),
        &basic_req,
        AbortFlag::new(),
        false,
        false,
    );
    // Basic clamp: no_text ≤ 45_000ms, first_text ≤ 75_000ms
    assert!(
        cr.no_text_timeout.unwrap() <= std::time::Duration::from_millis(45_000),
        "Basic tier no_text_timeout should be clamped to ≤ 45s"
    );
    assert!(
        cr.first_text_timeout.unwrap() <= std::time::Duration::from_millis(75_000),
        "Basic tier first_text_timeout should be clamped to ≤ 75s"
    );
}

// ── B1: APPEND MODE prompt injection ─────────────────────────────────────

/// When existing_section_labels is Some(non-empty), the user prompt must
/// contain the "APPEND MODE:" block with each label quoted.
#[test]
fn subagent_prompt_append_mode_injected_when_labels_present() {
    let st = crate::plan::Subtask {
        id: "pricing".into(),
        label: "Pricing".into(),
        region: crate::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "pricing".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: Some(vec!["Hero".into(), "Pricing".into()]),
        retry_feedback: None,
    };
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        cr.user_prompt.contains("APPEND MODE"),
        "user_prompt must contain APPEND MODE block"
    );
    assert!(
        cr.user_prompt.contains(r#""Hero""#),
        "user_prompt must contain quoted label \"Hero\""
    );
    assert!(
        cr.user_prompt.contains(r#""Pricing""#),
        "user_prompt must contain quoted label \"Pricing\""
    );
}

/// When existing_section_labels is None, the user prompt must NOT contain
/// the "APPEND MODE:" block.
#[test]
fn subagent_prompt_no_append_mode_when_labels_none() {
    let st = crate::plan::Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: crate::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    };
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        !cr.user_prompt.contains("APPEND MODE"),
        "user_prompt must NOT contain APPEND MODE block when labels is None"
    );
}

/// When existing_section_labels is Some(empty vec), the user prompt must
/// NOT contain the "APPEND MODE:" block.
#[test]
fn subagent_prompt_no_append_mode_when_labels_empty() {
    let st = crate::plan::Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: crate::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: Some(vec![]),
        retry_feedback: None,
    };
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        !cr.user_prompt.contains("APPEND MODE"),
        "user_prompt must NOT contain APPEND MODE block when labels is empty"
    );
}

// ── retry_feedback: self-check rejection echoed into the retry prompt ─────

/// When the retry ladder sets `retry_feedback` (attempt 2 after a self-check
/// quality rejection — see `concurrent::run_subtask_retry_ladder`), the
/// user prompt must carry the rejection reason and tell the model to keep
/// its full skill set, not simplify.
#[test]
fn subagent_prompt_injects_self_check_feedback_when_present() {
    let st = crate::plan::Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: crate::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: Some(crate::plan::RetryFeedback::SelfCheck(
            "self-check failed: radial-stack-not-concentric at n14: progress-ring track, \
             progress arc, and measurable centre content must share one point"
                .into(),
        )),
    };
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        cr.user_prompt.contains("SELF-CHECK FIX REQUIRED"),
        "user_prompt must contain the self-check feedback block"
    );
    assert!(
        cr.user_prompt.contains("radial-stack-not-concentric"),
        "user_prompt must echo the actual rejection reason verbatim"
    );
    assert!(
        cr.user_prompt.contains("Keep using the full skill set"),
        "user_prompt must tell the model NOT to simplify in response to the rejection"
    );
}

/// When `retry_feedback` is `None` (attempt 1, or any non-quality-rejection
/// retry), the user prompt must NOT contain the self-check feedback block.
#[test]
fn subagent_prompt_omits_self_check_feedback_when_absent() {
    let st = crate::plan::Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: crate::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    };
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        !cr.user_prompt.contains("SELF-CHECK FIX REQUIRED"),
        "user_prompt must not mention self-check feedback when there is none"
    );
}

/// `RetryFeedback::Geometry` (the `geometry_echo` step) must use DISTINCT
/// wording from the self-check block — "GEOMETRY FIX REQUIRED", not
/// "SELF-CHECK FIX REQUIRED" — so the model can tell "real content with a
/// resolved-layout problem" apart from "rejected before it even landed".
#[test]
fn subagent_prompt_injects_geometry_feedback_with_distinct_wording() {
    let st = crate::plan::Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: crate::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: Some(crate::plan::RetryFeedback::Geometry(
            "Card (n9): resolved 420px wide inside its 390px parent — it spills out".into(),
        )),
    };
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        cr.user_prompt.contains("GEOMETRY FIX REQUIRED"),
        "user_prompt must use the geometry-specific wording, not the self-check one"
    );
    assert!(
        !cr.user_prompt.contains("SELF-CHECK FIX REQUIRED"),
        "the two feedback kinds must not share a label"
    );
    assert!(
        cr.user_prompt.contains("resolved 420px wide"),
        "user_prompt must echo the actual diagnostic line verbatim"
    );
    assert!(
        cr.user_prompt.contains("Keep using the full skill set"),
        "user_prompt must tell the model NOT to simplify in response to the diagnostic"
    );
}

// ── B0: subtask_intent ────────────────────────────────────────────────────

/// subtask_intent must include the original request prompt, the subtask label,
/// and any screen/elements hints so keyword triggers see the full context.
#[test]
fn subtask_intent_includes_prompt_label_and_hints() {
    let req = DesignRequest {
        prompt: "design a polished mobile-app food landing page".into(),
        model: Some("claude".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let mut sub = crate::plan::Subtask {
        id: "header".into(),
        label: "顶部问候栏".into(),
        region: crate::plan::Region {
            width: 390.0,
            height: 96.0,
        },
        id_prefix: "header".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: Some("home".into()),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    };
    let intent = subtask_intent(&req, &sub);
    assert!(
        intent.contains("food landing page"),
        "original prompt keywords"
    );
    assert!(intent.contains("顶部问候栏"), "label");
    assert!(intent.contains("home"), "screen hint");

    // elements hint also included when present
    sub.elements = Some("avatar, greeting text".into());
    let intent2 = subtask_intent(&req, &sub);
    assert!(intent2.contains("avatar, greeting text"), "elements hint");
}

// ── B3: SkillLoadReport returned from build_subagent_prompt ──────────────

/// `build_subagent_prompt` returns a SkillLoadReport whose included
/// entries cover the skills baked into the system prompt, and whose
/// budget_max is non-zero.
#[test]
fn build_subagent_prompt_returns_skill_report() {
    let (call, report) = bsp(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);
    assert!(!call.system_prompt.is_empty());
    assert!(
        !report.included.is_empty(),
        "report must list loaded skills"
    );
    assert!(report.budget_max > 0, "budget_max must be set");
    // budget_used is the sum of included token counts and must be positive
    assert!(report.budget_used > 0, "budget_used must be positive");
    // verify all included entries have names
    assert!(
        report.included.iter().all(|e| !e.name.is_empty()),
        "all included entries must have a name"
    );
}
