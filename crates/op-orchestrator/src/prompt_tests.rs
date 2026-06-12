use super::*;
use crate::plan::{Region, RootFrameSpec};

fn req() -> DesignRequest {
    DesignRequest {
        prompt: "a pricing page".into(),
        model: Some("claude".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

#[test]
fn rich_prompt_has_style_guides_and_suffix() {
    let pp = build_orchestrator_prompt(&req(), PlanningMode::Rich, AbortFlag::new());
    assert_eq!(pp.mode, PlanningMode::Rich);
    assert!(pp.forced_style_guide_name.is_none());
    // style-guide context 经 {{availableStyleGuides}} 注入到 planning skill
    assert!(pp
        .call_request
        .system_prompt
        .contains("Available style guides"));
    // rich 后缀
    assert!(pp
        .call_request
        .system_prompt
        .contains("CRITICAL OUTPUT FORMAT ENFORCEMENT"));
    assert_eq!(pp.call_request.user_prompt, req().prompt);
}

#[test]
fn minimal_prompt_has_short_suffix_no_snippets() {
    let pp = build_orchestrator_prompt(&req(), PlanningMode::Minimal, AbortFlag::new());
    assert!(pp
        .call_request
        .system_prompt
        .contains("OUTPUT ONLY ONE JSON OBJECT"));
    assert!(!pp
        .call_request
        .system_prompt
        .contains("CRITICAL OUTPUT FORMAT ENFORCEMENT"));
}

#[test]
fn compact_prompt_carries_forced_guide_name() {
    let pp = build_orchestrator_prompt(&req(), PlanningMode::Compact, AbortFlag::new());
    assert!(pp.forced_style_guide_name.is_some());
    assert!(pp
        .call_request
        .system_prompt
        .starts_with("You are a UI planning assistant."));
    // compact 不带 rich/minimal 后缀
    assert!(!pp
        .call_request
        .system_prompt
        .contains("CRITICAL OUTPUT FORMAT ENFORCEMENT"));
}

#[test]
fn subagent_prompt_carries_subtask_and_node_format() {
    let st = Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: Some("root".into()),
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };
    let cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(cr.user_prompt.contains("Hero"));
    assert!(cr.user_prompt.contains("hero-"));
    assert!(cr.system_prompt.contains("PenNode"));
}

#[test]
fn subagent_prompt_carries_ts_layout_contract() {
    let mut plan = plan();
    plan.root_frame.width = 390.0;
    plan.subtasks = vec![
        Subtask {
            id: "header".into(),
            label: "Header".into(),
            region: Region {
                width: 390.0,
                height: 96.0,
            },
            id_prefix: "header".into(),
            parent_frame_id: Some("page".into()),
            elements: Some("delivery location".into()),
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
        },
        Subtask {
            id: "categories".into(),
            label: "Food Categories".into(),
            region: Region {
                width: 390.0,
                height: 112.0,
            },
            id_prefix: "categories".into(),
            parent_frame_id: Some("page".into()),
            elements: Some("category chips".into()),
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
        },
    ];
    let cr = build_subagent_prompt(
        &plan.subtasks[1],
        &plan,
        &req(),
        AbortFlag::new(),
        false,
        false,
    );

    assert!(cr.user_prompt.contains("Page sections:"));
    assert!(cr.user_prompt.contains("Food Categories [category chips]"));
    assert!(cr
        .user_prompt
        .contains("Root frame: id=\"categories-root\""));
    assert!(cr.user_prompt.contains("width=\"fill_container\""));
    assert!(cr.user_prompt.contains("height=\"fit_content\""));
    assert!(cr.user_prompt.contains("Generate enough elements"));
    assert!(cr.user_prompt.contains("MOBILE STATUS BAR"));
    assert!(cr.user_prompt.contains("time, signal, wifi, battery"));
    assert!(cr.user_prompt.contains("NO PHONE MOCKUP WRAPPER"));
    assert!(cr.user_prompt.contains("MOBILE WIDTH SAFETY"));
    assert!(cr.user_prompt.contains("MOBILE SECTION INSETS"));
    assert!(cr.user_prompt.contains("MOBILE SEARCH ACTIONS"));
    assert!(cr.user_prompt.contains("NO BLANK PLACEHOLDERS"));
    assert!(cr.user_prompt.contains("MOBILE NAV SURFACE"));
    assert!(cr.user_prompt.contains("TYPOGRAPHY HIERARCHY"));
}

#[test]
fn subagent_prompt_minimal_skills_only_has_schema_and_jsonl() {
    let st = Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };
    // minimal_skills=true: the system prompt should contain "schema" skill
    // content and "jsonl-format" skill content, but NOT layout/text-rules etc.
    let cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, true);
    // schema and jsonl-format skills should appear (they always exist)
    assert!(
        cr.system_prompt.contains("PenNode"),
        "NODE_FORMAT suffix should still be appended"
    );
    // The system_prompt should be considerably shorter than a full-skill prompt
    let full_cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        cr.system_prompt.len() < full_cr.system_prompt.len(),
        "minimal_skills prompt should be shorter than full-skill prompt"
    );
}

#[test]
fn subagent_prompt_reduced_complexity_basic_is_shorter_than_full() {
    let st = Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };
    // req() uses model "claude" which is Full tier — no narrowing.
    // Use a basic-tier model to test narrowing.
    let basic_req = DesignRequest {
        prompt: "a page".into(),
        model: Some("claude-haiku".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    };
    let full_cr = build_subagent_prompt(&st, &plan(), &basic_req, AbortFlag::new(), false, false);
    let reduced_cr = build_subagent_prompt(&st, &plan(), &basic_req, AbortFlag::new(), true, false);
    assert!(
        reduced_cr.system_prompt.len() <= full_cr.system_prompt.len(),
        "reduced_complexity Basic prompt should be no longer than full-skill prompt"
    );
}

#[test]
fn subagent_prompt_reduced_complexity_full_tier_is_noop() {
    let st = Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };
    // req() uses "claude" which maps to Full tier → reduced_complexity is no-op
    let full_cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
    let reduced_cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), true, false);
    assert_eq!(
        full_cr.system_prompt, reduced_cr.system_prompt,
        "reduced_complexity on Full tier should be a no-op"
    );
}

/// Regression guard for the flag-passing fix: a Basic-tier model must load
/// `jsonl-format-simplified` (gated by the `isBasicTier` flag) and drop the
/// verbose `jsonl-format`, while a Full-tier model keeps the verbose one.
/// Before the fix, `resolve_generation_skills` passed empty flags, so the
/// simplified skill could NEVER load for the weak models it targets.
#[test]
fn subagent_prompt_basic_tier_swaps_in_simplified_format_skill() {
    // Verbose-only marker (lives solely in jsonl-format.md) and simplified-only
    // marker (the parenthesized rectangle arg list lives solely in
    // jsonl-format-simplified.md).
    const VERBOSE_ONLY: &str = "imageSearchQuery MUST be UNIQUE";
    const SIMPLIFIED_ONLY: &str = "rectangle (width,height,cornerRadius,fill)";

    let basic_req = DesignRequest {
        prompt: "a page".into(),
        model: Some("claude-haiku".into()), // Basic tier
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
    };
    let basic_cr = build_subagent_prompt(
        &subtask(),
        &plan(),
        &basic_req,
        AbortFlag::new(),
        false,
        false,
    );
    // req() is model "claude" → Full tier.
    let full_cr =
        build_subagent_prompt(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);

    assert!(
        basic_cr.system_prompt.contains(SIMPLIFIED_ONLY),
        "Basic tier must load jsonl-format-simplified"
    );
    assert!(
        !basic_cr.system_prompt.contains(VERBOSE_ONLY),
        "Basic tier must NOT carry the verbose jsonl-format (deduped by simplified)"
    );
    assert!(
        full_cr.system_prompt.contains(VERBOSE_ONLY),
        "Full tier must keep the verbose jsonl-format"
    );
    assert!(
        !full_cr.system_prompt.contains(SIMPLIFIED_ONLY),
        "Full tier must NOT load jsonl-format-simplified (isBasicTier is false)"
    );
}

/// design-system is dropped whenever another styling source covers it:
/// (a) no style guide → `style-defaults` loads; (b) a guide IS named → the
/// style-guide instruction block (G2) is injected. In both cases the generic
/// design-system skill is replaced, not kept alongside (Codex review +
/// buildSubAgentStyleGuideInstruction port).
#[test]
fn subagent_prompt_drops_design_system_when_styling_covered() {
    const DESIGN_SYSTEM_ONLY: &str = "design system architect";
    const STYLE_DEFAULTS_ONLY: &str = "VISUAL STYLE POLICY";

    // (a) No style guide named, no design.md → noStyleGuideMatch → style-defaults
    // loads and covers styling, so design-system is dropped.
    let covered =
        build_subagent_prompt(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        covered.system_prompt.contains(STYLE_DEFAULTS_ONLY),
        "no-style-guide prompt should load style-defaults"
    );
    assert!(
        !covered.system_prompt.contains(DESIGN_SYSTEM_ONLY),
        "design-system dropped when style-defaults covers styling"
    );

    // (b) A style guide IS named → G2 injects its palette/fonts block, which
    // REPLACES design-system. style-defaults does NOT load (noStyleGuideMatch
    // false).
    let mut sg_plan = plan();
    sg_plan.style_guide_name = Some("saas-clean-light".into());
    let with_guide =
        build_subagent_prompt(&subtask(), &sg_plan, &req(), AbortFlag::new(), false, false);
    assert!(
        with_guide.system_prompt.contains("VISUAL STYLE GUIDE"),
        "named style guide injects its instruction block"
    );
    assert!(
        !with_guide.system_prompt.contains(DESIGN_SYSTEM_ONLY),
        "design-system dropped when the style-guide block replaces it"
    );
    assert!(
        !with_guide.system_prompt.contains(STYLE_DEFAULTS_ONLY),
        "named style guide should NOT load style-defaults"
    );
}

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
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    };
    let long_prompt = "x".repeat(5000); // >= 4200 chars
    let long_req = DesignRequest {
        prompt: long_prompt,
        model: Some("claude-sonnet".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
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
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    };
    let pp = build_orchestrator_prompt(&ds_req, PlanningMode::Rich, AbortFlag::new());
    // Short bucket base: 300_000ms × 2.0 = 600_000ms
    assert_eq!(
        pp.call_request.timeout,
        std::time::Duration::from_millis(600_000),
        "deepseek-v4-pro multiplier=2.0 should double the short-bucket timeout"
    );
}

fn subtask() -> crate::plan::Subtask {
    crate::plan::Subtask {
        id: "s".into(),
        label: "Section".into(),
        region: crate::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "s".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    }
}

/// Sub-agent prompt has profile-derived timeouts (not None).
#[test]
fn subagent_prompt_has_profile_timeouts() {
    let cr = build_subagent_prompt(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);
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
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    };
    let long_req = DesignRequest {
        prompt: "x".repeat(5000),
        model: Some("claude-sonnet".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    };
    let short_cr = build_subagent_prompt(
        &subtask(),
        &plan(),
        &short_req,
        AbortFlag::new(),
        false,
        false,
    );
    let long_cr = build_subagent_prompt(
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
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    };
    let cr = build_subagent_prompt(
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
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: Some(vec!["Hero".into(), "Pricing".into()]),
    };
    let cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
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
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };
    let cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
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
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: Some(vec![]),
    };
    let cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        !cr.user_prompt.contains("APPEND MODE"),
        "user_prompt must NOT contain APPEND MODE block when labels is empty"
    );
}

/// Manifest mode (spec 2026-06-10-element-manifest-v2): the element-manifest
/// skill + generated catalog replace the raw-JSONL output protocol, on both
/// Full and Basic tiers, while the default path stays untouched.
#[test]
fn subagent_prompt_manifest_mode_swaps_output_protocol() {
    const VERBOSE_ONLY: &str = "imageSearchQuery MUST be UNIQUE";
    const SIMPLIFIED_ONLY: &str = "rectangle (width,height,cornerRadius,fill)";
    const NODE_FORMAT_ONLY: &str = "FLAT _parent format";
    const MANIFEST_FORMAT_ONLY: &str = "OUTPUT PROTOCOL: ELEMENT MANIFEST JSONL";
    const MANIFEST_SKILL_ONLY: &str = "ELEMENT MANIFEST OUTPUT FORMAT";

    let basic_req = DesignRequest {
        prompt: "a page".into(),
        model: Some("claude-haiku".into()), // Basic tier
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
    };
    for request in [req(), basic_req] {
        let cr = build_subagent_prompt_with_manifest(
            &subtask(),
            &plan(),
            &request,
            AbortFlag::new(),
            false,
            false,
            true,
        );
        let model = request.model.as_deref().unwrap_or("");
        assert!(
            cr.system_prompt.contains(MANIFEST_SKILL_ONLY),
            "{model}: element-manifest skill must load"
        );
        // Recency placement: the skill rides at the END of the prompt
        // (right above the output contract), not at its priority-0
        // position — long Full-tier prompts buried a top-of-prompt
        // catalog (ab-v9 deepseek hand-rolled past it).
        assert!(
            cr.system_prompt.find(MANIFEST_SKILL_ONLY).unwrap() > 1000,
            "{model}: manifest skill must sit near the end, not the top"
        );
        assert!(
            cr.system_prompt.find(MANIFEST_SKILL_ONLY).unwrap()
                < cr.system_prompt.find(MANIFEST_FORMAT_ONLY).unwrap(),
            "{model}: output contract directly follows the catalog"
        );
        assert!(
            cr.system_prompt.contains("- stat_card:"),
            "{model}: generated catalog must be injected"
        );
        assert!(
            cr.system_prompt.contains(MANIFEST_FORMAT_ONLY),
            "{model}: manifest output contract replaces NODE_FORMAT"
        );
        assert!(
            !cr.system_prompt.contains(NODE_FORMAT_ONLY),
            "{model}: NODE_FORMAT must not load in manifest mode"
        );
        assert!(
            !cr.system_prompt.contains(VERBOSE_ONLY) && !cr.system_prompt.contains(SIMPLIFIED_ONLY),
            "{model}: jsonl-format skills are replaced by the manifest protocol"
        );
        assert!(
            cr.user_prompt.contains("Do NOT create a page wrapper"),
            "{model}: user prompt swaps the root-frame rule"
        );
        assert!(
            !cr.user_prompt.contains("-root\""),
            "{model}: no self-authored root id in manifest mode"
        );
    }

    // Default path (manifest off) is byte-stable: NODE_FORMAT + no manifest.
    let off = build_subagent_prompt_with_manifest(
        &subtask(),
        &plan(),
        &req(),
        AbortFlag::new(),
        false,
        false,
        false,
    );
    assert!(off.system_prompt.contains(NODE_FORMAT_ONLY));
    assert!(!off.system_prompt.contains(MANIFEST_FORMAT_ONLY));
    assert!(!off.system_prompt.contains(MANIFEST_SKILL_ONLY));
}

/// Manifest mode nominates catalog kinds from the subtask's own text and
/// injects them as an ELEMENT HINTS block; raw mode and hint-less
/// subtasks stay clean (ab-v9.1 adoption de-randomization).
#[test]
fn subagent_prompt_manifest_mode_injects_element_hints() {
    let mut st = subtask();
    st.label = "Notification Settings Row".into();
    st.elements = Some("bell icon, text stack, iOS toggle switch".into());
    let build = |st: &crate::plan::Subtask, manifest_on: bool| {
        build_subagent_prompt_with_manifest(
            st,
            &plan(),
            &req(),
            AbortFlag::new(),
            false,
            false,
            manifest_on,
        )
    };

    let cr = build(&st, true);
    assert!(
        cr.user_prompt.contains("ELEMENT HINTS:"),
        "hint block loads"
    );
    assert!(
        cr.user_prompt.contains("setting_row"),
        "composite kind hinted"
    );
    assert!(cr.user_prompt.contains("switch"), "part kind hinted");

    let raw = build(&st, false);
    assert!(
        !raw.user_prompt.contains("ELEMENT HINTS"),
        "raw JSONL mode has no catalog to hint from"
    );

    let none = build(&subtask(), true);
    assert!(
        !none.user_prompt.contains("ELEMENT HINTS"),
        "no matches must omit the block, not emit it empty"
    );
}
