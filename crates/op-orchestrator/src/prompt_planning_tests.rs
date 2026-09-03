//! Planning-prompt tests (rich / minimal / compact suffixes) and the
//! sub-agent skill-filtering matrix.

use super::*;

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
        retry_feedback: None,
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

#[test]
fn generation_protocols_require_first_class_interactive_controls() {
    for kind in [
        "text_input",
        "text_area",
        "select",
        "switch",
        "checkbox",
        "slider",
        "radio_group",
        "number_input",
        "progress",
        "tabs",
    ] {
        assert!(
            SCRIPT_FORMAT.contains(kind),
            "script protocol must list native widget `{kind}`"
        );
        assert!(
            NODE_FORMAT.contains(kind),
            "legacy JSONL protocol must list native widget `{kind}`"
        );
    }
    for contract in [
        "options:[{value,label}]",
        "checked",
        "min/max/step/value",
        "fill, stroke, and cornerRadius",
        "fill is the active/accent paint",
        "stroke.fill is the inactive track/border paint",
    ] {
        assert!(
            SCRIPT_FORMAT.contains(contract),
            "script protocol lost interactive contract {contract:?}"
        );
        assert!(
            NODE_FORMAT.contains(contract),
            "legacy JSONL protocol lost interactive contract {contract:?}"
        );
    }
    assert!(SCRIPT_FORMAT.contains("never a frame/rectangle mockup with a role marker"));
    assert!(NODE_FORMAT.contains("Never generate a frame/rectangle mockup with a role marker"));
}

/// The reduced-complexity retry rung keeps script-gen; only its skill set is
/// narrowed.
#[test]
fn subagent_prompt_reduced_complexity_carries_script_format() {
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
        retry_feedback: None,
    };
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), true, false);
    assert!(cr.user_prompt.contains("Hero"));
    assert!(!cr.user_prompt.contains("IDs prefix=\"hero-\""));
    assert!(cr
        .system_prompt
        .contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"));
    assert!(!cr.system_prompt.contains(
        "Respond with THIS section's canonical PenNode objects in the FLAT _parent format"
    ));
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
            retry_feedback: None,
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
            retry_feedback: None,
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

    let required = "Page sections:|Food Categories [category chips]|\"fill_container\"|\"fit_content\"|Generate enough elements|MOBILE STATUS BAR|time, signal, wifi, battery|NO PHONE MOCKUP WRAPPER|MOBILE WIDTH SAFETY|MOBILE SINGLE CONTENT RAIL|MOBILE SCROLLER RAIL|MOBILE SEARCH BAR|MOBILE SECTION CHROME|MOBILE VERTICAL RHYTHM|MOBILE TOP RHYTHM|MOBILE GRID ALIGNMENT|MOBILE CARD OVERLAYS|MOBILE IMAGE PRESENTATION|verify only rendering integrity|Do not judge or replace a displayed image during self-check based on subject relevance|explicit user-requested image edit remains allowed|NO BLANK PLACEHOLDERS|MOBILE NAV SURFACE|MOBILE NAV SHADOW|NO FIXED FOOD TEMPLATE|Do not default to the same search + categories + orange promo + two product cards composition|TYPOGRAPHY HIERARCHY|DENSITY|VISUAL HIERARCHY|SPACING CONSISTENCY|CRAFT POLISH|MEDIA CONSISTENCY|ICON SCALE|SIGNATURE MOMENT|WOW FACTOR|COMPOSITIONAL CONTRAST|PREMIUM DETAIL|NO DECORATION SPAM";
    // Mobile UI guardrails now load via the `mobile-ui` skill (system prompt);
    // section + quality markers stay in the user prompt. Accept either. The
    // `"fill_container"` / `"fit_content"` markers are quote-only (no
    // `width=` / `height=` prefix) so they match both the flat-JSONL root_rule
    // wording and script-gen's `width:"fill_container"` JS-object wording —
    // the root-frame AUTHORING convention itself (`Root frame: id="..."` vs
    // `const sec = I(null, ...)`) is protocol-specific and covered by the
    // dedicated `subagent_prompt_carries_subtask_and_script_format` tests.
    for required in required.split('|') {
        assert!(
            cr.system_prompt.contains(required) || cr.user_prompt.contains(required),
            "missing {required}"
        );
    }
    let combined = format!("{}\n{}", cr.system_prompt, cr.user_prompt);
    for required in [
        "root page may keep 0 horizontal padding",
        "ordinary transparent root-direct content section",
        "padding:[0,24]",
        "inset its header 24px on both sides",
        "24px leading inset",
        "0px trailing edge",
    ] {
        assert!(
            combined.contains(required),
            "missing mobile content-rail rule {required}"
        );
    }
    for stale in [
        "ONE PAGE GUTTER, ON THE ROOT",
        "ALL content elements must sit inside ONE wrapper",
        "Add per-section horizontal padding (wrapper handles it)",
    ] {
        assert!(
            !combined.contains(stale),
            "stale mobile gutter rule must not survive: {stale}"
        );
    }
    assert!(!cr.system_prompt.contains("MOBILE IMAGE QUALITY"));
    assert!(!cr.user_prompt.contains("MOBILE IMAGE QUALITY"));
}

#[test]
fn subagent_prompt_minimal_skills_has_schema_and_script_format() {
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
        retry_feedback: None,
    };
    // minimal_skills=true: the system prompt should contain schema skill
    // content plus the script-gen protocol suffix, but NOT layout/text-rules
    // or the retired jsonl-format skill.
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, true);
    assert!(
        cr.system_prompt
            .contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"),
        "minimal_skills prompt should still append SCRIPT_FORMAT"
    );
    assert!(
        !cr.system_prompt
            .contains("CRITICAL — OUTPUT FORMAT: Emit raw JSONL only"),
        "minimal_skills prompt must not mount jsonl-format"
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
        retry_feedback: None,
    };
    // req() uses model "claude" which is Full tier — no narrowing.
    // Use a basic-tier model to test narrowing.
    let basic_req = DesignRequest {
        prompt: "a page".into(),
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
    let (full_cr, _) = bsp(&st, &plan(), &basic_req, AbortFlag::new(), false, false);
    let (reduced_cr, _) = bsp(&st, &plan(), &basic_req, AbortFlag::new(), true, false);
    assert!(
        reduced_cr.system_prompt.len() <= full_cr.system_prompt.len(),
        "reduced_complexity Basic prompt should be no longer than full-skill prompt"
    );
}

/// `reduced_complexity` only drives tier-gated SKILL narrowing. Holding
/// `script_on` fixed via the core fn isolates the still-true "Full tier skill
/// narrowing is a no-op" invariant.
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
        retry_feedback: None,
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
        false,
        true,
        &ComponentLibrary::default(),
        &[],
    );
    let (reduced_cr, _) = build_subagent_prompt_core(
        &st,
        &plan(),
        &req(),
        AbortFlag::new(),
        true,
        false,
        false,
        true,
        &ComponentLibrary::default(),
        &[],
    );
    assert_eq!(
        full_cr.system_prompt, reduced_cr.system_prompt,
        "reduced_complexity's skill narrowing should be a no-op on Full tier"
    );
}

/// Public prompt construction keeps script-gen even when `reduced_complexity`
/// is enabled; on Full tier the skill set is also unchanged, so the prompt is
/// byte-for-byte equal to the full attempt.
#[test]
fn subagent_prompt_reduced_complexity_keeps_script_gen_even_on_full_tier() {
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
        retry_feedback: None,
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
        "schema skill should still describe PenNode"
    );
    assert!(
        reduced_cr
            .system_prompt
            .contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"),
        "reduced rung must stay on script-gen"
    );
    assert!(
        !reduced_cr.system_prompt.contains(
            "Respond with THIS section's canonical PenNode objects in the FLAT _parent format"
        ),
        "reduced rung must not append NODE_FORMAT"
    );
    assert_eq!(full_cr.system_prompt, reduced_cr.system_prompt);
}

/// Regression guard for retiring the flat JSONL generation protocol: Basic
/// reduced-complexity retries still narrow the skill set, but the subagent
/// prompt must not mount either JSONL output-format skill.
#[test]
fn subagent_prompt_basic_tier_reduced_retry_drops_jsonl_format_skills() {
    // Historical JSONL-only markers. The files are no longer mounted in the
    // generation corpus, and this reduced retry should not reintroduce their
    // wording through any compact-skill path.
    const VERBOSE_ONLY: &str = "imageSearchQuery MUST be UNIQUE";
    const SIMPLIFIED_ONLY: &str = "rectangle (width,height,cornerRadius,fill)";

    let basic_req = DesignRequest {
        prompt: "a page".into(),
        model: Some("claude-haiku".into()), // Basic tier
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
    let (basic_cr, basic_report) = bsp(
        &subtask(),
        &plan(),
        &basic_req,
        AbortFlag::new(),
        true,
        false,
    );
    let included: Vec<&str> = basic_report
        .included
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();

    assert!(
        !included.contains(&"jsonl-format-simplified"),
        "Basic reduced retry must not mount jsonl-format-simplified"
    );
    assert!(
        !included.contains(&"jsonl-format"),
        "Basic reduced retry must not mount jsonl-format"
    );
    assert!(
        !basic_cr.system_prompt.contains(SIMPLIFIED_ONLY),
        "Basic reduced retry must not carry simplified JSONL wording"
    );
    assert!(
        !basic_cr.system_prompt.contains(VERBOSE_ONLY),
        "Basic reduced retry must not carry verbose JSONL wording"
    );
    assert!(
        basic_cr
            .system_prompt
            .contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"),
        "Basic reduced retry must still append SCRIPT_FORMAT"
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
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
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
        retry_feedback: None,
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

/// gemini-3.6-flash wrote `justify.content:` three times in one slide script
/// and QuickJS threw the whole board away at the first `.`. The runner now
/// repairs that shape, but the cheaper fix is the model not writing it — so
/// the property-naming rule must actually reach the sub-agent prompt.
#[test]
fn subagent_prompt_teaches_camelcase_property_names() {
    let st = Subtask {
        id: "cover".into(),
        label: "Cover".into(),
        region: Region {
            width: 1920.0,
            height: 1080.0,
        },
        id_prefix: "cover".into(),
        parent_frame_id: Some("root".into()),
        elements: None,
        screen: Some("Cover".into()),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    };
    let (cr, _) = bsp(&st, &plan(), &req(), AbortFlag::new(), false, false);
    for rule in ["PROPERTY NAMES are camelCase", "justify.content"] {
        assert!(
            cr.system_prompt.contains(rule),
            "sub-agent prompt lost the property-naming rule {rule:?}"
        );
    }
}
