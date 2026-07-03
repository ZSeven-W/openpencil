use super::*;
use crate::plan::{Region, RootFrameSpec};
use op_editor_core::ComponentLibrary;

/// Test shim: the production `build_subagent_prompt` gained a `components`
/// param (the AVAILABLE COMPONENTS manifest source). The vast majority of
/// these tests predate it and exercise the no-component path, so this forwards
/// an empty registry — behaviour identical to before that param existed.
/// Component-aware behaviour is covered by the dedicated `available_components_*`
/// tests, which call `build_subagent_prompt` directly with a populated library.
fn bsp(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
) -> (CallRequest, SkillLoadReport) {
    build_subagent_prompt(
        subtask,
        plan,
        req,
        abort,
        reduced_complexity,
        minimal_skills,
        &ComponentLibrary::default(),
    )
}

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
fn provider_planning_prompt_carries_quality_guardrails() {
    let pp = build_orchestrator_prompt(&req(), PlanningMode::Rich, AbortFlag::new());
    let prompt = &pp.call_request.system_prompt;
    assert!(prompt.contains("PLANNING QUALITY GUARDRAILS"));
    assert!(prompt.contains("Do not plan the same predictable mobile stack"));
    assert!(prompt.contains("Mobile top rhythm"));
    assert!(prompt.contains("signature moment"));
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

/// Script-gen is THE default generation protocol on the full attempt (no env
/// var needed — see the Task 4 protocol collapse): the system prompt carries
/// SCRIPT_FORMAT, not NODE_FORMAT/`_parent` JSONL.
#[test]
fn subagent_prompt_carries_subtask_and_script_format() {
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
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(cr.user_prompt.contains("Hero"));
    assert!(
        cr.system_prompt
            .contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"),
        "full attempt must use SCRIPT_FORMAT by default:\n{}",
        cr.system_prompt
    );
}

/// The reduced-complexity retry rung falls back to flat `_parent` JSONL
/// (NODE_FORMAT) — matching what `subagent::run_subtask` parses there.
#[test]
fn subagent_prompt_reduced_complexity_carries_node_format() {
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
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), true, false);
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
    let (cr, _) = bsp(
        &plan.subtasks[1],
        &plan,
        &req(),
        AbortFlag::new(),
        false,
        false,
    );

    let required = "Page sections:|Food Categories [category chips]|\"fill_container\"|\"fit_content\"|Generate enough elements|MOBILE STATUS BAR|time, signal, wifi, battery|NO PHONE MOCKUP WRAPPER|MOBILE WIDTH SAFETY|MOBILE SINGLE CONTENT RAIL|MOBILE SEARCH BAR|MOBILE SECTION CHROME|MOBILE VERTICAL RHYTHM|MOBILE TOP RHYTHM|MOBILE GRID ALIGNMENT|MOBILE CARD OVERLAYS|MOBILE IMAGE QUALITY|NO BLANK PLACEHOLDERS|MOBILE NAV SURFACE|MOBILE NAV SHADOW|NO FIXED FOOD TEMPLATE|Do not default to the same search + categories + orange promo + two product cards composition|TYPOGRAPHY HIERARCHY|DENSITY|VISUAL HIERARCHY|SPACING CONSISTENCY|CRAFT POLISH|MEDIA CONSISTENCY|ICON SCALE|SIGNATURE MOMENT|WOW FACTOR|COMPOSITIONAL CONTRAST|PREMIUM DETAIL|NO DECORATION SPAM";
    // Mobile UI guardrails now load via the `mobile-ui` skill (system prompt);
    // section + quality markers stay in the user prompt. Accept either. The
    // `"fill_container"` / `"fit_content"` markers are quote-only (no
    // `width=` / `height=` prefix) so they match both the flat-JSONL root_rule
    // wording and script-gen's `width:"fill_container"` JS-object wording —
    // the root-frame AUTHORING convention itself (`Root frame: id="..."` vs
    // `const sec = I(null, ...)`) is protocol-specific and covered by the
    // dedicated `subagent_prompt_carries_subtask_and_script_format` /
    // `subagent_prompt_reduced_complexity_carries_node_format` tests.
    for required in required.split('|') {
        assert!(
            cr.system_prompt.contains(required) || cr.user_prompt.contains(required),
            "missing {required}"
        );
    }
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
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, true);
    // schema and jsonl-format skills should appear (they always exist)
    assert!(
        cr.system_prompt.contains("PenNode"),
        "NODE_FORMAT suffix should still be appended"
    );
    // The system_prompt should be considerably shorter than a full-skill prompt
    let (full_cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
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
    let (full_cr, _) = bsp(&st, &plan(), &basic_req, AbortFlag::new(), false, false);
    let (reduced_cr, _) = bsp(&st, &plan(), &basic_req, AbortFlag::new(), true, false);
    assert!(
        reduced_cr.system_prompt.len() <= full_cr.system_prompt.len(),
        "reduced_complexity Basic prompt should be no longer than full-skill prompt"
    );
}

/// Before the Task 4 protocol collapse, `reduced_complexity` only drove
/// tier-gated SKILL narrowing (a no-op for Full tier). It now ALSO selects
/// the output protocol (script-gen on the full attempt, flat JSONL on the
/// reduced rung) for EVERY tier — so holding `script_on` fixed via the core
/// fn isolates the still-true "Full tier skill narrowing is a no-op"
/// invariant from that new, tier-independent protocol switch.
#[test]
fn subagent_prompt_reduced_complexity_full_tier_skill_filtering_is_noop() {
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
    // req() uses "claude" which maps to Full tier → reduced_complexity's skill
    // narrowing is a no-op there (unlike Basic, which drops to the
    // retryAllowed 8-skill set). script_on held fixed at `true` for both calls.
    let (full_cr, _) = build_subagent_prompt_core(
        &st,
        &plan(),
        &req(),
        AbortFlag::new(),
        false,
        false,
        true,
        &ComponentLibrary::default(),
    );
    let (reduced_cr, _) = build_subagent_prompt_core(
        &st,
        &plan(),
        &req(),
        AbortFlag::new(),
        true,
        false,
        true,
        &ComponentLibrary::default(),
    );
    assert_eq!(
        full_cr.system_prompt, reduced_cr.system_prompt,
        "reduced_complexity's skill narrowing should be a no-op on Full tier"
    );
}

/// The new, tier-independent half of the same invariant: with `script_on`
/// left to its normal `reduced_complexity`-derived value, the full attempt
/// and the reduced rung now legitimately differ — even on Full tier — because
/// `reduced_complexity` also flips the output protocol.
#[test]
fn subagent_prompt_reduced_complexity_flips_protocol_even_on_full_tier() {
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
    let (full_cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    let (reduced_cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), true, false);
    assert!(
        full_cr
            .system_prompt
            .contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"),
        "full attempt uses script-gen even on Full tier"
    );
    assert!(
        reduced_cr.system_prompt.contains("PenNode"),
        "reduced rung falls back to flat JSONL even on Full tier"
    );
    assert_ne!(full_cr.system_prompt, reduced_cr.system_prompt);
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
    // jsonl-format-simplified.md). Both `jsonl-format` skills are dropped
    // outright on the script-gen full attempt (SCRIPT_FORMAT governs the
    // output contract alone), so this exercises the reduced-complexity JSONL
    // rung, where the tier-based swap is actually live.
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
    let (basic_cr, _) = bsp(
        &subtask(),
        &plan(),
        &basic_req,
        AbortFlag::new(),
        true,
        false,
    );
    // req() is model "claude" → Full tier.
    let (full_cr, _) = bsp(&subtask(), &plan(), &req(), AbortFlag::new(), true, false);

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

#[test]
fn subagent_prompt_basic_mobile_food_keeps_mobile_app_skill() {
    let mobile_req = DesignRequest {
        prompt: "Design a 402x874 mobile food delivery home screen with search, categories, promo offer, restaurant cards, and bottom navigation".into(),
        model: Some("claude-haiku".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
    };
    let mut mobile_plan = plan();
    mobile_plan.root_frame.width = 402.0;
    mobile_plan.root_frame.height = 874.0;
    mobile_plan.style_guide_name = Some("warm-food-mobile-light".into());
    let mobile_subtask = Subtask {
        id: "main-content".into(),
        label: "Main Content".into(),
        region: Region {
            width: 402.0,
            height: 640.0,
        },
        id_prefix: "main-content".into(),
        parent_frame_id: Some("page".into()),
        elements: Some(
            "search, filters, category chips, promotional banner, restaurant cards".into(),
        ),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };

    let (_, report) = bsp(
        &mobile_subtask,
        &mobile_plan,
        &mobile_req,
        AbortFlag::new(),
        false,
        false,
    );
    let mobile_entry = report
        .included
        .iter()
        .find(|entry| entry.name == "mobile-app");
    assert!(
        mobile_entry.is_some(),
        "Basic mobile prompts must keep mobile-app guidance; report={report:?}"
    );
    assert!(
        !mobile_entry.unwrap().truncated,
        "mobile-app carries bottom-nav and top-rhythm rules and must not be truncated; report={report:?}"
    );
}

#[test]
fn subagent_prompt_honors_explicit_radius_and_spacing_numbers() {
    let mobile_req = DesignRequest {
        prompt: "设计一个美食移动端首页，圆角和间距要统一，圆角 8 px，间距 12 px".into(),
        model: Some("claude-haiku".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
    };
    let mut mobile_plan = plan();
    mobile_plan.root_frame.width = 402.0;
    mobile_plan.root_frame.height = 874.0;
    mobile_plan.style_guide_name = Some("warm-food-mobile-light".into());
    let subtask = Subtask {
        id: "content".into(),
        label: "Content".into(),
        region: Region {
            width: 402.0,
            height: 640.0,
        },
        id_prefix: "content".into(),
        parent_frame_id: Some("page".into()),
        elements: Some("search, categories, promo, restaurant cards".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };

    let (cr, _) = bsp(
        &subtask,
        &mobile_plan,
        &mobile_req,
        AbortFlag::new(),
        false,
        false,
    );
    assert!(
        cr.user_prompt
            .contains("EXPLICIT USER DESIGN TOKENS: cornerRadius must be 8px"),
        "missing explicit radius override:\n{}",
        cr.user_prompt
    );
    assert!(
        cr.user_prompt.contains("layout gap/spacing must be 12px"),
        "missing explicit spacing override:\n{}",
        cr.user_prompt
    );
    assert!(
        !cr.user_prompt.contains("cornerRadius 14-18"),
        "mobile search guidance must not contradict explicit 8px radius:\n{}",
        cr.user_prompt
    );
}

#[test]
fn mobile_food_prompt_avoids_fixed_food_template() {
    let mobile_req = DesignRequest {
        prompt: "设计一个美食应用移动端首页，希望好看点".into(),
        model: Some("claude-haiku".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
    };
    let mut mobile_plan = plan();
    mobile_plan.root_frame.width = 402.0;
    mobile_plan.root_frame.height = 874.0;
    // Test the GENERAL mobile path (no specific style guide). A domain style
    // guide is a separate, opt-in source of layout rules — this test verifies
    // the general pipeline no longer hardcodes the fixed food template.
    mobile_plan.style_guide_name = None;
    let subtask = Subtask {
        id: "content".into(),
        label: "Content".into(),
        region: Region {
            width: 402.0,
            height: 640.0,
        },
        id_prefix: "content".into(),
        parent_frame_id: Some("page".into()),
        elements: Some("header, search, categories, featured dish, popular list".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };

    let (cr, _) = bsp(
        &subtask,
        &mobile_plan,
        &mobile_req,
        AbortFlag::new(),
        false,
        false,
    );
    // Mobile UI guardrails now load via the `mobile-ui` skill (system prompt),
    // so check the combined prompt.
    let combined = format!("{}\n{}", cr.system_prompt, cr.user_prompt);
    // The hardcoded food-template anatomy is REMOVED — it locked every food app
    // into the same structure (user direction 2026-06-23). It must appear in
    // NEITHER prompt.
    assert!(
        !combined.contains("MOBILE FOOD POLISH"),
        "the fixed food-template rule must not be injected:\n{combined}"
    );
    assert!(
        !combined.contains("never use space_between or space_around for category chips"),
        "category-rail distribution must NOT be hardcoded:\n{combined}"
    );
    assert!(
        !combined.contains("Product card rows use two equal fill_container cards"),
        "product-card layout must NOT be hardcoded:\n{combined}"
    );
    // The variation guidance + general (non-food-specific) alignment rules stay.
    assert!(
        combined.contains("NO FIXED FOOD TEMPLATE"),
        "variation guidance must remain:\n{combined}"
    );
    assert!(
        combined.contains("MOBILE GRID ALIGNMENT"),
        "general grid-alignment guidance must remain:\n{combined}"
    );
    // Every nav tab keeps its label (the cart-tab-loses-label fix).
    assert!(
        combined.contains("never emit a tab (e.g. cart) with an icon but no label"),
        "nav tabs must all keep a label:\n{combined}"
    );
    // The filter button is a neutral surface, not an accent-filled dark-icon button.
    assert!(
        combined.contains("Do NOT make it an accent-filled"),
        "filter button must be neutral, not orange-on-dark:\n{combined}"
    );
}

#[test]
fn chinese_mobile_food_prompt_carries_language_consistency_rule() {
    let mobile_req = DesignRequest {
        prompt: "设计一个美食应用移动端首页，包含配送地址、搜索、分类和主题推荐".into(),
        model: Some("claude-haiku".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
    };
    let mut mobile_plan = plan();
    mobile_plan.root_frame.width = 390.0;
    mobile_plan.root_frame.height = 844.0;
    let subtask = Subtask {
        id: "content".into(),
        label: "首页内容".into(),
        region: Region {
            width: 390.0,
            height: 640.0,
        },
        id_prefix: "content".into(),
        parent_frame_id: Some("page".into()),
        elements: Some("配送地址、搜索框、美食分类、主题推荐卡片".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };

    let (cr, report) = bsp(
        &subtask,
        &mobile_plan,
        &mobile_req,
        AbortFlag::new(),
        false,
        false,
    );

    assert!(
        report
            .included
            .iter()
            .any(|entry| entry.name == "cjk-typography"),
        "Chinese prompts must keep cjk-typography guidance; report={report:?}"
    );
    assert!(
        cr.system_prompt.contains("LANGUAGE CONSISTENCY"),
        "Chinese prompts must forbid mixed English boilerplate:\n{}",
        cr.system_prompt
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
    let (covered, _) = bsp(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);
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
    let (with_guide, _) = bsp(&subtask(), &sg_plan, &req(), AbortFlag::new(), false, false);
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
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
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
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: Some(vec!["Hero".into(), "Pricing".into()]),
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
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
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
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: Some(vec![]),
    };
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    assert!(
        !cr.user_prompt.contains("APPEND MODE"),
        "user_prompt must NOT contain APPEND MODE block when labels is empty"
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
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
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
        elements: None,
        screen: Some("home".into()),
        generated_root_id: None,
        existing_section_labels: None,
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

// ── Available-components manifest (Stage 2 Part B) ───────────────────────────

use jian_ops_schema::node::PenNode;
use op_editor_core::{Component, NodeId};

/// Build a `ComponentLibrary` with `n` reusable masters whose names cycle
/// through a few categories so the grouped manifest exercises bucketing.
/// The manifest only reads each component's `id` + `name`, so the `root`
/// frame is a minimal reusable-flagged stub.
fn library_with(n: usize) -> ComponentLibrary {
    let names = [
        "Primary Button",
        "Search Input",
        "Stat Card",
        "Nav Item",
        "Status Badge",
        "User Avatar",
        "Confirm Dialog",
        "Table Row",
        "Page Header",
    ];
    let mut lib = ComponentLibrary::default();
    for i in 0..n {
        let id = format!("comp-{i}");
        let name = format!("{} {i}", names[i % names.len()]);
        let root: PenNode = serde_json::from_value(serde_json::json!({
            "id": id,
            "type": "frame",
            "name": name,
            "reusable": true,
            "width": 100,
            "height": 40,
        }))
        .expect("frame fixture");
        lib.insert(Component {
            id: NodeId::new(&id),
            name,
            root,
        });
    }
    lib
}

/// With NO components, the prompt is unchanged: no AVAILABLE COMPONENTS block
/// and no `ref` teaching from the `component-composition` skill.
#[test]
fn no_components_prompt_omits_manifest_and_ref_teaching() {
    let (cr, report) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &req(),
        AbortFlag::new(),
        false,
        false,
        &ComponentLibrary::default(),
    );
    assert!(
        !cr.system_prompt.contains("AVAILABLE COMPONENTS"),
        "empty library must not inject the components manifest"
    );
    // The component-composition skill only loads behind `hasReusableComponents`.
    assert!(
        !report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "component-composition skill must not load without components"
    );
    // And the empty-library prompt must byte-match the no-arg path (the `bsp`
    // shim forwards an empty library too).
    let (baseline, _) = bsp(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);
    assert_eq!(cr.system_prompt, baseline.system_prompt);
}

/// A Full-tier request (no budget override, no Basic allow-set) so the
/// flag-gated `component-composition` skill reliably survives filtering.
fn full_req() -> DesignRequest {
    DesignRequest {
        model: Some("claude-opus-4".into()),
        ..req()
    }
}

/// With components present, the prompt injects the AVAILABLE COMPONENTS
/// manifest (concrete ids), the `ref` teaching, and loads the
/// `component-composition` skill.
#[test]
fn components_prompt_injects_manifest_and_ref_teaching() {
    let lib = library_with(5);
    let (cr, report) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &full_req(),
        AbortFlag::new(),
        false,
        false,
        &lib,
    );
    let sys = &cr.system_prompt;
    assert!(
        sys.contains("AVAILABLE COMPONENTS"),
        "manifest header must be present"
    );
    // Concrete ids from the registry are listed.
    assert!(sys.contains("comp-0"), "manifest must list component ids");
    assert!(sys.contains("comp-4"), "manifest must list all 5 ids");
    // The `ref` instantiation teaching is present.
    assert!(
        sys.contains("\"type\":\"ref\""),
        "manifest must teach the ref node syntax"
    );
    // Category grouping appears (button → Buttons bucket).
    assert!(sys.contains("Buttons:"), "manifest groups by category");
    // The component-composition skill loaded behind the flag.
    assert!(
        report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "component-composition skill must load when components exist"
    );
}

/// A large library is capped: the manifest lists at most
/// `MAX_COMPONENT_MANIFEST_ENTRIES` and notes the remainder, so the prompt
/// budget can't be blown by a 200-master kit.
#[test]
fn large_component_library_is_capped() {
    let lib = library_with(200);
    let (cr, _) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &req(),
        AbortFlag::new(),
        false,
        false,
        &lib,
    );
    let sys = &cr.system_prompt;
    // The header reports the true total even though the body is capped.
    assert!(sys.contains("AVAILABLE COMPONENTS (200 reusable"));
    assert!(
        sys.contains("more not listed"),
        "capped manifest must note the remainder"
    );
    // The number of listed `- id (name)` rows must not exceed the cap.
    let listed = sys.matches("  - comp-").count();
    assert!(
        listed <= MAX_COMPONENT_MANIFEST_ENTRIES,
        "listed {listed} entries exceeds cap {MAX_COMPONENT_MANIFEST_ENTRIES}"
    );
}

/// Regression guard for the protocol-mismatch stop-gate: the AVAILABLE
/// COMPONENTS manifest's trailing ref-syntax example must match whichever
/// output protocol governs the REST of this prompt — script-gen (`I(...)`
/// calls) on the full attempt, flat `_parent` JSONL on the
/// reduced-complexity retry rung. Teaching the wrong dialect makes every
/// `ref` silently vanish: a bare `{"_parent":...}` line is never recorded by
/// the script sandbox (only `I(...)` calls are), so under script-gen the
/// old always-flat instruction taught the model a no-op.
///
/// Uses `full_req()` (Full tier) for both calls so `reduced_complexity`'s
/// tier-gated SKILL narrowing stays a no-op (per
/// `subagent_prompt_reduced_complexity_full_tier_skill_filtering_is_noop`)
/// and the manifest + `component-composition` skill both survive on either
/// side — isolating the assertion to the protocol switch alone.
#[test]
fn components_manifest_instruction_matches_active_protocol() {
    let lib = library_with(3);

    // (a) Full attempt: script-gen is THE default protocol. The manifest
    // must teach the `I(...)` ref call and must NOT teach the flat `_parent`
    // ref line.
    let (script_cr, _) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &full_req(),
        AbortFlag::new(),
        false,
        false,
        &lib,
    );
    let script_sys = &script_cr.system_prompt;
    assert!(
        script_sys.contains("I(<containerBinding>, {\"type\":\"ref\""),
        "full-attempt (script-gen) manifest must teach the I(...) ref call:\n{script_sys}"
    );
    assert!(
        !script_sys.contains("\"_parent\":\"<container-id>\""),
        "full-attempt (script-gen) manifest must NOT teach the flat _parent ref line \
         — a bare {{\"_parent\":...}} line is never recorded by the script sandbox:\n{script_sys}"
    );

    // (b) Reduced-complexity retry rung: falls back to flat `_parent` JSONL.
    // The manifest must teach the `_parent` ref line.
    let (flat_cr, _) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &full_req(),
        AbortFlag::new(),
        true,
        false,
        &lib,
    );
    let flat_sys = &flat_cr.system_prompt;
    assert!(
        flat_sys.contains("\"_parent\":\"<container-id>\""),
        "reduced-complexity manifest must teach the flat _parent ref line:\n{flat_sys}"
    );
}

/// Regression guard for the tier-drop bug (smoke `OPENPENCIL_SMOKE_LIBRARY`
/// scenario): a Basic-tier model with a component library loaded must get BOTH
/// the AVAILABLE COMPONENTS manifest (concrete ids) AND the
/// `component-composition` teaching (the `ref` + `descendants` syntax) in its
/// assembled subtask prompt.
///
/// Before the fix the `component-composition` skill resolved in behind the
/// `hasReusableComponents` flag but was then dropped by the Basic-tier
/// `ALLOWED` allow-set (DropReason::TierFiltered) even with budget room
/// (`budget_used < budget_max`) — so a weak model saw the component list with
/// no instruction on how to emit a `ref` node and built everything from
/// scratch (0 component instances). Reproduces the real smoke path: a MiniMax
/// (Basic-tier) mobile screen, whose 9200-token budget has room to spare.
#[test]
fn basic_tier_components_prompt_keeps_both_manifest_and_teaching() {
    // Sanity: the model classifies as Basic, the path that drops non-allowed
    // skills via the allow-set (the bug surface).
    assert_eq!(
        resolve_model_profile("minimax-m3").tier,
        ModelTier::Basic,
        "test fixture must exercise the Basic tier"
    );

    // A Basic-tier mobile request — the actual smoke scenario. Mobile routes
    // through the wider 9200-token budget so the drop is provably tier-caused,
    // not budget-caused.
    let basic_req = DesignRequest {
        prompt: "Design a 402x874 mobile shop home screen with product cards, \
                 search, and bottom navigation using the available components"
            .into(),
        model: Some("minimax-m3".into()),
        ..req()
    };
    let mut mobile_plan = plan();
    mobile_plan.root_frame.width = 402.0;
    mobile_plan.root_frame.height = 874.0;
    let mobile_subtask = Subtask {
        id: "main-content".into(),
        label: "Main Content".into(),
        region: Region {
            width: 402.0,
            height: 640.0,
        },
        id_prefix: "main-content".into(),
        parent_frame_id: Some("page".into()),
        elements: Some("product cards, search bar, category chips".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };

    let lib = library_with(5);
    let (cr, report) = build_subagent_prompt(
        &mobile_subtask,
        &mobile_plan,
        &basic_req,
        AbortFlag::new(),
        false,
        false,
        &lib,
    );
    let sys = &cr.system_prompt;

    // The drop the fix removes was budget-room-permitting: prove there was
    // headroom so the original drop can only have been the tier allow-set.
    assert!(
        report.budget_used < report.budget_max,
        "fixture must have budget headroom (the bug dropped despite room); report={report:?}"
    );

    // (1) The AVAILABLE COMPONENTS manifest reached the system prompt with
    // concrete ids — it is a plain appended block, never tier-dropped.
    assert!(
        sys.contains("AVAILABLE COMPONENTS"),
        "Basic-tier prompt must carry the components manifest"
    );
    assert!(
        sys.contains("comp-0") && sys.contains("comp-4"),
        "manifest must list the concrete component ids"
    );
    assert!(
        sys.contains("\"type\":\"ref\""),
        "manifest must point at the ref node syntax"
    );

    // (2) The component-composition TEACHING skill survived the Basic allow-set
    // (this is the part the bug dropped).
    assert!(
        report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "Basic tier must KEEP component-composition when a library is present; report={report:?}"
    );
    assert!(
        !report
            .dropped
            .iter()
            .any(|s| s.name == "component-composition"),
        "component-composition must not be tier-dropped; dropped={:?}",
        report.dropped
    );

    // (3) The teaching skill's actual body (the `ref` + `descendants` rules)
    // is present in the system prompt, not just listed in the report.
    assert!(
        sys.contains("COMPONENT COMPOSITION"),
        "the component-composition skill body must be in the system prompt"
    );
    assert!(
        sys.contains("descendants"),
        "the prompt must teach overriding instance content via descendants"
    );
}

/// Regression guard for the BUDGET-drop bug — the non-mobile dashboard path.
///
/// The earlier `basic_tier_components_prompt_keeps_both_*` test covers the
/// MOBILE path (9200-token budget with headroom), which only ever exercised the
/// TIER allow-set drop. The real loss in production is on the NON-MOBILE,
/// Basic-tier dashboard path: `budget_max = 5200`, base skills alone consume
/// ~3900, so the flag-gated `component-composition` skill (~1200 tok) does NOT
/// fit and was dropped with `DropReason::BudgetExhausted` — the model got the
/// AVAILABLE COMPONENTS list but no `ref` + `descendants` teaching and emitted 0
/// instances (`("component-composition","budget")` ×4 subtasks across runs).
///
/// The force-include pin (prompt.rs: `pinned_skills` when `has_reusable_components`,
/// threaded into `trim_by_budget_pinned`) keeps it budget-exempt. This test
/// reproduces the EXACT scenario that dropped it (wide plan ⇒ 5200 budget, a
/// library present, budget already exhausted) and asserts the teaching survives.
#[test]
fn tight_budget_dashboard_force_includes_component_composition() {
    // Basic tier is the path that overrides the budget down to 5200 when the
    // plan is NOT a mobile full screen (the bug surface).
    assert_eq!(
        resolve_model_profile("minimax-m3").tier,
        ModelTier::Basic,
        "fixture must exercise the Basic tier (5200 budget on non-mobile)"
    );

    // A wide (non-mobile) dashboard plan → is_mobile_full_screen = false →
    // budget_override = Some(5200), the exact tight path that budget-dropped it.
    let basic_req = DesignRequest {
        prompt: "Design a 1280x800 analytics dashboard with metric cards, \
                 a chart panel, and a data table using the available components"
            .into(),
        model: Some("minimax-m3".into()),
        ..req()
    };
    let mut dash_plan = plan();
    dash_plan.root_frame.width = 1280.0;
    dash_plan.root_frame.height = 800.0;
    let dash_subtask = Subtask {
        id: "main".into(),
        label: "Main".into(),
        region: Region {
            width: 1280.0,
            height: 600.0,
        },
        id_prefix: "main".into(),
        parent_frame_id: Some("page".into()),
        elements: Some("metric cards, chart, table".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };

    let lib = library_with(5);
    // Drive the core with script_on forced OFF so this exercises the
    // FLAT-JSONL tight-budget path the test is about — deterministically,
    // regardless of the model's default protocol. (script-gen is THE default
    // on the full attempt and drops the jsonl-format skill, freeing budget;
    // that would un-exhaust the 5200 budget and void the pin scenario.)
    let (cr, report) = build_subagent_prompt_core(
        &dash_subtask,
        &dash_plan,
        &basic_req,
        AbortFlag::new(),
        false,
        false,
        false,
        &lib,
    );
    let sys = &cr.system_prompt;

    // (0) Prove this is the TIGHT path: the 5200 budget is genuinely exhausted —
    // budget_used >= budget_max — so the survival of component-composition can
    // ONLY be the force-include pin, not leftover headroom. (Before the fix this
    // same exhaustion is what dropped it with DropReason::BudgetExhausted.)
    assert_eq!(
        report.budget_max, 5200,
        "non-mobile Basic must use the 5200 budget"
    );
    assert!(
        report.budget_used >= report.budget_max,
        "fixture must EXHAUST the budget so the pin is the only thing keeping the \
         skill (the bug dropped it here); report={report:?}"
    );

    // (1) The component-composition TEACHING skill survived the tight budget.
    assert!(
        report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "tight 5200 budget must FORCE-INCLUDE component-composition when a library \
         is present; report={report:?}"
    );
    // (2) It is NOT recorded as a budget drop (the exact regression).
    assert!(
        !report
            .dropped
            .iter()
            .any(|s| s.name == "component-composition"),
        "component-composition must not be budget-dropped on the 5200 path; \
         dropped={:?}",
        report.dropped
    );
    // (3) The skill BODY (the ref + descendants rules) is in the system prompt.
    assert!(
        sys.contains("COMPONENT COMPOSITION"),
        "the component-composition skill body must reach the tight-budget prompt"
    );
    assert!(
        sys.contains("descendants"),
        "the prompt must teach overriding instance content via descendants"
    );
    // (4) The AVAILABLE COMPONENTS manifest with concrete ids is also present —
    // both halves (LIST + HOW) reach the model on the tight path.
    assert!(
        sys.contains("AVAILABLE COMPONENTS"),
        "tight-budget prompt must carry the components manifest"
    );
    assert!(
        sys.contains("comp-0") && sys.contains("comp-4"),
        "manifest must list the concrete component ids"
    );
}

/// The force-include is gated on a library being present: with NO components, a
/// tight-budget Basic dashboard prompt must NOT pin (or contain) the
/// component-composition skill — proving the pin is additive and never changes
/// normal no-library generation.
#[test]
fn tight_budget_dashboard_without_library_does_not_pin_component_composition() {
    let basic_req = DesignRequest {
        prompt: "Design a 1280x800 analytics dashboard with metric cards, \
                 a chart panel, and a data table"
            .into(),
        model: Some("minimax-m3".into()),
        ..req()
    };
    let mut dash_plan = plan();
    dash_plan.root_frame.width = 1280.0;
    dash_plan.root_frame.height = 800.0;
    let dash_subtask = Subtask {
        id: "main".into(),
        label: "Main".into(),
        region: Region {
            width: 1280.0,
            height: 600.0,
        },
        id_prefix: "main".into(),
        parent_frame_id: Some("page".into()),
        elements: Some("metric cards, chart, table".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    };

    let (cr, report) = build_subagent_prompt(
        &dash_subtask,
        &dash_plan,
        &basic_req,
        AbortFlag::new(),
        false,
        false,
        &ComponentLibrary::default(),
    );
    assert!(
        !report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "no library ⇒ component-composition must not be force-included; report={report:?}"
    );
    assert!(
        !cr.system_prompt.contains("AVAILABLE COMPONENTS"),
        "no library ⇒ no components manifest"
    );
    assert!(
        !cr.system_prompt.contains("COMPONENT COMPOSITION"),
        "no library ⇒ no component-composition teaching"
    );
}
